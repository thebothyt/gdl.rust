use std::collections::BTreeMap;
use std::fmt;

/// One level object: GD property key -> value text. Keys print in ascending order.
#[derive(Debug, Clone, Default)]
pub struct Obj {
    pub props: BTreeMap<u32, String>,
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
