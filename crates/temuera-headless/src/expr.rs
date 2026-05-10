use crate::runtime::{Value, VarResolver, VarStore};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExprError {
    UnexpectedEnd,
    UnexpectedToken(String),
    InvalidNumber(String),
    InvalidVariable(String),
    DivisionByZero,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expr {
    Value(Value),
    Variable(String),
    Unary {
        op: UnaryOp,
        rhs: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    Ne,
    Ge,
    Gt,
    Le,
    Lt,
    BitOr,
    BitAnd,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

pub fn parse(text: &str) -> Result<Expr, ExprError> {
    let tokens = tokenize(text)?;
    let mut parser = Parser {
        tokens: &tokens,
        position: 0,
    };
    let expr = parser.parse_expr(0)?;
    if parser.peek().is_some() {
        return Err(ExprError::UnexpectedToken(parser.peek_text().to_owned()));
    }
    Ok(expr)
}

pub fn eval(
    expr: &Expr,
    resolver: &VarResolver<'_>,
    variables: &VarStore,
) -> Result<Value, ExprError> {
    match expr {
        Expr::Value(value) => Ok(value.clone()),
        Expr::Variable(address) => {
            let address = resolver
                .parse_address(address)
                .ok_or_else(|| ExprError::InvalidVariable(address.clone()))?;
            Ok(variables.get(&address))
        }
        Expr::Unary { op, rhs } => {
            let rhs = eval(rhs, resolver, variables)?;
            match op {
                UnaryOp::Neg => Ok(Value::Int(-rhs.as_int())),
                UnaryOp::Not => Ok(Value::Int((!rhs.as_bool()) as i64)),
            }
        }
        Expr::Binary { lhs, op, rhs } => {
            let lhs = eval(lhs, resolver, variables)?;
            if *op == BinaryOp::And && !lhs.as_bool() {
                return Ok(Value::Int(0));
            }
            if *op == BinaryOp::Or && lhs.as_bool() {
                return Ok(Value::Int(1));
            }
            let rhs = eval(rhs, resolver, variables)?;
            eval_binary(lhs, *op, rhs)
        }
        Expr::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            if eval(condition, resolver, variables)?.as_bool() {
                eval(then_expr, resolver, variables)
            } else {
                eval(else_expr, resolver, variables)
            }
        }
        Expr::Call { name, args } => eval_call(name, args, resolver, variables),
    }
}

fn eval_call(
    name: &str,
    args: &[Expr],
    resolver: &VarResolver<'_>,
    variables: &VarStore,
) -> Result<Value, ExprError> {
    match name.to_ascii_uppercase().as_str() {
        "GETBIT" => {
            if args.len() != 2 {
                return Err(ExprError::UnexpectedToken(name.to_owned()));
            }
            let value = eval(&args[0], resolver, variables)?.as_int();
            let bit = eval(&args[1], resolver, variables)?.as_int();
            Ok(Value::Int((value >> bit) & 1))
        }
        "STRLENS" => {
            if args.len() != 1 {
                return Err(ExprError::UnexpectedToken(name.to_owned()));
            }
            let value = eval(&args[0], resolver, variables)?;
            let len = match value {
                Value::Int(value) => value.to_string().chars().count(),
                Value::Str(value) => value.chars().count(),
            };
            Ok(Value::Int(len as i64))
        }
        _ => Err(ExprError::InvalidVariable(name.to_owned())),
    }
}

fn eval_binary(lhs: Value, op: BinaryOp, rhs: Value) -> Result<Value, ExprError> {
    match op {
        BinaryOp::Or => Ok(Value::Int((lhs.as_bool() || rhs.as_bool()) as i64)),
        BinaryOp::And => Ok(Value::Int((lhs.as_bool() && rhs.as_bool()) as i64)),
        BinaryOp::Eq => Ok(Value::Int((lhs == rhs) as i64)),
        BinaryOp::Ne => Ok(Value::Int((lhs != rhs) as i64)),
        BinaryOp::Ge => Ok(Value::Int((lhs.as_int() >= rhs.as_int()) as i64)),
        BinaryOp::Gt => Ok(Value::Int((lhs.as_int() > rhs.as_int()) as i64)),
        BinaryOp::Le => Ok(Value::Int((lhs.as_int() <= rhs.as_int()) as i64)),
        BinaryOp::Lt => Ok(Value::Int((lhs.as_int() < rhs.as_int()) as i64)),
        BinaryOp::BitOr => Ok(Value::Int(lhs.as_int() | rhs.as_int())),
        BinaryOp::BitAnd => Ok(Value::Int(lhs.as_int() & rhs.as_int())),
        BinaryOp::Add => match (lhs, rhs) {
            (Value::Str(lhs), Value::Str(rhs)) => Ok(Value::Str(lhs + &rhs)),
            (Value::Str(lhs), rhs) => Ok(Value::Str(lhs + &rhs.to_display_string())),
            (lhs, Value::Str(rhs)) => Ok(Value::Str(lhs.to_display_string() + &rhs)),
            (lhs, rhs) => Ok(Value::Int(lhs.as_int() + rhs.as_int())),
        },
        BinaryOp::Sub => Ok(Value::Int(lhs.as_int() - rhs.as_int())),
        BinaryOp::Mul => Ok(Value::Int(lhs.as_int() * rhs.as_int())),
        BinaryOp::Div => {
            let rhs = rhs.as_int();
            if rhs == 0 {
                Err(ExprError::DivisionByZero)
            } else {
                Ok(Value::Int(lhs.as_int() / rhs))
            }
        }
        BinaryOp::Rem => {
            let rhs = rhs.as_int();
            if rhs == 0 {
                Err(ExprError::DivisionByZero)
            } else {
                Ok(Value::Int(lhs.as_int() % rhs))
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Token {
    Int(i64),
    Str(String),
    Symbol(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
    Question,
    Hash,
}

fn tokenize(text: &str) -> Result<Vec<Token>, ExprError> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch.is_ascii_digit() {
            let mut text = String::new();
            while chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                text.push(chars.next().unwrap());
            }
            let value = text
                .parse::<i64>()
                .map_err(|_| ExprError::InvalidNumber(text.clone()))?;
            tokens.push(Token::Int(value));
            continue;
        }
        if ch == '"' {
            chars.next();
            let mut value = String::new();
            while let Some(ch) = chars.next() {
                if ch == '"' {
                    if chars.peek() == Some(&'"') {
                        value.push('"');
                        chars.next();
                    } else {
                        break;
                    }
                } else {
                    value.push(ch);
                }
            }
            tokens.push(Token::Str(value));
            continue;
        }

        let two = {
            let mut clone = chars.clone();
            let first = clone.next();
            let second = clone.next();
            first
                .zip(second)
                .map(|(a, b)| [a, b].iter().collect::<String>())
        };
        if let Some(op) = two.as_deref().and_then(two_char_op) {
            chars.next();
            chars.next();
            tokens.push(Token::Op(op));
            continue;
        }

        match ch {
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '?' => {
                chars.next();
                tokens.push(Token::Question);
            }
            '#' => {
                chars.next();
                tokens.push(Token::Hash);
            }
            '+' | '-' | '*' | '/' | '%' | '!' | '>' | '<' | '&' | '|' => {
                chars.next();
                tokens.push(Token::Op(match ch {
                    '+' => "+",
                    '-' => "-",
                    '*' => "*",
                    '/' => "/",
                    '%' => "%",
                    '!' => "!",
                    '>' => ">",
                    '<' => "<",
                    '&' => "&",
                    '|' => "|",
                    _ => unreachable!(),
                }));
            }
            _ => {
                let mut symbol = String::new();
                while let Some(ch) = chars.peek().copied() {
                    if ch.is_whitespace()
                        || matches!(
                            ch,
                            '(' | ')'
                                | ','
                                | '?'
                                | '#'
                                | '+'
                                | '-'
                                | '*'
                                | '/'
                                | '%'
                                | '!'
                                | '>'
                                | '<'
                                | '='
                                | '&'
                                | '|'
                        )
                    {
                        break;
                    }
                    symbol.push(ch);
                    chars.next();
                }
                if symbol.is_empty() {
                    return Err(ExprError::UnexpectedToken(ch.to_string()));
                }
                tokens.push(Token::Symbol(symbol));
            }
        }
    }
    Ok(tokens)
}

fn two_char_op(text: &str) -> Option<&'static str> {
    match text {
        "||" => Some("||"),
        "&&" => Some("&&"),
        "==" => Some("=="),
        "!=" => Some("!="),
        ">=" => Some(">="),
        "<=" => Some("<="),
        _ => None,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl Parser<'_> {
    fn parse_expr(&mut self, min_binding_power: u8) -> Result<Expr, ExprError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if self.consume_question() {
                let then_expr = self.parse_expr(0)?;
                if !self.consume_hash() {
                    return Err(ExprError::UnexpectedToken(self.peek_text().to_owned()));
                }
                let else_expr = self.parse_expr(0)?;
                lhs = Expr::Conditional {
                    condition: Box::new(lhs),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                };
                continue;
            }

            let Some((op, left_bp, right_bp)) = self.peek_binary_op() else {
                break;
            };
            if left_bp < min_binding_power {
                break;
            }
            self.position += 1;
            let rhs = self.parse_expr(right_bp)?;
            lhs = Expr::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ExprError> {
        match self.next() {
            Some(Token::Int(value)) => Ok(Expr::Value(Value::Int(*value))),
            Some(Token::Str(value)) => Ok(Expr::Value(Value::Str(value.clone()))),
            Some(Token::Symbol(value)) => {
                let value = value.clone();
                if self.consume_lparen() {
                    let mut args = Vec::new();
                    if !self.consume_rparen() {
                        loop {
                            args.push(self.parse_expr(0)?);
                            if self.consume_rparen() {
                                break;
                            }
                            if !self.consume_comma() {
                                return Err(ExprError::UnexpectedToken(
                                    self.peek_text().to_owned(),
                                ));
                            }
                        }
                    }
                    Ok(Expr::Call { name: value, args })
                } else {
                    Ok(Expr::Variable(value))
                }
            }
            Some(Token::Op("-")) => Ok(Expr::Unary {
                op: UnaryOp::Neg,
                rhs: Box::new(self.parse_expr(13)?),
            }),
            Some(Token::Op("!")) => Ok(Expr::Unary {
                op: UnaryOp::Not,
                rhs: Box::new(self.parse_expr(13)?),
            }),
            Some(Token::LParen) => {
                let expr = self.parse_expr(0)?;
                match self.next() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err(ExprError::UnexpectedToken(self.peek_text().to_owned())),
                }
            }
            Some(token) => Err(ExprError::UnexpectedToken(format!("{token:?}"))),
            None => Err(ExprError::UnexpectedEnd),
        }
    }

    fn peek_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.peek()? {
            Token::Op("||") => Some((BinaryOp::Or, 1, 2)),
            Token::Op("&&") => Some((BinaryOp::And, 3, 4)),
            Token::Op("==") => Some((BinaryOp::Eq, 5, 6)),
            Token::Op("!=") => Some((BinaryOp::Ne, 5, 6)),
            Token::Op(">=") => Some((BinaryOp::Ge, 7, 8)),
            Token::Op(">") => Some((BinaryOp::Gt, 7, 8)),
            Token::Op("<=") => Some((BinaryOp::Le, 7, 8)),
            Token::Op("<") => Some((BinaryOp::Lt, 7, 8)),
            Token::Op("|") => Some((BinaryOp::BitOr, 8, 9)),
            Token::Op("&") => Some((BinaryOp::BitAnd, 8, 9)),
            Token::Op("+") => Some((BinaryOp::Add, 9, 10)),
            Token::Op("-") => Some((BinaryOp::Sub, 9, 10)),
            Token::Op("*") => Some((BinaryOp::Mul, 11, 12)),
            Token::Op("/") => Some((BinaryOp::Div, 11, 12)),
            Token::Op("%") => Some((BinaryOp::Rem, 11, 12)),
            _ => None,
        }
    }

    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position)?;
        self.position += 1;
        Some(token)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek_text(&self) -> &str {
        match self.peek() {
            Some(Token::Op(op)) => op,
            Some(Token::Question) => "?",
            Some(Token::Hash) => "#",
            Some(Token::Comma) => ",",
            Some(Token::LParen) => "(",
            Some(Token::RParen) => ")",
            Some(Token::Symbol(value)) => value,
            Some(Token::Str(_)) => "<string>",
            Some(Token::Int(_)) => "<int>",
            None => "<end>",
        }
    }

    fn consume_question(&mut self) -> bool {
        if self.peek() == Some(&Token::Question) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn consume_hash(&mut self) -> bool {
        if self.peek() == Some(&Token::Hash) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn consume_lparen(&mut self) -> bool {
        if self.peek() == Some(&Token::LParen) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn consume_rparen(&mut self) -> bool {
        if self.peek() == Some(&Token::RParen) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn consume_comma(&mut self) -> bool {
        if self.peek() == Some(&Token::Comma) {
            self.position += 1;
            true
        } else {
            false
        }
    }
}

trait ValueExt {
    fn as_int(&self) -> i64;
    fn as_bool(&self) -> bool;
    fn to_display_string(&self) -> String;
}

impl ValueExt for Value {
    fn as_int(&self) -> i64 {
        match self {
            Value::Int(value) => *value,
            Value::Str(value) => value.parse().unwrap_or(0),
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            Value::Int(value) => *value != 0,
            Value::Str(value) => !value.is_empty(),
        }
    }

    fn to_display_string(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Str(value) => value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::csv::{CsvCatalog, NameTable};
    use crate::runtime::{VarAddress, VarStore};

    use super::*;

    fn empty_resolver() -> (CsvCatalog, VarStore) {
        (CsvCatalog::default(), VarStore::default())
    }

    #[test]
    fn evaluates_arithmetic_and_comparison() {
        let (csv, store) = empty_resolver();
        let resolver = VarResolver::new(&csv);
        let expr = parse("1 + 2 * 3 == 7").unwrap();
        assert_eq!(eval(&expr, &resolver, &store), Ok(Value::Int(1)));
    }

    #[test]
    fn evaluates_emuera_conditional() {
        let (csv, store) = empty_resolver();
        let resolver = VarResolver::new(&csv);
        let expr = parse("0 ? 10 # 20").unwrap();
        assert_eq!(eval(&expr, &resolver, &store), Ok(Value::Int(20)));
    }

    #[test]
    fn evaluates_variables_with_csv_name_indexes() {
        let mut csv = CsvCatalog::default();
        csv.name_tables.insert(
            "TALENT".to_owned(),
            NameTable {
                source_file: PathBuf::from("CSV/Talent.csv"),
                by_id: HashMap::from([(2, "性別".to_owned())]),
                by_name: HashMap::from([("性別".to_owned(), 2)]),
            },
        );
        let resolver = VarResolver::new(&csv);
        let mut store = VarStore::default();
        store.set(
            VarAddress {
                name: "TALENT".to_owned(),
                indexes: vec![0, 2],
            },
            Value::Int(3),
        );
        let expr = parse("TALENT:ARG:性別 & 1").unwrap();
        assert_eq!(eval(&expr, &resolver, &store), Ok(Value::Int(1)));
        let expr = parse("TALENT:ARG:性別 == 3").unwrap();
        assert_eq!(eval(&expr, &resolver, &store), Ok(Value::Int(1)));
    }
}
