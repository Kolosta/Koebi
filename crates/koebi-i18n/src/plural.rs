//! Plural rules: the `plural=` expression of a PO header (`n != 1`, `n > 1`,
//! `n%10==1 && n%100!=11 ? 0 : ...`), compiled to a short program evaluated on a fixed stack.
//!
//! The grammar is gettext's, a subset of C on unsigned integers: `n`, decimal numbers,
//! parentheses, `!`, `* / %`, `+ -`, `< <= > >=`, `== !=`, `&&`, `||` and `? :`, with C's
//! precedence. A comparison is 1 or 0; dividing by zero gives 0 rather than failing.
//!
//! This module has no dependency: the build script and `cargo xtask i18n` include it as it is.

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binary {
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `&&`
    And,
    /// `||`
    Or,
}

impl Binary {
    fn apply(self, a: u64, b: u64) -> u64 {
        match self {
            Self::Mul => a.wrapping_mul(b),
            Self::Div => a.checked_div(b).unwrap_or(0),
            Self::Rem => a.checked_rem(b).unwrap_or(0),
            Self::Add => a.wrapping_add(b),
            Self::Sub => a.wrapping_sub(b),
            Self::Less => u64::from(a < b),
            Self::LessEqual => u64::from(a <= b),
            Self::Greater => u64::from(a > b),
            Self::GreaterEqual => u64::from(a >= b),
            Self::Equal => u64::from(a == b),
            Self::NotEqual => u64::from(a != b),
            Self::And => u64::from(a != 0 && b != 0),
            Self::Or => u64::from(a != 0 || b != 0),
        }
    }

    /// Its name in Rust, for generated code.
    pub fn name(self) -> &'static str {
        match self {
            Self::Mul => "Mul",
            Self::Div => "Div",
            Self::Rem => "Rem",
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Less => "Less",
            Self::LessEqual => "LessEqual",
            Self::Greater => "Greater",
            Self::GreaterEqual => "GreaterEqual",
            Self::Equal => "Equal",
            Self::NotEqual => "NotEqual",
            Self::And => "And",
            Self::Or => "Or",
        }
    }
}

/// One step of a compiled rule, in postfix order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Pushes the count.
    N,
    /// Pushes a number.
    Number(u64),
    /// Replaces the top by 1 when it is 0, else by 0.
    Not,
    /// Replaces the two top values by the operator applied to them.
    Binary(Binary),
    /// Replaces the three top values (condition, then, else) by the one chosen.
    Select,
}

/// Deepest stack a rule may need: every rule of the CLDR languages needs less than 8.
pub const STACK: usize = 16;

/// Compiles the expression after `plural=` (its trailing `;` allowed).
///
/// # Errors
///
/// What is wrong with the expression: an unknown character, a missing operand or parenthesis,
/// or a rule deeper than [`STACK`].
pub fn compile(expression: &str) -> Result<Vec<Op>, String> {
    let tokens = tokenize(expression.trim().trim_end_matches(';'))?;
    let mut parser = Parser {
        tokens: &tokens,
        at: 0,
        ops: Vec::new(),
    };
    parser.ternary()?;
    if parser.at != tokens.len() {
        return Err(format!("unexpected {:?}", tokens[parser.at]));
    }
    if depth(&parser.ops) > STACK {
        return Err(format!("the rule needs more than {STACK} values at once"));
    }
    Ok(parser.ops)
}

/// The form the rule `ops` gives for the count `n`.
pub fn eval(ops: &[Op], n: u64) -> u64 {
    let mut stack = [0_u64; STACK];
    let mut top = 0;
    for op in ops {
        match *op {
            Op::N | Op::Number(_) => {
                if top == STACK {
                    return 0;
                }
                stack[top] = if let Op::Number(value) = *op {
                    value
                } else {
                    n
                };
                top += 1;
            }
            Op::Not => {
                if top == 0 {
                    return 0;
                }
                stack[top - 1] = u64::from(stack[top - 1] == 0);
            }
            Op::Binary(binary) => {
                if top < 2 {
                    return 0;
                }
                top -= 1;
                stack[top - 1] = binary.apply(stack[top - 1], stack[top]);
            }
            Op::Select => {
                if top < 3 {
                    return 0;
                }
                top -= 2;
                let (condition, then, otherwise) = (stack[top - 1], stack[top], stack[top + 1]);
                stack[top - 1] = if condition == 0 { otherwise } else { then };
            }
        }
    }
    if top == 0 { 0 } else { stack[top - 1] }
}

/// How many values the rule holds at once, at most.
fn depth(ops: &[Op]) -> usize {
    let mut top: usize = 0;
    let mut deepest = 0;
    for op in ops {
        match op {
            Op::N | Op::Number(_) => top += 1,
            Op::Not => {}
            Op::Binary(_) => top = top.saturating_sub(1),
            Op::Select => top = top.saturating_sub(2),
        }
        deepest = deepest.max(top);
    }
    deepest
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    N,
    Number(u64),
    Open,
    Close,
    Question,
    Colon,
    Not,
    Binary(Binary),
}

