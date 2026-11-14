//! The `.gdlrust` project format.
//!
//! A `.gdlrust` file is one plain UTF-8 text file that holds everything about a project:
//! your script, the level objects that were already in the level, the level header, and the
//! level's XML wrapper. Nothing else. It never contains cookies, browser storage, save-game
//! data or credentials, and you can read, diff and commit it like source code.
//!
//! ```text
//! #gdlrust 1
//! name: My Level
//! created_with: gdl 0.2.0
//! wrap: base64            outer layer of the original .gdlvl (xml | gzip | base64 | base64-urlsafe)
//! k4: compressed          whether the level string was gzip+base64 (compressed | raw)
//! header: kS38,1_40_...   level header, exactly as stored in the level
//! %%% script
//! ...your gdl script...
//! %%% objects
//! 1,1,2,15,3,15           one imported object record per line
//! %%% template
//! <?xml ...><s>{{LEVEL}}</s>...   the level's XML with the level string replaced by {{LEVEL}}
//! ```
//!
//! Section markers start with `%%%`. That can never begin a valid script line (`%` is an
//! operator), so scripts can't collide with them.
//!
//! Compiling runs the script and writes `header + imported objects + script objects` back
//! into the template, re-applying the original wrapping.

use crate::codec::{self, Wrap};

pub const FORMAT_VERSION: u32 = 1;
const MARKER: &str = "%%% ";
const PLACEHOLDER: &str = "{{LEVEL}}";
const TOOL: &str = concat!("gdl ", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    /// Outer layer of the source level: "xml", "gzip", "base64" or "base64-urlsafe".
    pub wrap: String,
    /// Whether the level string inside the XML is gzip+base64 (true) or plain text (false).
    pub compressed: bool,
    pub header: String,
    /// Objects that were already in the level, one raw record each (no trailing `;`).
    pub objects: Vec<String>,
    pub script: String,
    /// The level's XML with the level string replaced by `{{LEVEL}}`. None for a project that
    /// didn't start from a level file.
    pub template: Option<String>,
}

#[derive(Debug)]
pub struct Compiled {
    pub bytes: Vec<u8>,
    /// Objects created by the script (not counting imported ones).
    pub script_objects: usize,
    pub logs: Vec<String>,
}

const STARTER: &str = "// Your gdl script. Run `gdl run` to see what it makes, or open the web editor.\n\nlet g = group()\nfor i in 0..5 {\n  let b = block(\"basic\", 15 + i * 30, 15)\n  add_group(b, g)\n}\nmove(300, 15, g, 0, 60, 1, easing = EASE_IN_OUT)\n";

impl Project {
    /// An empty project with a starter script. It has no level template, so compile it with
    /// `--into <level>` to add its objects to an existing level.
    pub fn new(name: &str) -> Project {
        Project {
            name: clean_line(name),
            wrap: "base64".to_string(),
            compressed: true,
            header: String::new(),
            objects: Vec::new(),
            script: STARTER.to_string(),
            template: None,
        }
    }

    /// Convert a `.gdlvl` file into a project. `name` is used if the level has no name.
    pub fn from_level(file: &[u8], name: &str) -> Result<Project, String> {
        let (wrap, text) = codec::unwrap_file(file)?;
        let (a, b) = codec::find_k4(&text).ok_or_else(|| {
            "couldn't find the level string (<k>k4</k>) in this file; run `inspect` to check its layout".to_string()
        })?;
        let k = codec::read_k4(&text[a..b])?;
        let (header, rest) = match k.raw.split_once(';') {
            Some((h, r)) => (h.to_string(), r.to_string()),
            None => (k.raw.clone(), String::new()),
        };
        let objects: Vec<String> = rest
            .split(';')
            .map(|r| r.trim())
            .filter(|r| !r.is_empty())
            .map(|r| r.to_string())
            .collect();
        let mut template = String::with_capacity(text.len());
        template.push_str(&text[..a]);
        template.push_str(PLACEHOLDER);
        template.push_str(&text[b..]);
        let level_name = xml_string_value(&text, "k2").unwrap_or_else(|| name.to_string());
        Ok(Project {
            name: clean_line(&level_name),
            wrap: wrap.name().to_string(),
            compressed: k.compressed,
            header,
            objects,
            script: "// Objects already in the level are kept as they are.\n// Anything this script makes is added on top when you compile.\n\n".to_string(),
            template: Some(template),
        })
    }

