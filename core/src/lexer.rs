#[derive(Clone, Debug, PartialEq)]
pub enum T {
    Num(f64),
    Str(String),
    Ident(String),
    // keywords
    Let,
    Fn,
    If,
    Else,
    For,
    In,
    Step,
    While,
    Return,
    True,
    False,
    Nil,
    // operators / punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    DotDot,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub t: T,
    pub line: usize,
    /// True if a newline came between the previous token and this one.
    /// The parser uses it so an operator or `(` at the start of a line
    /// begins a new statement instead of continuing the last one.
    pub nl: bool,
}

pub fn lex(src: &str) -> Result<Vec<Token>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1usize;
    let mut nl = false;
    let mut out: Vec<Token> = Vec::new();

    macro_rules! push {
        ($t:expr) => {{
            out.push(Token { t: $t, line, nl });
            nl = false;
        }};
    }

    while i < c.len() {
        let ch = c[i];
        match ch {
            '\n' => {
                line += 1;
                nl = true;
                i += 1;
            }
            ' ' | '\t' | '\r' => i += 1,
            '/' if c.get(i + 1) == Some(&'/') => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '0'..='9' => {
                let st = i;
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
                // a fraction needs a digit after the dot, so `0..10` stays a range
                if i + 1 < c.len() && c[i] == '.' && c[i + 1].is_ascii_digit() {
                    i += 1;
                    while i < c.len() && c[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let s: String = c[st..i].iter().collect();
                let n: f64 = s
                    .parse()
                    .map_err(|_| format!("line {line}: bad number '{s}'"))?;
                push!(T::Num(n));
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                loop {
                    if i >= c.len() {
                        return Err(format!("line {line}: unterminated string"));
                    }
                    let d = c[i];
                    i += 1;
                    match d {
                        '"' => break,
                        '\\' => {
                            let e = c
                                .get(i)
                                .copied()
                                .ok_or_else(|| format!("line {line}: unterminated string"))?;
                            i += 1;
                            s.push(match e {
                                'n' => '\n',
                                't' => '\t',
                                other => other,
                            });
                        }
                        '\n' => {
                            line += 1;
                            s.push('\n');
                        }
                        other => s.push(other),
                    }
                }
                push!(T::Str(s));
            }
            c0 if c0.is_alphabetic() || c0 == '_' => {
                let st = i;
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
                let word: String = c[st..i].iter().collect();
                let t = match word.as_str() {
                    "let" => T::Let,
                    "fn" => T::Fn,
                    "if" => T::If,
                    "else" => T::Else,
                    "for" => T::For,
                    "in" => T::In,
                    "step" => T::Step,
                    "while" => T::While,
                    "return" => T::Return,
                    "true" => T::True,
                    "false" => T::False,
                    "nil" => T::Nil,
                    _ => T::Ident(word),
                };
                push!(t);
            }
            _ => {
                let n = c.get(i + 1).copied();
                let (t, len) = match (ch, n) {
                    ('=', Some('=')) => (T::Eq, 2),
                    ('!', Some('=')) => (T::Ne, 2),
                    ('<', Some('=')) => (T::Le, 2),
                    ('>', Some('=')) => (T::Ge, 2),
                    ('&', Some('&')) => (T::And, 2),
                    ('|', Some('|')) => (T::Or, 2),
                    ('.', Some('.')) => (T::DotDot, 2),
                    ('+', Some('=')) => (T::PlusEq, 2),
                    ('-', Some('=')) => (T::MinusEq, 2),
                    ('*', Some('=')) => (T::StarEq, 2),
                    ('/', Some('=')) => (T::SlashEq, 2),
                    ('=', _) => (T::Assign, 1),
                    ('!', _) => (T::Not, 1),
                    ('<', _) => (T::Lt, 1),
                    ('>', _) => (T::Gt, 1),
                    ('+', _) => (T::Plus, 1),
                    ('-', _) => (T::Minus, 1),
                    ('*', _) => (T::Star, 1),
                    ('/', _) => (T::Slash, 1),
                    ('%', _) => (T::Percent, 1),
                    ('(', _) => (T::LParen, 1),
                    (')', _) => (T::RParen, 1),
                    ('{', _) => (T::LBrace, 1),
                    ('}', _) => (T::RBrace, 1),
                    (',', _) => (T::Comma, 1),
                    (';', _) => (T::Semi, 1),
                    _ => return Err(format!("line {line}: unexpected character '{ch}'")),
                };
                i += len;
                push!(t);
            }
        }
    }
    out.push(Token {
        t: T::Eof,
        line,
        nl: true,
    });
    Ok(out)
}