fn tokenize(text: &str) -> Result<Vec<Token>, String> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let next = bytes.get(i + 1).copied();
        let (token, len) = match (c, next) {
            (b' ' | b'\t', _) => {
                i += 1;
                continue;
            }
            (b'n', _) => (Token::N, 1),
            (b'0'..=b'9', _) => {
                let end = bytes[i..]
                    .iter()
                    .position(|b| !b.is_ascii_digit())
                    .map_or(bytes.len(), |len| i + len);
                let value = text[i..end]
                    .parse()
                    .map_err(|_| format!("{} is too large", &text[i..end]))?;
                (Token::Number(value), end - i)
            }
            (b'(', _) => (Token::Open, 1),
            (b')', _) => (Token::Close, 1),
            (b'?', _) => (Token::Question, 1),
            (b':', _) => (Token::Colon, 1),
            (b'&', Some(b'&')) => (Token::Binary(Binary::And), 2),
            (b'|', Some(b'|')) => (Token::Binary(Binary::Or), 2),
            (b'=', Some(b'=')) => (Token::Binary(Binary::Equal), 2),
            (b'!', Some(b'=')) => (Token::Binary(Binary::NotEqual), 2),
            (b'<', Some(b'=')) => (Token::Binary(Binary::LessEqual), 2),
            (b'>', Some(b'=')) => (Token::Binary(Binary::GreaterEqual), 2),
            (b'!', _) => (Token::Not, 1),
            (b'<', _) => (Token::Binary(Binary::Less), 1),
            (b'>', _) => (Token::Binary(Binary::Greater), 1),
            (b'*', _) => (Token::Binary(Binary::Mul), 1),
            (b'/', _) => (Token::Binary(Binary::Div), 1),
            (b'%', _) => (Token::Binary(Binary::Rem), 1),
            (b'+', _) => (Token::Binary(Binary::Add), 1),
            (b'-', _) => (Token::Binary(Binary::Sub), 1),
            _ => {
                let rest: String = text[i..].chars().take(1).collect();
                return Err(format!("unexpected {rest:?}"));
            }
        };
        tokens.push(token);
        i += len;
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    ops: Vec<Op>,
}

/// The binary operators by precedence, loosest first.
const LEVELS: [&[Binary]; 6] = [
    &[Binary::Or],
    &[Binary::And],
    &[Binary::Equal, Binary::NotEqual],
    &[
        Binary::Less,
        Binary::LessEqual,
        Binary::Greater,
        Binary::GreaterEqual,
    ],
    &[Binary::Add, Binary::Sub],
    &[Binary::Mul, Binary::Div, Binary::Rem],
];

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).copied()
    }

    fn expect(&mut self, token: Token, what: &str) -> Result<(), String> {
        if self.peek() == Some(token) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected {what}"))
        }
    }

    fn ternary(&mut self) -> Result<(), String> {
        self.binary(0)?;
        if self.peek() == Some(Token::Question) {
            self.at += 1;
            self.ternary()?;
            self.expect(Token::Colon, "':' after '?'")?;
            self.ternary()?;
            self.ops.push(Op::Select);
        }
        Ok(())
    }

    fn binary(&mut self, level: usize) -> Result<(), String> {
        let Some(operators) = LEVELS.get(level) else {
            return self.unary();
        };
        self.binary(level + 1)?;
        while let Some(Token::Binary(binary)) = self.peek() {
            if !operators.contains(&binary) {
                break;
            }
            self.at += 1;
            self.binary(level + 1)?;
            self.ops.push(Op::Binary(binary));
        }
        Ok(())
    }

    fn unary(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(Token::Not) => {
                self.at += 1;
                self.unary()?;
                self.ops.push(Op::Not);
            }
            Some(Token::N) => {
                self.at += 1;
                self.ops.push(Op::N);
            }
            Some(Token::Number(value)) => {
                self.at += 1;
                self.ops.push(Op::Number(value));
            }
            Some(Token::Open) => {
                self.at += 1;
                self.ternary()?;
                self.expect(Token::Close, "')'")?;
            }
            Some(token) => return Err(format!("unexpected {token:?}")),
            None => return Err("the rule ends too early".to_owned()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forms(rule: &str, counts: &[u64]) -> Vec<u64> {
        let ops = compile(rule).unwrap();
        counts.iter().map(|&n| eval(&ops, n)).collect()
    }

    #[test]
    fn english_and_french_differ_at_zero() {
        assert_eq!(forms("(n != 1);", &[0, 1, 2, 5]), [1, 0, 1, 1]);
        assert_eq!(forms("(n > 1)", &[0, 1, 2, 5]), [0, 0, 1, 1]);
        assert_eq!(forms("0;", &[0, 1, 2]), [0, 0, 0]);
    }

    #[test]
    fn slavic_rules_read_with_c_precedence() {
        let russian =
            "(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2)";
        assert_eq!(
            forms(russian, &[1, 2, 5, 11, 12, 21, 22, 25, 101, 111]),
            [0, 1, 2, 2, 2, 0, 1, 2, 0, 2]
        );
        let arabic =
            "n==0 ? 0 : n==1 ? 1 : n==2 ? 2 : n%100>=3 && n%100<=10 ? 3 : n%100>=11 ? 4 : 5";
        assert_eq!(
            forms(arabic, &[0, 1, 2, 3, 11, 100, 102]),
            [0, 1, 2, 3, 4, 5, 5]
        );
    }

    #[test]
    fn broken_rules_are_refused() {
        for rule in [
            "",
            "n +",
            "(n > 1",
            "n ? 1",
            "m",
            "n == 1 2",
            "99999999999999999999",
        ] {
            assert!(compile(rule).is_err(), "{rule:?}");
        }
        assert_eq!(forms("n / 0 + n % 0 + !n", &[0, 3]), [1, 0]);
    }
}