    pub fn parse(text: &str) -> Result<Project, String> {
        let mut lines = text.lines();
        let first = lines.next().unwrap_or("").trim_start_matches('\u{feff}');
        let version: u32 = first
            .strip_prefix("#gdlrust")
            .and_then(|v| v.trim().parse().ok())
            .ok_or("not a .gdlrust file (the first line should be `#gdlrust 1`)")?;
        if version > FORMAT_VERSION {
            return Err(format!(
                ".gdlrust format version {version} is newer than this tool understands ({FORMAT_VERSION}); update gdl"
            ));
        }
        let mut p = Project {
            name: String::new(),
            wrap: "base64".to_string(),
            compressed: true,
            header: String::new(),
            objects: Vec::new(),
            script: String::new(),
            template: None,
        };
        let mut section: Option<String> = None;
        let mut bodies: Vec<(String, Vec<&str>)> = Vec::new();
        for (i, line) in lines.enumerate() {
            if let Some(name) = line.strip_prefix(MARKER) {
                let name = name.trim().to_string();
                if !["script", "objects", "template"].contains(&name.as_str()) {
                    return Err(format!("line {}: unknown section '{name}'", i + 2));
                }
                section = Some(name.clone());
                bodies.push((name, Vec::new()));
            } else if section.is_some() {
                bodies.last_mut().unwrap().1.push(line);
            } else if line.trim().is_empty() || line.starts_with('#') {
                continue;
            } else if let Some((k, v)) = line.split_once(':') {
                let v = v.trim();
                match k.trim() {
                    "name" => p.name = v.to_string(),
                    "wrap" => {
                        Wrap::from_name(v).ok_or_else(|| format!("line {}: unknown wrap '{v}'", i + 2))?;
                        p.wrap = v.to_string();
                    }
                    "k4" => {
                        p.compressed = match v {
                            "compressed" => true,
                            "raw" => false,
                            _ => return Err(format!("line {}: k4 must be 'compressed' or 'raw'", i + 2)),
                        }
                    }
                    "header" => p.header = v.to_string(),
                    _ => {} // created_with and unknown keys are informational
                }
            } else {
                return Err(format!("line {}: expected `key: value`, got {:?}", i + 2, line));
            }
        }
        for (name, body) in bodies {
            let joined = body.join("\n");
            match name.as_str() {
                "script" => p.script = joined,
                "objects" => {
                    p.objects = body
                        .iter()
                        .map(|l| l.trim().trim_end_matches(';').to_string())
                        .filter(|l| !l.is_empty())
                        .collect()
                }
                _ => {
                    if !joined.contains(PLACEHOLDER) {
                        return Err(format!("the template section must contain {PLACEHOLDER}"));
                    }
                    p.template = Some(joined);
                }
            }
        }
        Ok(p)
    }

    pub fn to_text(&self) -> Result<String, String> {
        for (what, body) in [("script", Some(&self.script)), ("template", self.template.as_ref())] {
            if let Some(b) = body {
                if b.lines().any(|l| l.starts_with("%%%")) {
                    return Err(format!("a line in the {what} starts with %%%, which is reserved for section markers"));
                }
            }
        }
        let mut s = format!("#gdlrust {FORMAT_VERSION}\n");
        s.push_str(&format!("name: {}\n", clean_line(&self.name)));
        s.push_str(&format!("created_with: {TOOL}\n"));
        s.push_str(&format!("wrap: {}\n", self.wrap));
        s.push_str(&format!("k4: {}\n", if self.compressed { "compressed" } else { "raw" }));
        s.push_str(&format!("header: {}\n", clean_line(&self.header)));
        s.push_str("%%% script\n");
        s.push_str(self.script.trim_end_matches('\n'));
        s.push('\n');
        s.push_str("%%% objects\n");
        for o in &self.objects {
            s.push_str(o);
            s.push('\n');
        }
        if let Some(t) = &self.template {
            s.push_str("%%% template\n");
            s.push_str(t.trim_end_matches('\n'));
            s.push('\n');
        }
        Ok(s)
    }

