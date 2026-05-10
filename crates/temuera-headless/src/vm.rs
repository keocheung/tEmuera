use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;

use crate::csv::CsvCatalog;
use crate::expr;
use crate::instruction::{Instruction, parse_line};
use crate::runtime::{Value, VarAddress, VarResolver, VarStore};
use crate::script::{LocatedScriptLine, ScriptCatalog};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StopReason {
    EndOfFunction,
    NeedInput,
    Unsupported {
        file: PathBuf,
        line_no: usize,
        text: String,
    },
    FunctionNotFound(String),
    StepLimit,
    Error(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VmReport {
    pub stop_reason: StopReason,
    pub steps: usize,
    pub output: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Vm<'a> {
    scripts: &'a ScriptCatalog,
    resolver: VarResolver<'a>,
    variables: VarStore,
    inputs: VecDeque<Value>,
    output: Vec<String>,
}

impl<'a> Vm<'a> {
    pub fn new(scripts: &'a ScriptCatalog, csv: &'a CsvCatalog) -> Self {
        Self {
            scripts,
            resolver: VarResolver::new(csv),
            variables: VarStore::default(),
            inputs: VecDeque::new(),
            output: Vec::new(),
        }
    }

    pub fn push_input(&mut self, input: impl Into<String>) {
        let input = input.into();
        let value = input
            .parse::<i64>()
            .map(Value::Int)
            .unwrap_or(Value::Str(input));
        self.inputs.push_back(value);
    }

    pub fn run_function(&mut self, name: &str, step_limit: usize) -> VmReport {
        self.run_function_inner(name, step_limit, 0)
    }

    fn run_function_inner(&mut self, name: &str, step_limit: usize, depth: usize) -> VmReport {
        if depth > 32 {
            return self.report(StopReason::Error("call stack limit exceeded".to_owned()), 0);
        }
        let Some(lines) = self.scripts.function_lines(name) else {
            return self.report(StopReason::FunctionNotFound(name.to_owned()), 0);
        };
        self.run_lines(lines, step_limit, depth)
    }

    fn run_lines(
        &mut self,
        lines: Vec<LocatedScriptLine<'_>>,
        step_limit: usize,
        depth: usize,
    ) -> VmReport {
        let mut pc = 0usize;
        let mut steps = 0usize;
        let labels = build_label_map(&lines);
        let mut if_stack = Vec::<IfFrame>::new();
        let mut for_stack = Vec::<ForFrame>::new();
        let mut skip_next = false;

        while pc < lines.len() {
            if steps >= step_limit {
                return self.report(StopReason::StepLimit, steps);
            }
            steps += 1;

            let located = lines[pc];
            let instruction = parse_line(&located.line.text);
            let active = if_stack.iter().all(|frame| frame.active);

            if skip_next {
                skip_next = false;
                if !matches!(
                    instruction,
                    Instruction::Empty
                        | Instruction::Comment
                        | Instruction::Label(_)
                        | Instruction::If(_)
                        | Instruction::ElseIf(_)
                        | Instruction::Else
                        | Instruction::EndIf
                ) {
                    pc += 1;
                    continue;
                }
            }

            match instruction {
                Instruction::Empty | Instruction::Comment | Instruction::Label(_) => {}
                Instruction::If(condition) => {
                    let parent_active = active;
                    let condition_active =
                        parent_active && self.eval_bool(&condition).unwrap_or(false);
                    if_stack.push(IfFrame {
                        parent_active,
                        active: condition_active,
                        branch_taken: condition_active,
                    });
                }
                Instruction::ElseIf(condition) => {
                    let Some(frame) = if_stack.last_mut() else {
                        return self.report(error_at("ELSEIF without IF", located), steps);
                    };
                    if !frame.parent_active || frame.branch_taken {
                        frame.active = false;
                    } else {
                        frame.active = self.eval_bool(&condition).unwrap_or(false);
                        frame.branch_taken = frame.active;
                    }
                }
                Instruction::Else => {
                    let Some(frame) = if_stack.last_mut() else {
                        return self.report(error_at("ELSE without IF", located), steps);
                    };
                    frame.active = frame.parent_active && !frame.branch_taken;
                    frame.branch_taken = true;
                }
                Instruction::EndIf => {
                    if if_stack.pop().is_none() {
                        return self.report(error_at("ENDIF without IF", located), steps);
                    }
                }
                Instruction::SIf(condition) => {
                    if active && !self.eval_bool(&condition).unwrap_or(false) {
                        skip_next = true;
                    }
                }
                _ if !active => {}
                Instruction::Print { text, newline } => {
                    self.push_output(text, newline);
                }
                Instruction::PrintForm { text, newline } => {
                    let text = self.render_form(&text);
                    self.push_output(text, newline);
                }
                Instruction::PrintExpr { expr, newline } => {
                    let value = match self.eval_value(&expr) {
                        Ok(value) => value,
                        Err(err) => {
                            return self.report(error_at(&format!("{err:?}"), located), steps);
                        }
                    };
                    self.push_output(value_to_string(&value), newline);
                }
                Instruction::Bar(_) => {
                    self.push_output("[----------------]".to_owned(), false);
                }
                Instruction::DrawLine => {
                    self.push_output("-".repeat(80), true);
                }
                Instruction::Assign { target, expr } => {
                    let Some(address) = self.resolve_address(&target) else {
                        return self.report(error_at("invalid assignment target", located), steps);
                    };
                    let value = match self.eval_value(&expr) {
                        Ok(value) => value,
                        Err(err) => {
                            return self.report(error_at(&format!("{err:?}"), located), steps);
                        }
                    };
                    self.variables.set(address, value);
                }
                Instruction::Inc(target) => {
                    let Some(address) = self.resolve_address(&target) else {
                        return self.report(error_at("invalid mutation target", located), steps);
                    };
                    let value = self.variables.get(&address).as_int_lossy() + 1;
                    self.variables.set(address, Value::Int(value));
                }
                Instruction::Dec(target) => {
                    let Some(address) = self.resolve_address(&target) else {
                        return self.report(error_at("invalid mutation target", located), steps);
                    };
                    let value = self.variables.get(&address).as_int_lossy() - 1;
                    self.variables.set(address, Value::Int(value));
                }
                Instruction::For {
                    variable,
                    start,
                    end,
                } => {
                    let start = match self.eval_int(&start) {
                        Ok(value) => value,
                        Err(err) => {
                            return self.report(error_at(&format!("{err:?}"), located), steps);
                        }
                    };
                    let end = match self.eval_int(&end) {
                        Ok(value) => value,
                        Err(err) => {
                            return self.report(error_at(&format!("{err:?}"), located), steps);
                        }
                    };
                    let Some(address) = self.resolve_address(&variable) else {
                        return self.report(error_at("invalid FOR variable", located), steps);
                    };
                    self.variables.set(address, Value::Int(start));
                    for_stack.push(ForFrame {
                        variable,
                        end,
                        body_pc: pc + 1,
                    });
                }
                Instruction::Next => {
                    let Some(frame) = for_stack.last().cloned() else {
                        return self.report(error_at("NEXT without FOR", located), steps);
                    };
                    let Some(address) = self.resolve_address(&frame.variable) else {
                        return self.report(error_at("invalid FOR variable", located), steps);
                    };
                    let next = self.variables.get(&address).as_int_lossy() + 1;
                    if next < frame.end {
                        self.variables.set(address, Value::Int(next));
                        pc = frame.body_pc;
                        continue;
                    }
                    for_stack.pop();
                }
                Instruction::Goto(label) => {
                    let Some(next_pc) = labels.get(&label).copied() else {
                        return self.report(
                            error_at(&format!("label not found: {label}"), located),
                            steps,
                        );
                    };
                    pc = next_pc;
                    continue;
                }
                Instruction::Call(name) => {
                    let remaining = step_limit.saturating_sub(steps);
                    let report = self.run_function_inner(&name, remaining, depth + 1);
                    steps += report.steps;
                    if report.stop_reason != StopReason::EndOfFunction {
                        return self.report(report.stop_reason, steps);
                    }
                }
                Instruction::Input { string } => {
                    if let Some(value) = self.inputs.pop_front() {
                        let name = if string { "RESULTS" } else { "RESULT" };
                        self.variables.set(
                            VarAddress {
                                name: name.to_owned(),
                                indexes: Vec::new(),
                            },
                            value,
                        );
                    } else {
                        return self.report(StopReason::NeedInput, steps);
                    }
                }
                Instruction::Unsupported(text) => {
                    return self.report(
                        StopReason::Unsupported {
                            file: located.file.to_path_buf(),
                            line_no: located.line.line_no,
                            text,
                        },
                        steps,
                    );
                }
            }

            pc += 1;
        }

        self.report(StopReason::EndOfFunction, steps)
    }

    fn eval_bool(&self, expression: &str) -> Result<bool, expr::ExprError> {
        let expr = expr::parse(expression)?;
        let value = expr::eval(&expr, &self.resolver, &self.variables)?;
        Ok(match value {
            Value::Int(value) => value != 0,
            Value::Str(value) => !value.is_empty(),
        })
    }

    fn eval_int(&self, expression: &str) -> Result<i64, expr::ExprError> {
        let value = self.eval_value(expression)?;
        Ok(value.as_int_lossy())
    }

    fn eval_value(&self, expression: &str) -> Result<Value, expr::ExprError> {
        if let Some(address) = expression
            .trim()
            .strip_prefix('%')
            .and_then(|value| value.strip_suffix('%'))
            .and_then(|value| self.resolve_address(value.trim()))
        {
            return Ok(self.variables.get(&address));
        }
        let expr = expr::parse(expression)?;
        expr::eval(&expr, &self.resolver, &self.variables)
    }

    fn resolve_address(&self, text: &str) -> Option<crate::runtime::VarAddress> {
        if let Some(address) = self.resolver.parse_address(text) {
            return Some(address);
        }

        let mut parts = text.split(':');
        let name = parts.next()?.trim().to_ascii_uppercase();
        if name.is_empty() {
            return None;
        }

        let mut indexes = Vec::new();
        for part in parts.map(str::trim) {
            if part.is_empty() {
                return None;
            }
            if let Some(index) = self
                .scripts_csv_name(&name, part)
                .or_else(|| self.eval_int(part).ok())
            {
                indexes.push(index);
            } else {
                return None;
            }
        }
        Some(crate::runtime::VarAddress { name, indexes })
    }

    fn scripts_csv_name(&self, variable_name: &str, part: &str) -> Option<i64> {
        self.resolver.csv().resolve_name(variable_name, part)
    }

    fn push_output(&mut self, text: String, newline: bool) {
        if newline || self.output.is_empty() {
            self.output.push(text);
        } else if let Some(last) = self.output.last_mut() {
            last.push_str(&text);
        }
    }

    fn render_form(&self, text: &str) -> String {
        let text = self.replace_delimited(text, '%', '%');
        self.replace_delimited(&text, '{', '}')
    }

    fn replace_delimited(&self, text: &str, open: char, close: char) -> String {
        let mut output = String::new();
        let mut rest = text;
        while let Some(start) = rest.find(open) {
            output.push_str(&rest[..start]);
            let after_open = &rest[start + open.len_utf8()..];
            let Some(end) = after_open.find(close) else {
                output.push(open);
                output.push_str(after_open);
                return output;
            };
            let expression = &after_open[..end];
            output.push_str(&self.eval_form_piece(expression));
            rest = &after_open[end + close.len_utf8()..];
        }
        output.push_str(rest);
        output
    }

    fn eval_form_piece(&self, expression: &str) -> String {
        let expression = expression.trim();
        let expression = expression.split(',').next().unwrap_or(expression).trim();
        if let Some(value) = self.eval_name_lookup(expression) {
            return value;
        }
        self.eval_value(expression)
            .map(|value| value_to_string(&value))
            .unwrap_or_else(|_| String::new())
    }

    fn eval_name_lookup(&self, expression: &str) -> Option<String> {
        let (table_name, index_expr) = expression.split_once(':')?;
        let table_name = table_name.trim();
        let table = table_name
            .to_ascii_uppercase()
            .strip_suffix("NAME")?
            .to_owned();
        let index = self.eval_int(index_expr.trim()).ok()?;
        self.resolver
            .csv()
            .name_for_id(&table, index)
            .map(str::to_owned)
    }

    fn report(&self, stop_reason: StopReason, steps: usize) -> VmReport {
        VmReport {
            stop_reason,
            steps,
            output: self.output.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct IfFrame {
    parent_active: bool,
    active: bool,
    branch_taken: bool,
}

#[derive(Debug, Clone)]
struct ForFrame {
    variable: String,
    end: i64,
    body_pc: usize,
}

trait VmValueExt {
    fn as_int_lossy(&self) -> i64;
}

impl VmValueExt for Value {
    fn as_int_lossy(&self) -> i64 {
        match self {
            Value::Int(value) => *value,
            Value::Str(value) => value.parse().unwrap_or(0),
        }
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Str(value) => value.clone(),
    }
}

fn build_label_map(lines: &[LocatedScriptLine<'_>]) -> HashMap<String, usize> {
    let mut labels = HashMap::new();
    for (index, located) in lines.iter().enumerate() {
        if let Instruction::Label(name) = parse_line(&located.line.text) {
            labels.entry(name).or_insert(index + 1);
        }
    }
    labels
}

fn error_at(message: &str, located: LocatedScriptLine<'_>) -> StopReason {
    StopReason::Error(format!(
        "{} at {}:{}",
        message,
        located.file.display(),
        located.line.line_no
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::csv::CsvCatalog;
    use crate::script::{
        FunctionDef, ScriptCatalog, ScriptFile, ScriptFileKind, ScriptIndex, ScriptLine,
        ScriptLineKind, ScriptLocation,
    };

    use super::*;

    fn catalog(lines: &[&str]) -> ScriptCatalog {
        let script_lines = std::iter::once("@TEST")
            .chain(lines.iter().copied())
            .enumerate()
            .map(|(index, text)| ScriptLine {
                line_no: index + 1,
                text: text.to_owned(),
                kind: if text.starts_with('@') {
                    ScriptLineKind::Function
                } else if text.starts_with('$') {
                    ScriptLineKind::Label
                } else {
                    ScriptLineKind::Instruction
                },
            })
            .collect();
        ScriptCatalog {
            files: vec![ScriptFile {
                path: PathBuf::from("TEST.ERB"),
                relative_path: PathBuf::from("ERB/TEST.ERB"),
                kind: ScriptFileKind::Erb,
                lines: script_lines,
            }],
            index: ScriptIndex {
                functions: vec![FunctionDef {
                    name: "TEST".to_owned(),
                    location: ScriptLocation {
                        file: PathBuf::from("ERB/TEST.ERB"),
                        line_no: 1,
                    },
                }],
                labels: HashMap::new(),
            },
            enabled_lines: lines.len(),
        }
    }

    #[test]
    fn executes_print_assignment_and_if() {
        let scripts = catalog(&[
            "FLAG:1 = 1",
            "IF FLAG:1 == 1",
            "PRINTL yes",
            "ELSE",
            "PRINTL no",
            "ENDIF",
        ]);
        let csv = CsvCatalog::default();
        let mut vm = Vm::new(&scripts, &csv);
        let report = vm.run_function("TEST", 100);
        assert_eq!(report.stop_reason, StopReason::EndOfFunction);
        assert_eq!(report.output, vec!["yes"]);
    }

    #[test]
    fn stops_at_input() {
        let scripts = catalog(&["PRINTL before", "INPUT", "PRINTL after"]);
        let csv = CsvCatalog::default();
        let mut vm = Vm::new(&scripts, &csv);
        let report = vm.run_function("TEST", 100);
        assert_eq!(report.stop_reason, StopReason::NeedInput);
        assert_eq!(report.output, vec!["before"]);
    }

    #[test]
    fn consumes_queued_input_as_result() {
        let scripts = catalog(&[
            "INPUT",
            "IF RESULT == 0",
            "PRINTL zero",
            "ELSE",
            "PRINTL other",
            "ENDIF",
        ]);
        let csv = CsvCatalog::default();
        let mut vm = Vm::new(&scripts, &csv);
        vm.push_input("0");
        let report = vm.run_function("TEST", 100);
        assert_eq!(report.stop_reason, StopReason::EndOfFunction);
        assert_eq!(report.output, vec!["zero"]);
    }

    #[test]
    fn supports_single_line_if() {
        let scripts = catalog(&["FLAG:1 = 0", "SIF FLAG:1", "PRINTL hidden", "PRINTL shown"]);
        let csv = CsvCatalog::default();
        let mut vm = Vm::new(&scripts, &csv);
        let report = vm.run_function("TEST", 100);
        assert_eq!(report.stop_reason, StopReason::EndOfFunction);
        assert_eq!(report.output, vec!["shown"]);
    }
}
