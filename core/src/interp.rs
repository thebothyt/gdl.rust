use crate::ast::*;
use crate::level::{fmt_num, Obj, RunOutput};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

// Limits keep a hosted Worker safe from runaway scripts.
const MAX_STEPS: u64 = 3_000_000;
const MAX_OBJECTS: usize = 150_000;
const MAX_DEPTH: usize = 100;

pub const BUILTINS: &[&str] = &[
    "obj", "set", "add_group", "group", "color", "move", "alpha", "toggle", "print", "floor",
    "ceil", "round", "abs", "sqrt", "sin", "cos", "min", "max", "pow",
];

type R<X> = Result<X, String>;
type Scope = HashMap<String, Value>;

fn err(line: usize, msg: impl AsRef<str>) -> String {
    format!("line {line}: {}", msg.as_ref())
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
    Nil,
}

impl Value {
    fn truthy(&self) -> bool {
        !matches!(self, Value::Bool(false) | Value::Nil) && *self != Value::Num(0.0)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Num(n) => f.write_str(&fmt_num(*n)),
            Value::Str(s) => f.write_str(s),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => f.write_str("nil"),
        }
    }
}

enum Flow {
    Next,
    Return(Value),
}

struct Interp {
    funcs: HashMap<String, Rc<FnDecl>>,
    /// One entry per call; each call holds a stack of block scopes. Frame 0 is global.
    frames: Vec<Vec<Scope>>,
    objects: Vec<Obj>,
    logs: Vec<String>,
    next_group: u32,
    next_color: u32,
    steps: u64,
}

pub fn execute(prog: &[Stmt]) -> R<RunOutput> {
    let mut it = Interp::new();
    // hoist top-level functions so they can be called before their definition
    for s in prog {
        if let StmtKind::Fn(d) = &s.kind {
            it.define_fn(d, s.line)?;
        }
    }
    it.exec_stmts(prog)?;
    Ok(RunOutput {
        objects: it.objects,
        logs: it.logs,
    })
}

fn set_in(frame: &mut [Scope], name: &str, v: Value) -> Option<Value> {
    for sc in frame.iter_mut().rev() {
        if let Some(slot) = sc.get_mut(name) {
            *slot = v;
            return None;
        }
    }
    Some(v)
}

impl Interp {
    fn new() -> Self {
        let mut globals = Scope::new();
        globals.insert("PI".to_string(), Value::Num(std::f64::consts::PI));
        Interp {
            funcs: HashMap::new(),
            frames: vec![vec![globals]],
            objects: Vec::new(),
            logs: Vec::new(),
            next_group: 1,
            next_color: 1,
            steps: 0,
        }
    }

    fn define_fn(&mut self, d: &Rc<FnDecl>, line: usize) -> R<()> {
        if BUILTINS.contains(&d.name.as_str()) {
            return Err(err(line, format!("'{}' is a builtin and can't be redefined", d.name)));
        }
        self.funcs.insert(d.name.clone(), d.clone());
        Ok(())
    }

