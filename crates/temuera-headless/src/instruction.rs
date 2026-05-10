#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Instruction {
    Empty,
    Comment,
    Label(String),
    Print {
        text: String,
        newline: bool,
    },
    PrintForm {
        text: String,
        newline: bool,
    },
    PrintExpr {
        expr: String,
        newline: bool,
    },
    Bar(String),
    DrawLine,
    Assign {
        target: String,
        expr: String,
    },
    Inc(String),
    Dec(String),
    SIf(String),
    If(String),
    ElseIf(String),
    Else,
    EndIf,
    For {
        variable: String,
        start: String,
        end: String,
    },
    Next,
    Goto(String),
    Call(String),
    Input {
        string: bool,
    },
    Unsupported(String),
}

pub fn parse_line(line: &str) -> Instruction {
    let line = strip_inline_comment(line.trim_start().trim_start_matches('\u{feff}')).trim();
    if line.is_empty() {
        return Instruction::Empty;
    }
    if line.starts_with(';') {
        return Instruction::Comment;
    }
    if let Some(label) = line.strip_prefix('$') {
        return Instruction::Label(symbol_name(label));
    }
    if line.starts_with('@') {
        return Instruction::Unsupported(line.to_owned());
    }

    let (keyword, rest) = split_keyword(line);
    match keyword.to_ascii_uppercase().as_str() {
        "PRINTL" => Instruction::Print {
            text: rest.to_owned(),
            newline: true,
        },
        "PRINT" => Instruction::Print {
            text: rest.to_owned(),
            newline: false,
        },
        "PRINTFORML" => Instruction::PrintForm {
            text: rest.to_owned(),
            newline: true,
        },
        "PRINTFORM" => Instruction::PrintForm {
            text: rest.to_owned(),
            newline: false,
        },
        "PRINTSL" => Instruction::PrintExpr {
            expr: rest.to_owned(),
            newline: true,
        },
        "PRINTS" => Instruction::PrintExpr {
            expr: rest.to_owned(),
            newline: false,
        },
        "BAR" => Instruction::Bar(rest.to_owned()),
        "DRAWLINE" => Instruction::DrawLine,
        "SIF" => Instruction::SIf(rest.to_owned()),
        "IF" => Instruction::If(rest.to_owned()),
        "ELSEIF" => Instruction::ElseIf(rest.to_owned()),
        "ELSE" => Instruction::Else,
        "ENDIF" => Instruction::EndIf,
        "FOR" => parse_for(rest).unwrap_or_else(|| Instruction::Unsupported(line.to_owned())),
        "NEXT" => Instruction::Next,
        "GOTO" => Instruction::Goto(symbol_name(rest)),
        "CALL" => Instruction::Call(call_name(rest)),
        "INPUT" | "TINPUT" | "ONEINPUT" => Instruction::Input { string: false },
        "INPUTS" | "TINPUTS" | "ONEINPUTS" => Instruction::Input { string: true },
        _ => parse_mutation(line)
            .or_else(|| parse_assignment(line))
            .unwrap_or_else(|| Instruction::Unsupported(line.to_owned())),
    }
}

fn parse_mutation(line: &str) -> Option<Instruction> {
    let trimmed = line.trim();
    if let Some(target) = trimmed.strip_suffix("++") {
        return Some(Instruction::Inc(target.trim().to_owned()));
    }
    if let Some(target) = trimmed.strip_suffix("--") {
        return Some(Instruction::Dec(target.trim().to_owned()));
    }
    None
}

fn parse_for(rest: &str) -> Option<Instruction> {
    let mut parts = rest.split(',').map(str::trim);
    let variable = parts.next()?.to_owned();
    let start = parts.next()?.to_owned();
    let end = parts.next()?.to_owned();
    if variable.is_empty() || start.is_empty() || end.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(Instruction::For {
        variable,
        start,
        end,
    })
}

fn parse_assignment(line: &str) -> Option<Instruction> {
    let operator = find_assignment_operator(line)?;
    let target = line[..operator].trim();
    let expr = line[operator + 1..].trim();
    if target.is_empty() || expr.is_empty() {
        return None;
    }
    Some(Instruction::Assign {
        target: target.to_owned(),
        expr: expr.to_owned(),
    })
}

fn find_assignment_operator(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index] != b'=' {
            continue;
        }
        let prev = index.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
        let next = bytes.get(index + 1).copied();
        if matches!(prev, Some(b'=' | b'!' | b'<' | b'>')) || next == Some(b'=') {
            continue;
        }
        return Some(index);
    }
    None
}

fn split_keyword(line: &str) -> (&str, &str) {
    let end = line
        .find(|ch: char| ch.is_whitespace())
        .unwrap_or(line.len());
    let keyword = &line[..end];
    let rest = line[end..].trim_start();
    (keyword, rest)
}

fn symbol_name(text: &str) -> String {
    text.trim()
        .split(|ch: char| ch.is_whitespace() || ch == '(' || ch == ';')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase()
}

fn call_name(text: &str) -> String {
    symbol_name(text)
}

fn strip_inline_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (index, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ';' && !in_quotes {
            return &line[..index];
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_instructions() {
        assert_eq!(
            parse_line("PRINTL hello"),
            Instruction::Print {
                text: "hello".to_owned(),
                newline: true,
            }
        );
        assert_eq!(parse_line("$LOOP"), Instruction::Label("LOOP".to_owned()));
        assert_eq!(
            parse_line("FLAG:真实模式 = 1"),
            Instruction::Assign {
                target: "FLAG:真实模式".to_owned(),
                expr: "1".to_owned(),
            }
        );
        assert_eq!(
            parse_line("GOTO LOOP"),
            Instruction::Goto("LOOP".to_owned())
        );
        assert_eq!(
            parse_line("SIF !FLAG:真实模式"),
            Instruction::SIf("!FLAG:真实模式".to_owned())
        );
    }

    #[test]
    fn does_not_treat_comparison_as_assignment() {
        assert!(matches!(
            parse_line("RESULT == 0"),
            Instruction::Unsupported(_)
        ));
    }
}
