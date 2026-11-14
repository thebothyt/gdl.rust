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
    "obj", "set", "add_group", "group", "color", "trigger", "block", "rect", "set_rotation",
    "set_scale", "flip_x", "flip_y", "set_z", "set_color", "set_detail_color", "print", "floor",
    "ceil", "round", "abs", "sqrt", "sin", "cos", "min", "max", "pow",
];

/// True for anything a script can't redefine: core builtins and every trigger.
pub fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name) || crate::triggers::find(name).is_some()
}

// Property keys, available in scripts as P_<NAME>. Cross-checked against the property
// tables in SPWN (constants.spwn) and the gdlib crate (lookup.rs). Item/counter keys
// (77, 78, 80, 95) are from SPWN's 2.1-era table; verify them on 2.2 with `gdl objects`.
const PROPS: &[(&str, u32)] = &[
    ("OBJ_ID", 1), ("X", 2), ("Y", 3), ("FLIP_X", 4), ("FLIP_Y", 5), ("ROTATION", 6),
    ("RED", 7), ("GREEN", 8), ("BLUE", 9), ("DURATION", 10), ("TOUCH_TRIGGERED", 11),
    ("PLAYER_COLOR_1", 15), ("PLAYER_COLOR_2", 16), ("BLENDING", 17), ("EDITOR_LAYER_1", 20),
    ("COLOR", 21), ("COLOR_2", 22), ("TARGET_COLOR", 23), ("Z_LAYER", 24), ("Z_ORDER", 25),
    ("MOVE_X", 28), ("MOVE_Y", 29), ("EASING", 30), ("TEXT", 31), ("SCALE", 32),
    ("GROUP_PARENT", 34), ("OPACITY", 35), ("ACTIVE_TRIGGER", 36), ("FADE_IN", 45),
    ("HOLD", 46), ("FADE_OUT", 47), ("PULSE_HSV", 48), ("TARGET", 51), ("TARGET_TYPE", 52),
    ("ACTIVATE_GROUP", 56), ("GROUPS", 57), ("LOCK_TO_PLAYER_X", 58), ("LOCK_TO_PLAYER_Y", 59),
    ("COPY_OPACITY", 60), ("EDITOR_LAYER_2", 61), ("SPAWN_TRIGGERED", 62), ("SPAWN_DELAY", 63),
    ("DONT_FADE", 64), ("DONT_ENTER", 67), ("ROTATE_DEGREES", 68), ("TIMES_360", 69),
    ("LOCK_OBJECT_ROTATION", 70), ("CENTER", 71), ("X_MOD", 72), ("Y_MOD", 73), ("STRENGTH", 75),
    ("COUNT", 77), ("SUBTRACT_COUNT", 78), ("ITEM", 80), ("INTERVAL", 84), ("EASING_RATE", 85),
    ("EXCLUSIVE", 86), ("MULTI_TRIGGER", 87), ("ITEM_2", 95), ("HIGH_DETAIL", 103),
    ("SCALE_X", 128), ("SCALE_Y", 129),
];

// Object IDs, available in scripts as ID_<NAME> (from SPWN's constants.spwn).
const IDS: &[(&str, u32)] = &[
    ("MOVE", 901), ("ROTATE", 1346), ("ANIMATE", 1585), ("PULSE", 1006), ("COUNT", 1611),
    ("ALPHA", 1007), ("TOGGLE", 1049), ("FOLLOW", 1347), ("SPAWN", 1268), ("STOP", 1616),
    ("TOUCH", 1595), ("INSTANT_COUNT", 1811), ("ON_DEATH", 1812), ("FOLLOW_PLAYER_Y", 1814),
    ("COLLISION", 1815), ("PICKUP", 1817), ("BG_EFFECT_ON", 1818), ("BG_EFFECT_OFF", 1819),
    ("SHAKE", 1520), ("COLOR", 899), ("ENABLE_TRAIL", 32), ("DISABLE_TRAIL", 33), ("HIDE", 1612),
    ("SHOW", 1613), ("GRAVITY_DOWN", 10), ("GRAVITY_UP", 11), ("CUBE", 12), ("SHIP", 13),
    ("BALL", 47), ("UFO", 111), ("WAVE", 660), ("ROBOT", 745), ("SPIDER", 1331),
    ("MIRROR_ON", 45), ("MIRROR_OFF", 46), ("SIZE_NORMAL", 99), ("SIZE_MINI", 101),
    ("DUAL_ON", 286), ("DUAL_OFF", 287), ("TELEPORT", 747), ("SPEED_YELLOW", 200),
    ("SPEED_BLUE", 201), ("SPEED_GREEN", 202), ("SPEED_PINK", 203), ("SPEED_RED", 1334),
    ("COLLISION_BLOCK", 1816), ("TEXT", 914), ("USER_COIN", 1329), ("ITEM_DISPLAY", 1615),
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
    execute_with(prog, 1, 1)
}