    /// Run the script and write the level back out. Needs a template (a project made from a
    /// level); otherwise use [`Project::compile_into`].
    pub fn compile(&self) -> Result<Compiled, String> {
        let template = self.template.as_ref().ok_or(
            "this project has no level template (it didn't start from a .gdlvl); compile it into an existing level with `--into <level.gdlvl>`",
        )?;
        let out = self.run_script()?;
        let mut raw = self.header.clone();
        raw.push(';');
        for o in &self.objects {
            raw.push_str(o);
            raw.push(';');
        }
        raw.push_str(&out.objects_string());
        let k4 = if self.compressed {
            codec::b64_encode(&codec::gzip_stored(raw.as_bytes()), true)
        } else {
            raw
        };
        let text = template.replacen(PLACEHOLDER, &k4, 1);
        let wrap = Wrap::from_name(&self.wrap).ok_or("unknown wrap")?;
        Ok(Compiled {
            bytes: codec::rewrap(&wrap, &text),
            script_objects: out.objects.len(),
            logs: out.logs,
        })
    }

    /// Run the script and append its objects to an existing `.gdlvl` file.
    pub fn compile_into(&self, level: &[u8]) -> Result<Compiled, String> {
        let out = self.run_script()?;
        Ok(Compiled {
            bytes: codec::patch_level_file(level, &out.objects_string())?,
            script_objects: out.objects.len(),
            logs: out.logs,
        })
    }

    fn run_script(&self) -> Result<crate::RunOutput, String> {
        let (g, c) = self.next_free_ids();
        crate::run_with(&self.script, g, c)
    }

    /// What the web editor does on Save/Compile: take the project text the page is holding
    /// (empty if none), replace its script with what is in the editor, and, for a brand-new
    /// project, start from the uploaded level if there is one.
    pub fn for_editor(project: &str, script: &str, name: &str, level: Option<&[u8]>) -> Result<Project, String> {
        let mut p = if project.trim().is_empty() {
            match level {
                Some(l) => Project::from_level(l, name)?,
                None => Project::new(name),
            }
        } else {
            Project::parse(project)?
        };
        p.script = script.to_string();
        Ok(p)
    }

    /// Compile; a project without a level template is compiled into `level` when one is given.
    pub fn compile_for_editor(&self, level: Option<&[u8]>) -> Result<Compiled, String> {
        match (&self.template, level) {
            (None, Some(l)) => self.compile_into(l),
            _ => self.compile(),
        }
    }