    fn tick(&mut self, line: usize) -> R<()> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(err(line, "script ran too long (step limit reached)"));
        }
        Ok(())
    }

    // ---- scopes ----

    fn push_scope(&mut self) {
        self.frames.last_mut().unwrap().push(Scope::new());
    }

    fn pop_scope(&mut self) {
        self.frames.last_mut().unwrap().pop();
    }

    fn declare(&mut self, name: &str, v: Value) {
        self.frames
            .last_mut()
            .unwrap()
            .last_mut()
            .unwrap()
            .insert(name.to_string(), v);
    }

    fn lookup(&self, name: &str) -> Option<Value> {
        for sc in self.frames.last()?.iter().rev() {
            if let Some(v) = sc.get(name) {
                return Some(v.clone());
            }
        }
        if self.frames.len() > 1 {
            for sc in self.frames[0].iter().rev() {
                if let Some(v) = sc.get(name) {
                    return Some(v.clone());
                }
            }
        }
        None
    }

    fn assign(&mut self, name: &str, v: Value, line: usize) -> R<()> {
        let n = self.frames.len();
        let v = match set_in(&mut self.frames[n - 1], name, v) {
            None => return Ok(()),
            Some(v) => v,
        };
        if n > 1 && set_in(&mut self.frames[0], name, v).is_none() {
            return Ok(());
        }
        Err(err(line, format!("undefined variable '{name}' (declare it with let)")))
    }

    // ---- statements ----

    fn exec_block(&mut self, stmts: &[Stmt]) -> R<Flow> {
        self.push_scope();
        let r = self.exec_stmts(stmts);
        self.pop_scope();
        r
    }

    fn exec_stmts(&mut self, stmts: &[Stmt]) -> R<Flow> {
        for s in stmts {
            if let Flow::Return(v) = self.exec(s)? {
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Next)
    }

    fn exec(&mut self, s: &Stmt) -> R<Flow> {
        let line = s.line;
        self.tick(line)?;
        match &s.kind {
            StmtKind::Let(name, e) => {
                let v = self.eval(e, line)?;
                self.declare(name, v);
            }
            StmtKind::Assign(name, e) => {
                let v = self.eval(e, line)?;
                self.assign(name, v, line)?;
            }
            StmtKind::Expr(e) => {
                self.eval(e, line)?;
            }
            StmtKind::If(cond, then, els) => {
                if self.eval(cond, line)?.truthy() {
                    return self.exec_block(then);
                } else if let Some(els) = els {
                    return self.exec_block(els);
                }
            }
            StmtKind::For {
                var,
                start,
                end,
                step,
                body,
            } => {
                let s0 = self.num(start, line)?;
                let e0 = self.num(end, line)?;
                let st = match step {
                    Some(x) => self.num(x, line)?,
                    None => 1.0,
                };
                if st == 0.0 {
                    return Err(err(line, "loop step can't be 0"));
                }
                let mut i = s0;
                while (st > 0.0 && i < e0) || (st < 0.0 && i > e0) {
                    self.tick(line)?;
                    self.push_scope();
                    self.declare(var, Value::Num(i));
                    let r = self.exec_stmts(body);
                    self.pop_scope();
                    if let Flow::Return(v) = r? {
                        return Ok(Flow::Return(v));
                    }
                    i += st;
                }
            }
            StmtKind::While(cond, body) => loop {
                self.tick(line)?;
                if !self.eval(cond, line)?.truthy() {
                    break;
                }
                if let Flow::Return(v) = self.exec_block(body)? {
                    return Ok(Flow::Return(v));
                }
            },
            StmtKind::Fn(d) => self.define_fn(d, line)?,
            StmtKind::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, line)?,
                    None => Value::Nil,
                };
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Next)
    }

    // ---- expressions ----

    fn num(&mut self, e: &Expr, line: usize) -> R<f64> {
        match self.eval(e, line)? {
            Value::Num(n) => Ok(n),
            other => Err(err(line, format!("expected a number, got {other}"))),
        }
    }

    fn eval(&mut self, e: &Expr, line: usize) -> R<Value> {
        match e {
            Expr::Num(n) => Ok(Value::Num(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),
            Expr::Var(n) => self
                .lookup(n)
                .ok_or_else(|| err(line, format!("undefined variable '{n}'"))),
            Expr::Neg(x) => Ok(Value::Num(-self.num(x, line)?)),
            Expr::Not(x) => Ok(Value::Bool(!self.eval(x, line)?.truthy())),
            Expr::Bin(op, a, b) => self.eval_bin(*op, a, b, line),
            Expr::Call(name, args) => self.call(name, args, line),
        }
    }

    fn eval_bin(&mut self, op: BinOp, a: &Expr, b: &Expr, line: usize) -> R<Value> {
        match op {
            BinOp::And => {
                let l = self.eval(a, line)?;
                if !l.truthy() {
                    return Ok(Value::Bool(false));
                }
                return Ok(Value::Bool(self.eval(b, line)?.truthy()));
            }
            BinOp::Or => {
                let l = self.eval(a, line)?;
                if l.truthy() {
                    return Ok(Value::Bool(true));
                }
                return Ok(Value::Bool(self.eval(b, line)?.truthy()));
            }
            _ => {}
        }
        let l = self.eval(a, line)?;
        let r = self.eval(b, line)?;
        match (op, &l, &r) {
            (BinOp::Add, Value::Str(_), _) | (BinOp::Add, _, Value::Str(_)) => {
                Ok(Value::Str(format!("{l}{r}")))
            }
            (BinOp::Eq, _, _) => Ok(Value::Bool(l == r)),
            (BinOp::Ne, _, _) => Ok(Value::Bool(l != r)),
            (_, Value::Num(x), Value::Num(y)) => {
                let (x, y) = (*x, *y);
                Ok(match op {
                    BinOp::Add => Value::Num(x + y),
                    BinOp::Sub => Value::Num(x - y),
                    BinOp::Mul => Value::Num(x * y),
                    BinOp::Div => {
                        if y == 0.0 {
                            return Err(err(line, "division by zero"));
                        }
                        Value::Num(x / y)
                    }
                    BinOp::Mod => {
                        if y == 0.0 {
                            return Err(err(line, "modulo by zero"));
                        }
                        Value::Num(x % y)
                    }
                    BinOp::Lt => Value::Bool(x < y),
                    BinOp::Le => Value::Bool(x <= y),
                    BinOp::Gt => Value::Bool(x > y),
                    BinOp::Ge => Value::Bool(x >= y),
                    _ => unreachable!(),
                })
            }
            _ => Err(err(line, format!("that operator needs two numbers, got {l} and {r}"))),
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], line: usize) -> R<Value> {
        let mut vals = Vec::with_capacity(args.len());
        for a in args {
            vals.push(self.eval(a, line)?);
        }
        if let Some(f) = self.funcs.get(name).cloned() {
            return self.call_user(&f, vals, line);
        }
        self.builtin(name, vals, line)
    }

    fn call_user(&mut self, f: &Rc<FnDecl>, vals: Vec<Value>, line: usize) -> R<Value> {
        if vals.len() != f.params.len() {
            return Err(err(
                line,
                format!("{}() takes {} argument(s), got {}", f.name, f.params.len(), vals.len()),
            ));
        }
        if self.frames.len() > MAX_DEPTH {
            return Err(err(line, "recursion is too deep"));
        }
        let mut scope = Scope::new();
        for (p, v) in f.params.iter().zip(vals) {
            scope.insert(p.clone(), v);
        }
        self.frames.push(vec![scope]);
        let r = self.exec_stmts(&f.body);
        self.frames.pop();
        match r? {
            Flow::Return(v) => Ok(v),
            Flow::Next => Ok(Value::Nil),
        }
    }

    // ---- builtins ----

    fn push_obj(&mut self, props: &[(u32, String)], line: usize) -> R<f64> {
        if self.objects.len() >= MAX_OBJECTS {
            return Err(err(line, format!("too many objects (limit {MAX_OBJECTS})")));
        }
        let mut o = Obj::default();
        for (k, v) in props {
            o.props.insert(*k, v.clone());
        }
        self.objects.push(o);
        Ok((self.objects.len() - 1) as f64)
    }

    fn trigger(&mut self, id: u32, x: f64, y: f64, extra: &[(u32, f64)], line: usize) -> R<Value> {
        let mut p = vec![
            (1, id.to_string()),
            (2, fmt_num(x)),
            (3, fmt_num(y)),
            (36, "1".to_string()),
        ];
        for (k, v) in extra {
            p.push((*k, fmt_num(*v)));
        }
        Ok(Value::Num(self.push_obj(&p, line)?))
    }

    fn handle(&self, x: f64, name: &str, line: usize) -> R<usize> {
        if x >= 0.0 && x.fract() == 0.0 && (x as usize) < self.objects.len() {
            Ok(x as usize)
        } else {
            Err(err(line, format!("{name}(): {} is not a valid object handle", fmt_num(x))))
        }
    }

    fn builtin(&mut self, name: &str, a: Vec<Value>, line: usize) -> R<Value> {
        let want = |n: usize| -> R<()> {
            if a.len() == n {
                Ok(())
            } else {
                Err(err(line, format!("{name}() takes {n} argument(s), got {}", a.len())))
            }
        };
        let n = |i: usize| -> R<f64> {
            match a.get(i) {
                Some(Value::Num(x)) => Ok(*x),
                _ => Err(err(line, format!("{name}(): argument {} must be a number", i + 1))),
            }
        };
        match name {
            "obj" => {
                want(3)?;
                let props = [
                    (1, fmt_num(n(0)?)),
                    (2, fmt_num(n(1)?)),
                    (3, fmt_num(n(2)?)),
                ];
                Ok(Value::Num(self.push_obj(&props, line)?))
            }
            "set" => {
                want(3)?;
                let h = self.handle(n(0)?, name, line)?;
                let key = n(1)?;
                if key < 0.0 || key.fract() != 0.0 {
                    return Err(err(line, "set(): key must be a whole number"));
                }
                let v = match &a[2] {
                    Value::Num(x) => fmt_num(*x),
                    Value::Str(s) => s.clone(),
                    Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
                    Value::Nil => return Err(err(line, "set(): value can't be nil")),
                };
                self.objects[h].props.insert(key as u32, v);
                Ok(Value::Nil)
            }
            "add_group" => {
                want(2)?;
                let h = self.handle(n(0)?, name, line)?;
                let g = fmt_num(n(1)?);
                let o = &mut self.objects[h];
                let merged = match o.props.get(&57) {
                    Some(existing) => format!("{existing}.{g}"),
                    None => g,
                };
                o.props.insert(57, merged);
                Ok(Value::Nil)
            }
            "group" => {
                want(0)?;
                let g = self.next_group;
                self.next_group += 1;
                Ok(Value::Num(g as f64))
            }
            "color" => {
                want(0)?;
                let c = self.next_color;
                self.next_color += 1;
                Ok(Value::Num(c as f64))
            }
            // move(x, y, target_group, dx, dy, seconds)
            "move" => {
                want(6)?;
                self.trigger(
                    901,
                    n(0)?,
                    n(1)?,
                    &[(51, n(2)?), (28, n(3)?), (29, n(4)?), (10, n(5)?)],
                    line,
                )
            }
            // alpha(x, y, target_group, opacity 0..1, seconds)
            "alpha" => {
                want(5)?;
                self.trigger(1007, n(0)?, n(1)?, &[(51, n(2)?), (35, n(3)?), (10, n(4)?)], line)
            }
            // toggle(x, y, target_group, on)
            "toggle" => {
                want(4)?;
                let on = if a[3].truthy() { 1.0 } else { 0.0 };
                self.trigger(1049, n(0)?, n(1)?, &[(51, n(2)?), (56, on)], line)
            }
            "print" => {
                let s: Vec<String> = a.iter().map(|v| v.to_string()).collect();
                self.logs.push(s.join(" "));
                Ok(Value::Nil)
            }
            "floor" => {
                want(1)?;
                Ok(Value::Num(n(0)?.floor()))
            }
            "ceil" => {
                want(1)?;
                Ok(Value::Num(n(0)?.ceil()))
            }
            "round" => {
                want(1)?;
                Ok(Value::Num(n(0)?.round()))
            }
            "abs" => {
                want(1)?;
                Ok(Value::Num(n(0)?.abs()))
            }
            "sqrt" => {
                want(1)?;
                Ok(Value::Num(n(0)?.sqrt()))
            }
            "sin" => {
                want(1)?;
                Ok(Value::Num(n(0)?.sin()))
            }
            "cos" => {
                want(1)?;
                Ok(Value::Num(n(0)?.cos()))
            }
            "min" => {
                want(2)?;
                Ok(Value::Num(n(0)?.min(n(1)?)))
            }
            "max" => {
                want(2)?;
                Ok(Value::Num(n(0)?.max(n(1)?)))
            }
            "pow" => {
                want(2)?;
                Ok(Value::Num(n(0)?.powf(n(1)?)))
            }
            _ => Err(err(line, format!("unknown function '{name}'"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::run;

    #[test]
    fn single_object() {
        let o = run("obj(1, 15, 15)").unwrap();
        assert_eq!(o.objects_string(), "1,1,2,15,3,15;");
    }

    #[test]
    fn loops_and_math() {
        let o = run("for i in 0..3 { obj(1, i * 30 + 15, 15) }").unwrap();
        assert_eq!(o.objects.len(), 3);
        assert_eq!(
            o.objects_string(),
            "1,1,2,15,3,15;1,1,2,45,3,15;1,1,2,75,3,15;"
        );
    }

    #[test]
    fn step_and_descending() {
        let o = run("for i in 0..10 step 5 { obj(1, i, 0) }").unwrap();
        assert_eq!(o.objects.len(), 2);
        let o = run("for i in 3..0 step -1 { obj(1, i, 0) }").unwrap();
        assert_eq!(o.objects.len(), 3);
    }

    #[test]
    fn functions_and_recursion() {
        let o = run("fn f(n) { if n <= 1 { return 1 } return n * f(n - 1) }\nprint(f(5))").unwrap();
        assert_eq!(o.logs[0], "120");
    }

    #[test]
    fn hoisting() {
        let o = run("print(twice(4))\nfn twice(x) { return x * 2 }").unwrap();
        assert_eq!(o.logs[0], "8");
    }

    #[test]
    fn groups_merge() {
        let o = run("let h = obj(1, 0, 0)\nadd_group(h, 1)\nadd_group(h, 2)").unwrap();
        assert_eq!(o.objects_string(), "1,1,2,0,3,0,57,1.2;");
    }

    #[test]
    fn move_trigger() {
        let o = run("move(100, 15, 3, 0, 30, 1.5)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,901,2,100,3,15,10,1.5,28,0,29,30,36,1,51,3;"
        );
    }

    #[test]
    fn compound_assign_and_globals_in_fns() {
        let o = run("let x = 10\nx += 5\nfn show() { print(x) }\nshow()").unwrap();
        assert_eq!(o.logs[0], "15");
    }

    #[test]
    fn newline_ends_expression() {
        let o = run("let a = 1\n-2\nprint(a)").unwrap();
        assert_eq!(o.logs[0], "1");
    }

    #[test]
    fn runaway_loop_is_stopped() {
        assert!(run("while true { }").is_err());
    }

    #[test]
    fn runaway_recursion_is_stopped() {
        assert!(run("fn f() { return f() }\nf()").is_err());
    }

    #[test]
    fn errors_carry_line_numbers() {
        let e = run("let a = 1\nprint(b)").unwrap_err();
        assert!(e.starts_with("line 2:"), "{e}");
    }
}