/// Like [`execute`], but `group()` and `color()` start counting at the given ids. Used when a
/// project already contains objects, so new groups don't collide with existing ones.
pub fn execute_with(prog: &[Stmt], first_group: u32, first_color: u32) -> R<RunOutput> {
    let mut it = Interp::new();
    it.next_group = first_group.max(1);
    it.next_color = first_color.max(1);
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

/// Turn positional + named call arguments into one ordered list for a user function.
fn bind_named(fname: &str, params: &[&str], vals: Vec<Value>, named: Vec<(String, Value)>, line: usize) -> R<Vec<Value>> {
    if named.is_empty() {
        return Ok(vals);
    }
    let mut slots: Vec<Option<Value>> = vec![None; params.len()];
    if vals.len() > params.len() {
        return Err(err(line, format!("{fname}() takes {} argument(s), got {}", params.len(), vals.len())));
    }
    for (i, v) in vals.into_iter().enumerate() {
        slots[i] = Some(v);
    }
    for (k, v) in named {
        let i = params
            .iter()
            .position(|p| *p == k)
            .ok_or_else(|| err(line, format!("{fname}() has no parameter '{k}'")))?;
        if slots[i].is_some() {
            return Err(err(line, format!("{fname}(): '{k}' given twice")));
        }
        slots[i] = Some(v);
    }
    slots
        .into_iter()
        .enumerate()
        .map(|(i, s)| s.ok_or_else(|| err(line, format!("{fname}() is missing '{}'", params[i]))))
        .collect()
}

impl Interp {
    fn new() -> Self {
        let mut globals = Scope::new();
        globals.insert("PI".to_string(), Value::Num(std::f64::consts::PI));
        for (name, key) in PROPS {
            globals.insert(format!("P_{name}"), Value::Num(*key as f64));
        }
        for (name, id) in IDS {
            globals.insert(format!("ID_{name}"), Value::Num(*id as f64));
        }
        for (name, v) in crate::triggers::CONSTS {
            globals.insert(name.to_string(), Value::Num(*v));
        }
        for b in crate::blocks::BLOCKS {
            globals.insert(format!("B_{}", b.name.to_uppercase()), Value::Num(b.id as f64));
        }
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
        if is_builtin(&d.name) {
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
            Expr::Named(k, _) => Err(err(
                line,
                format!("'{k} = ...' is only allowed inside a call's argument list"),
            )),
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
        let mut named: Vec<(String, Value)> = Vec::new();
        for a in args {
            if let Expr::Named(k, e) = a {
                if named.iter().any(|(n, _)| n == k) {
                    return Err(err(line, format!("{name}(): '{k}' given twice")));
                }
                let v = self.eval(e, line)?;
                named.push((k.clone(), v));
            } else {
                if !named.is_empty() {
                    return Err(err(line, format!("{name}(): positional arguments can't follow named ones")));
                }
                vals.push(self.eval(a, line)?);
            }
        }
        if let Some(f) = self.funcs.get(name).cloned() {
            let vals = bind_named(&f.name, &f.params.iter().map(|p| p.as_str()).collect::<Vec<_>>(), vals, named, line)?;
            return self.call_user(&f, vals, line);
        }
        if let Some(def) = crate::triggers::find(name) {
            return self.call_def(def, vals, named, line);
        }
        if !named.is_empty() {
            return Err(err(line, format!("{name}() doesn't take named arguments")));
        }
        self.builtin(name, vals, line)
    }

    /// Build a trigger object from its table entry: x, y, then the table's arguments, filled
    /// by position and/or by name.
    fn call_def(
        &mut self,
        def: &'static crate::triggers::Def,
        vals: Vec<Value>,
        named: Vec<(String, Value)>,
        line: usize,
    ) -> R<Value> {
        use crate::triggers::{COMMON, K};
        let mut slots: Vec<(&str, Option<Value>)> = vec![("x", None), ("y", None)];
        for a in def.args {
            slots.push((a.name, None));
        }
        let n_pos = slots.len();
        if vals.len() > n_pos {
            return Err(err(line, format!("{}() takes at most {} arguments, got {}", def.name, n_pos, vals.len())));
        }
        for (i, v) in vals.into_iter().enumerate() {
            slots[i].1 = Some(v);
        }
        let mut common: Vec<(&crate::triggers::Arg, Value)> = Vec::new();
        for (k, v) in named {
            if let Some(slot) = slots.iter_mut().find(|(n, _)| *n == k) {
                if slot.1.is_some() {
                    return Err(err(line, format!("{}(): '{k}' given twice", def.name)));
                }
                slot.1 = Some(v);
            } else if let Some(c) = COMMON.iter().find(|c| c.name == k) {
                common.push((c, v));
            } else {
                let valid: Vec<&str> = slots.iter().map(|(n, _)| *n).chain(COMMON.iter().map(|c| c.name)).collect();
                return Err(err(line, format!("{}() has no argument '{k}' (valid: {})", def.name, valid.join(", "))));
            }
        }
        let num = |v: &Option<Value>, what: &str| -> R<f64> {
            match v {
                Some(Value::Num(n)) => Ok(*n),
                _ => Err(err(line, format!("{}(): '{what}' must be a number", def.name))),
            }
        };
        let x = num(&slots[0].1, "x").map_err(|_| err(line, format!("{}() needs x and y positions", def.name)))?;
        let y = num(&slots[1].1, "y").map_err(|_| err(line, format!("{}() needs x and y positions", def.name)))?;
        let mut props: Vec<(u32, String)> = vec![
            (1, def.id.to_string()),
            (2, fmt_num(x)),
            (3, fmt_num(y)),
            (36, "1".to_string()),
        ];
        for (k, v) in def.fixed {
            props.push((*k, v.to_string()));
        }
        let convert = |a: &crate::triggers::Arg, v: &Value| -> R<String> {
            match (a.kind, v) {
                (K::Num, Value::Num(n)) => Ok(fmt_num(*n)),
                (K::Num, Value::Bool(b)) => Ok(if *b { "1" } else { "0" }.to_string()),
                (K::Bool, v) => Ok(if v.truthy() { "1" } else { "0" }.to_string()),
                (K::Str, Value::Str(s)) => Ok(s.clone()),
                (K::Str, Value::Num(n)) => Ok(fmt_num(*n)),
                _ => Err(err(line, format!("{}(): '{}' expects {}, got {v}", def.name, a.name,
                    match a.kind { K::Num => "a number", K::Bool => "true/false", K::Str => "text" }))),
            }
        };
        for (i, a) in def.args.iter().enumerate() {
            match &slots[i + 2].1 {
                Some(v) => {
                    if let Some((k, val)) = a.also {
                        props.push((k, val.to_string()));
                    }
                    props.push((a.key, convert(a, v)?));
                }
                None if !a.optional => {
                    return Err(err(line, format!("{}() is missing '{}' ({})", def.name, a.name, a.doc)));
                }
                None => {}
            }
        }
        for (c, v) in &common {
            props.push((c.key, convert(c, v)?));
        }
        Ok(Value::Num(self.push_obj(&props, line)?))
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
        o.line = line;
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

    fn set_prop(&mut self, h: f64, key: u32, val: String, name: &str, line: usize) -> R<Value> {
        let h = self.handle(h, name, line)?;
        self.objects[h].props.insert(key, val);
        Ok(Value::Nil)
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
            // trigger(id, x, y): any trigger by object id; fill in its properties with set()
            "trigger" => {
                want(3)?;
                let id = n(0)?;
                if id < 1.0 || id.fract() != 0.0 {
                    return Err(err(line, "trigger(): the object id must be a whole number"));
                }
                self.trigger(id as u32, n(1)?, n(2)?, &[], line)
            }
            // block("spike", x, y): a catalog block by name (see `gdl blocks`)
            "block" => {
                want(3)?;
                let bname = match &a[0] {
                    Value::Str(s) => s.clone(),
                    _ => return Err(err(line, "block(): the first argument must be a name in quotes, e.g. block(\"spike\", 0, 0)")),
                };
                let blk = crate::blocks::find(&bname).ok_or_else(|| {
                    err(line, format!("block(): unknown block '{bname}' (run `gdl blocks` for the list)"))
                })?;
                let props = [(1, blk.id.to_string()), (2, fmt_num(n(1)?)), (3, fmt_num(n(2)?))];
                Ok(Value::Num(self.push_obj(&props, line)?))
            }
            // rect(id, x, y, w, h): fill a w-by-h grid (30 units apart); returns how many were placed
            "rect" => {
                want(5)?;
                let (id, x, y, w, h) = (n(0)?, n(1)?, n(2)?, n(3)?, n(4)?);
                if id < 1.0 || id.fract() != 0.0 || w < 0.0 || h < 0.0 || w.fract() != 0.0 || h.fract() != 0.0 {
                    return Err(err(line, "rect(): id, w and h must be whole numbers (w and h >= 0)"));
                }
                if w * h > MAX_OBJECTS as f64 {
                    return Err(err(line, format!("too many objects (limit {MAX_OBJECTS})")));
                }
                for j in 0..h as u32 {
                    for i in 0..w as u32 {
                        self.tick(line)?;
                        let props = [
                            (1, fmt_num(id)),
                            (2, fmt_num(x + i as f64 * 30.0)),
                            (3, fmt_num(y + j as f64 * 30.0)),
                        ];
                        self.push_obj(&props, line)?;
                    }
                }
                Ok(Value::Num(w * h))
            }
            "set_rotation" => {
                want(2)?;
                self.set_prop(n(0)?, 6, fmt_num(n(1)?), name, line)
            }
            "set_scale" => {
                want(2)?;
                self.set_prop(n(0)?, 32, fmt_num(n(1)?), name, line)
            }
            "flip_x" => {
                want(1)?;
                self.set_prop(n(0)?, 4, "1".into(), name, line)
            }
            "flip_y" => {
                want(1)?;
                self.set_prop(n(0)?, 5, "1".into(), name, line)
            }
            // set_z(handle, layer, order)
            "set_z" => {
                want(3)?;
                self.set_prop(n(0)?, 24, fmt_num(n(1)?), name, line)?;
                self.set_prop(n(0)?, 25, fmt_num(n(2)?), name, line)
            }
            "set_color" => {
                want(2)?;
                self.set_prop(n(0)?, 21, fmt_num(n(1)?), name, line)
            }
            "set_detail_color" => {
                want(2)?;
                self.set_prop(n(0)?, 22, fmt_num(n(1)?), name, line)
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

    #[test]
    fn more_triggers() {
        let o = run("spawn(100, 15, 2, 0.5)\nrotate(130, 15, 2, 3, 90, 1)\nstop(160, 15, 2)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,1268,2,100,3,15,36,1,51,2,63,0.5;\
             1,1346,2,130,3,15,10,1,36,1,51,2,68,90,71,3;\
             1,1616,2,160,3,15,36,1,51,2;"
        );
        let o = run("pulse(0, 0, 4, 255, 0, 0, 0.1, 0.2, 0.3)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,1006,2,0,3,0,7,255,8,0,9,0,36,1,45,0.1,46,0.2,47,0.3,51,4;"
        );
        let o = run("color_trigger(0, 0, 5, 10, 20, 30, 2)\nshake(0, 0, 3, 0, 1)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,899,2,0,3,0,7,10,8,20,9,30,10,2,23,5,36,1;\
             1,1520,2,0,3,0,10,1,36,1,75,3,84,0;"
        );
    }

    #[test]
    fn generic_trigger_and_constants() {
        let o = run("let h = trigger(ID_COUNT, 0, 0)\nset(h, P_ITEM, 5)\nprint(P_TARGET, ID_MOVE)").unwrap();
        assert_eq!(o.objects_string(), "1,1611,2,0,3,0,36,1,80,5;");
        assert_eq!(o.logs[0], "51 901");
    }

    #[test]
    fn objects_remember_their_script_line() {
        let o = run("\nobj(1, 0, 0)\n\nobj(1, 30, 0)").unwrap();
        assert_eq!(o.objects[0].line, 2);
        assert_eq!(o.objects[1].line, 4);
        let j = o.to_json();
        assert!(j.contains("\"lines\":[2,4]"), "{j}");
        assert!(j.starts_with("{\"ok\":true"), "{j}");
    }

    #[test]
    fn preview_json_reports_errors() {
        let j = crate::preview_json("obj(1, 0)");
        assert!(j.starts_with("{\"ok\":false,\"error\":\"line 1:"), "{j}");
    }

    // ---- 2.2 triggers, named arguments, blocks ----

    #[test]
    fn pickup_with_named_option() {
        let o = run("pickup(300, 15, 1, 2, mode = PICKUP_MULTIPLY)").unwrap();
        assert_eq!(o.objects_string(), "1,1817,2,300,3,15,36,1,77,2,80,1,88,1,449,1;");
    }

    #[test]
    fn item_edit_adds_item_type_and_defaults() {
        let o = run("item_edit(0, 0, 5, OP_ADD, 1, item1 = 2)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,3619,2,0,3,0,36,1,51,5,80,2,476,1,478,1,479,1,480,1,481,1,482,3;"
        );
    }

    #[test]
    fn item_compare_sets_both_branches() {
        let o = run("item_compare(0, 0, 1, 2, CMP_GTE, 10, 11)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,3620,2,0,3,0,36,1,51,10,71,11,80,1,95,2,476,1,477,1,479,1,480,3,481,3,482,2,483,1;"
        );
    }

    #[test]
    fn gradient_trigger() {
        let o = run("gradient(0, 0, 1, 2, 3, 4, 5)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,2903,2,0,3,0,36,1,202,1,203,2,204,3,205,4,206,5,209,1,456,1;"
        );
    }

    #[test]
    fn easing_and_common_options() {
        let o = run("move(0, 0, 1, 0, 30, 2, easing = EASE_IN_OUT, multi_triggered = true)").unwrap();
        let s = o.objects_string();
        assert!(s.contains(",30,1,"), "{s}");
        assert!(s.contains(",87,1"), "{s}");
    }

    #[test]
    fn toggle_and_bool_args() {
        let o = run("toggle(0, 0, 4, false)\ntoggle(30, 0, 4, true)").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,1049,2,0,3,0,36,1,51,4,56,0;1,1049,2,30,3,0,36,1,51,4,56,1;"
        );
    }

    #[test]
    fn named_args_for_user_functions() {
        let o = run("fn f(a, b) { print(a - b) }\nf(b = 1, a = 5)\nf(9, b = 2)").unwrap();
        assert_eq!(o.logs, vec!["4", "7"]);
    }

    #[test]
    fn named_argument_errors_are_clear() {
        let e = run("pickup(0, 0, 1)").unwrap_err();
        assert!(e.contains("missing 'count'"), "{e}");
        let e = run("pickup(0, 0, 1, 2, nope = 1)").unwrap_err();
        assert!(e.contains("no argument 'nope'") && e.contains("valid:"), "{e}");
        let e = run("pickup(0, 0, item = 1, 2)").unwrap_err();
        assert!(e.contains("can't follow"), "{e}");
        let e = run("pickup(0, 0, 1, 2, count = 3)").unwrap_err();
        assert!(e.contains("given twice"), "{e}");
        let e = run("let a = 1\nprint(a, b = 2)").unwrap_err();
        assert!(e.contains("named arguments"), "{e}");
        let e = run("fn pickup() { }").unwrap_err();
        assert!(e.contains("builtin"), "{e}");
    }

    #[test]
    fn blocks_by_name_constant_and_rect() {
        let o = run("block(\"spike\", 15, 15)\nobj(B_SLAB, 45, 15)").unwrap();
        assert_eq!(o.objects_string(), "1,8,2,15,3,15;1,40,2,45,3,15;");
        let o = run("print(rect(B_BASIC, 15, 15, 2, 2))").unwrap();
        assert_eq!(
            o.objects_string(),
            "1,1,2,15,3,15;1,1,2,45,3,15;1,1,2,15,3,45;1,1,2,45,3,45;"
        );
        assert_eq!(o.logs, vec!["4"]);
        assert!(run("block(\"nope\", 0, 0)").unwrap_err().contains("gdl blocks"));
        assert!(run("rect(1, 0, 0, 1000, 1000)").is_err());
    }

    #[test]
    fn object_helpers() {
        let o = run("let b = block(\"basic\", 0, 0)\nset_rotation(b, 90)\nset_scale(b, 0.5)\nflip_x(b)\nset_z(b, 3, 2)\nset_color(b, 5)").unwrap();
        assert_eq!(o.objects_string(), "1,1,2,0,3,0,4,1,6,90,21,5,24,3,25,2,32,0.5;");
    }

    #[test]
    fn every_trigger_in_the_table_can_be_called() {
        // Calls each trigger with just its required arguments and checks the object id.
        for d in crate::triggers::TRIGGERS {
            let mut args = vec!["0".to_string(), "0".to_string()];
            for a in d.args.iter().filter(|a| !a.optional) {
                args.push(if a.kind == crate::triggers::K::Str { "\"1.2\"".into() } else { "1".into() });
            }
            let src = format!("{}({})", d.name, args.join(", "));
            let o = run(&src).unwrap_or_else(|e| panic!("{src}: {e}"));
            assert!(o.objects_string().starts_with(&format!("1,{},2,0,3,0,", d.id)), "{src}");
        }
    }
}