    /// First group and colour ids not used by the imported objects.
    pub fn next_free_ids(&self) -> (u32, u32) {
        let (mut max_g, mut max_c) = (0u32, 0u32);
        for rec in &self.objects {
            let parts: Vec<&str> = rec.split(',').collect();
            for kv in parts.chunks(2) {
                if kv.len() < 2 {
                    continue;
                }
                match kv[0] {
                    "57" => {
                        for g in kv[1].split('.') {
                            if let Ok(n) = g.parse::<u32>() {
                                max_g = max_g.max(n);
                            }
                        }
                    }
                    "51" | "71" => {
                        if let Ok(n) = kv[1].parse::<u32>() {
                            if n < 10_000 {
                                max_g = max_g.max(n);
                            }
                        }
                    }
                    "21" | "22" | "23" => {
                        if let Ok(n) = kv[1].parse::<u32>() {
                            if n < 1000 {
                                max_c = max_c.max(n);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        (max_g + 1, max_c + 1)
    }

    /// The imported objects as a level-string fragment (`rec;rec;`), for the preview.
    pub fn objects_string(&self) -> String {
        let mut s = String::new();
        for o in &self.objects {
            s.push_str(o);
            s.push(';');
        }
        s
    }
}

fn clean_line(s: &str) -> String {
    s.replace(['\r', '\n'], " ").trim().to_string()
}

/// Value of `<k>key</k><s>value</s>` in a level's XML, if present and plain text.
fn xml_string_value(xml: &str, key: &str) -> Option<String> {
    let tag = format!("<k>{key}</k>");
    let after = xml.find(&tag)? + tag.len();
    let rest = xml[after..].trim_start();
    let body = rest.strip_prefix("<s>")?;
    let end = body.find("</s>")?;
    let v = body[..end].trim();
    if v.is_empty() || v.len() > 200 {
        None
    } else {
        Some(v.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_file(wrap: u8, compressed_k4: bool) -> Vec<u8> {
        let level = "kS38,1_40_2_125;1,1,2,15,3,15,57,4;1,8,2,45,3,15;";
        let k4 = if compressed_k4 {
            codec::b64_encode(&codec::gzip_stored(level.as_bytes()), true)
        } else {
            level.to_string()
        };
        let xml = format!(
            "<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict><k>k2</k><s>My Level</s><k>k4</k><s>{k4}</s></dict></plist>"
        );
        match wrap {
            0 => xml.into_bytes(),
            1 => codec::gzip_stored(xml.as_bytes()),
            _ => codec::b64_encode(&codec::gzip_stored(xml.as_bytes()), false).into_bytes(),
        }
    }

    #[test]
    fn level_to_project_and_back_for_every_wrapping() {
        for wrap in 0..3 {
            for comp in [false, true] {
                let file = fake_file(wrap, comp);
                let mut p = Project::from_level(&file, "fallback").unwrap();
                assert_eq!(p.name, "My Level");
                assert_eq!(p.header, "kS38,1_40_2_125");
                assert_eq!(p.objects.len(), 2);
                assert_eq!(p.compressed, comp);

                // survive a trip through the text format
                p.script = "obj(1, 75, 15)\nlet g = group()\nmove(0, 0, g, 0, 30, 1)\nprint(g)".to_string();
                let text = p.to_text().unwrap();
                let p2 = Project::parse(&text).unwrap();
                assert_eq!(p, p2, "wrap={wrap} comp={comp}");

                // compile and check the resulting level
                let out = p2.compile().unwrap();
                assert_eq!(out.script_objects, 2);
                // group 4 is used by an imported object, so the script's group() must be 5
                assert_eq!(out.logs, vec!["5".to_string()]);
                let (header, objs) = codec::level_objects(&out.bytes).unwrap();
                assert_eq!(header, "kS38,1_40_2_125");
                assert!(objs.starts_with("1,1,2,15,3,15,57,4;1,8,2,45,3,15;1,1,2,75,3,15;"), "{objs}");
                assert!(objs.contains("1,901,"), "{objs}");
            }
        }
    }

    #[test]
    fn converting_without_a_script_is_lossless() {
        let file = fake_file(2, true);
        let p = Project::from_level(&file, "x").unwrap();
        let out = p.compile().unwrap();
        assert_eq!(out.script_objects, 0);
        let (h1, o1) = codec::level_objects(&file).unwrap();
        let (h2, o2) = codec::level_objects(&out.bytes).unwrap();
        assert_eq!((h1, o1), (h2, o2));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Project::parse("hello").is_err());
        assert!(Project::parse("#gdlrust 99\n").unwrap_err().contains("newer"));
        assert!(Project::parse("#gdlrust 1\nwrap: bogus\n").is_err());
        assert!(Project::parse("#gdlrust 1\n%%% template\n<x/>\n").is_err());
        let mut p = Project::new("t");
        p.script = "%%% script".into();
        assert!(p.to_text().is_err());
    }

    #[test]
    fn new_project_needs_a_level_to_compile() {
        let p = Project::new("fresh");
        assert!(p.compile().unwrap_err().contains("--into"));
        let out = p.compile_into(&fake_file(0, true)).unwrap();
        assert_eq!(out.script_objects, 6); // 5 blocks + 1 move trigger
    }

    #[test]
    fn editor_flow_new_then_level_then_resave() {
        let level = fake_file(2, true);
        // nothing open, level uploaded: the project is built from the level
        let p = Project::for_editor("", "obj(1, 0, 0)", "x", Some(&level)).unwrap();
        assert_eq!(p.objects.len(), 2);
        let text = p.to_text().unwrap();
        // re-saving replaces only the script
        let p2 = Project::for_editor(&text, "obj(1, 5, 5)\nobj(1, 6, 6)", "x", None).unwrap();
        assert_eq!(p2.objects, p.objects);
        assert_eq!(p2.compile_for_editor(None).unwrap().script_objects, 2);
        // nothing open, no level: a fresh project that needs a level to compile
        let fresh = Project::for_editor("", "obj(1, 0, 0)", "x", None).unwrap();
        assert!(fresh.compile_for_editor(None).is_err());
        assert_eq!(fresh.compile_for_editor(Some(&level)).unwrap().script_objects, 1);
        assert!(Project::for_editor("garbage", "", "x", None).is_err());
    }

    #[test]
    fn project_text_has_no_binary_or_state() {
        let p = Project::from_level(&fake_file(2, true), "x").unwrap();
        let text = p.to_text().unwrap();
        assert!(text.starts_with("#gdlrust 1\n"));
        assert!(text.is_ascii());
    }
}
