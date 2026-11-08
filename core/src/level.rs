use std::collections::BTreeMap;
use std::fmt;

/// One level object: GD property key -> value text. Keys print in ascending order.
#[derive(Debug, Clone, Default)]
pub struct Obj {
    pub props: BTreeMap<u32, String>,
    /// Script line that created this object (0 if unknown). Not written to the level.
    pub line: usize,
}

impl fmt::Display for Obj {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for (k, v) in &self.props {
            if !first {
                f.write_str(",")?;
            }
            first = false;
            write!(f, "{k},{v}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RunOutput {
    pub objects: Vec<Obj>,
    pub logs: Vec<String>,
}

impl RunOutput {
    /// Objects in level-string form: `1,1,2,15,3,15;1,8,2,45,3,15;`
    pub fn objects_string(&self) -> String {
        let mut s = String::new();
        for o in &self.objects {
            s.push_str(&o.to_string());
            s.push(';');
        }
        s
    }

    /// JSON for the visual preview: the object string, the script line of each object,
    /// and anything the script printed.
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\"ok\":true,\"logs\":[");
        for (i, l) in self.logs.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&json_escape(l));
        }
        s.push_str("],\"lines\":[");
        for (i, o) in self.objects.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&o.line.to_string());
        }
        s.push_str("],\"objects\":");
        s.push_str(&json_escape(&self.objects_string()));
        s.push('}');
        s
    }
}

pub fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn error_json(msg: &str) -> String {
    format!("{{\"ok\":false,\"error\":{}}}", json_escape(msg))
}

/// Whole numbers print without a decimal point; others get up to 4 decimals.
pub fn fmt_num(x: f64) -> String {
    if !x.is_finite() {
        return "0".to_string();
    }
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        let s = format!("{:.4}", x);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
