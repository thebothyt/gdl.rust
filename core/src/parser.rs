use crate::ast::*;
use crate::lexer::{Token, T};
use std::rc::Rc;

type R<X> = Result<X, String>;

const MAX_NEST: usize = 100;

pub fn parse(toks: Vec<Token>) -> R<Vec<Stmt>> {
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
    };
    let mut out = Vec::new();
    while p.peek().t != T::Eof {
        out.push(p.stmt()?);
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
}

fn compound(op: BinOp, name: &str, rhs: Expr) -> Expr {
    Expr::Bin(op, Box::new(Expr::Var(name.to_string())), Box::new(rhs))
}

fn binop_info(t: &T) -> Option<(u8, BinOp)> {
    Some(match t {
        T::Or => (1, BinOp::Or),
        T::And => (2, BinOp::And),
        T::Eq => (3, BinOp::Eq),
        T::Ne => (3, BinOp::Ne),
        T::Lt => (4, BinOp::Lt),
        T::Le => (4, BinOp::Le),
        T::Gt => (4, BinOp::Gt),
        T::Ge => (4, BinOp::Ge),
        T::Plus => (5, BinOp::Add),
        T::Minus => (5, BinOp::Sub),
        T::Star => (6, BinOp::Mul),
        T::Slash => (6, BinOp::Div),
        T::Percent => (6, BinOp::Mod),
        _ => return None,
    })
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn peek_at(&self, k: usize) -> &T {
        &self.toks[(self.pos + k).min(self.toks.len() - 1)].t
    }

    fn next(&mut self) -> Token {
        let t = self.peek().clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn err<X>(&self, msg: &str) -> R<X> {
        let t = self.peek();
        Err(format!("line {}: {} (found {:?})", t.line, msg, t.t))
    }

    fn expect(&mut self, t: T, what: &str) -> R<Token> {
        if self.peek().t == t {
            Ok(self.next())
        } else {
            self.err(&format!("expected {what}"))
        }
    }

    fn ident(&mut self, what: &str) -> R<String> {
        if let T::Ident(s) = self.peek().t.clone() {
            self.pos += 1;
            Ok(s)
        } else {
            self.err(&format!("expected {what}"))
        }
    }

    fn enter(&mut self) -> R<()> {
        self.depth += 1;
        if self.depth > MAX_NEST {
            return self.err("nesting is too deep");
        }
        Ok(())
    }

    fn block(&mut self) -> R<Vec<Stmt>> {
        self.expect(T::LBrace, "'{'")?;
        self.enter()?;
        let mut v = Vec::new();
        while !matches!(self.peek().t, T::RBrace | T::Eof) {
            v.push(self.stmt()?);
        }
        self.expect(T::RBrace, "'}'")?;
        self.depth -= 1;
        Ok(v)
    }

    fn stmt(&mut self) -> R<Stmt> {
        let line = self.peek().line;
        let kind = match self.peek().t.clone() {
            T::Let => {
                self.pos += 1;
                let name = self.ident("a variable name")?;
                self.expect(T::Assign, "'='")?;
                let e = self.expr()?;
                StmtKind::Let(name, e)
            }
            T::Fn => {
                self.pos += 1;
                let name = self.ident("a function name")?;
                self.expect(T::LParen, "'('")?;
                let mut params = Vec::new();
                if self.peek().t != T::RParen {
                    loop {
                        params.push(self.ident("a parameter name")?);
                        if self.peek().t == T::Comma {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                }
                self.expect(T::RParen, "')'")?;
                let body = self.block()?;
                StmtKind::Fn(Rc::new(FnDecl { name, params, body }))
            }
            T::If => self.if_stmt()?,
            T::For => {
                self.pos += 1;
                let var = self.ident("a loop variable")?;
                self.expect(T::In, "'in'")?;
                let start = self.expr()?;
                self.expect(T::DotDot, "'..'")?;
                let end = self.expr()?;
                let step = if self.peek().t == T::Step {
                    self.pos += 1;
                    Some(self.expr()?)
                } else {
                    None
                };
                let body = self.block()?;
                StmtKind::For {
                    var,
                    start,
                    end,
                    step,
                    body,
                }
            }
            T::While => {
                self.pos += 1;
                let cond = self.expr()?;
                let body = self.block()?;
                StmtKind::While(cond, body)
            }
            T::Return => {
                self.pos += 1;
                let t = self.peek();
                let none = t.nl || matches!(t.t, T::RBrace | T::Eof | T::Semi);
                StmtKind::Return(if none { None } else { Some(self.expr()?) })
            }
            T::Ident(name)
                if matches!(
                    self.peek_at(1),
                    T::Assign | T::PlusEq | T::MinusEq | T::StarEq | T::SlashEq
                ) =>
            {
                self.pos += 1;
                let op = self.next().t;
                let rhs = self.expr()?;
                let e = match op {
                    T::PlusEq => compound(BinOp::Add, &name, rhs),
                    T::MinusEq => compound(BinOp::Sub, &name, rhs),
                    T::StarEq => compound(BinOp::Mul, &name, rhs),
                    T::SlashEq => compound(BinOp::Div, &name, rhs),
                    _ => rhs,
                };
                StmtKind::Assign(name, e)
            }
            _ => StmtKind::Expr(self.expr()?),
        };
        if self.peek().t == T::Semi {
            self.pos += 1;
        }
        Ok(Stmt { line, kind })
    }

    fn if_stmt(&mut self) -> R<StmtKind> {
        self.pos += 1; // `if`
        let cond = self.expr()?;
        let then = self.block()?;
        let els = if self.peek().t == T::Else {
            self.pos += 1;
            if self.peek().t == T::If {
                let line = self.peek().line;
                let kind = self.if_stmt()?;
                Some(vec![Stmt { line, kind }])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(StmtKind::If(cond, then, els))
    }

    fn expr(&mut self) -> R<Expr> {
        self.bin(1)
    }

    fn bin(&mut self, min_prec: u8) -> R<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let tok = self.peek();
            if tok.nl {
                break;
            }
            let Some((prec, op)) = binop_info(&tok.t) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.pos += 1;
            let rhs = self.bin(prec + 1)?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> R<Expr> {
        self.enter()?;
        let e = match self.peek().t.clone() {
            T::Minus => {
                self.pos += 1;
                Expr::Neg(Box::new(self.unary()?))
            }
            T::Not => {
                self.pos += 1;
                Expr::Not(Box::new(self.unary()?))
            }
            _ => self.primary()?,
        };
        self.depth -= 1;
        Ok(e)
    }

    fn primary(&mut self) -> R<Expr> {
        let tok = self.next();
        match tok.t {
            T::Num(n) => Ok(Expr::Num(n)),
            T::Str(s) => Ok(Expr::Str(s)),
            T::True => Ok(Expr::Bool(true)),
            T::False => Ok(Expr::Bool(false)),
            T::Nil => Ok(Expr::Nil),
            T::Ident(name) => {
                if self.peek().t == T::LParen && !self.peek().nl {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek().t != T::RParen {
                        loop {
                            if let (T::Ident(k), T::Assign) = (self.peek().t.clone(), self.peek_at(1).clone()) {
                                self.pos += 2;
                                let v = self.expr()?;
                                args.push(Expr::Named(k, Box::new(v)));
                            } else {
                                args.push(self.expr()?);
                            }
                            if self.peek().t == T::Comma {
                                self.pos += 1;
                                if self.peek().t == T::RParen {
                                    break; // trailing comma
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(T::RParen, "')'")?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            T::LParen => {
                let e = self.expr()?;
                self.expect(T::RParen, "')'")?;
                Ok(e)
            }
            other => Err(format!(
                "line {}: unexpected {:?} in expression",
                tok.line, other
            )),
        }
    }
}
