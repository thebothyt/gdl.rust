//! gdl_core: a tiny scripting language that generates Geometry Dash level objects.
//!
//! Pipeline: source -> lexer -> parser (AST) -> interpreter -> objects -> level string.
//! `codec` handles the file side (.gdlvl: base64 / gzip / XML patching).

pub mod ast;
pub mod blocks;
pub mod codec;
pub mod interp;
pub mod lexer;
pub mod level;
pub mod parser;
pub mod project;
pub mod triggers;

pub use level::{Obj, RunOutput};

/// Run a script and return JSON for the visual preview (never fails; errors are JSON too).
pub fn preview_json(source: &str) -> String {
    match run(source) {
        Ok(o) => o.to_json(),
        Err(e) => level::error_json(&e),
    }
}

/// JSON describing the objects already in a .gdlvl file, for drawing under the script's objects.
pub fn base_json(file: &[u8]) -> String {
    match codec::level_objects(file) {
        Ok((header, objects)) => format!(
            "{{\"ok\":true,\"header\":{},\"objects\":{}}}",
            level::json_escape(&header),
            level::json_escape(&objects)
        ),
        Err(e) => level::error_json(&e),
    }
}

/// Run a script and return the objects it created plus anything it printed.
pub fn run(source: &str) -> Result<RunOutput, String> {
    run_with(source, 1, 1)
}

/// Like [`run`], but `group()` / `color()` start at the given ids (so a project that already
/// contains objects doesn't reuse their groups).
pub fn run_with(source: &str, first_group: u32, first_color: u32) -> Result<RunOutput, String> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    interp::execute_with(&program, first_group, first_color)
}

/// Convert a `.gdlvl` file into `.gdlrust` text.
pub fn level_to_project(file: &[u8], name: &str) -> Result<String, String> {
    project::Project::from_level(file, name)?.to_text()
}

/// Compile `.gdlrust` text back into `.gdlvl` bytes.
pub fn compile_project(text: &str) -> Result<Vec<u8>, String> {
    Ok(project::Project::parse(text)?.compile()?.bytes)
}

/// Open a `.gdlrust` for the editor: its script, header and the objects already in the level.
/// Always returns JSON: {"ok":true,"name","script","header","objects"} or {"ok":false,"error"}.
pub fn project_open_json(text: &str) -> String {
    match project::Project::parse(text) {
        Ok(p) => format!(
            "{{\"ok\":true,\"name\":{},\"script\":{},\"header\":{},\"objects\":{},\"has_template\":{}}}",
            level::json_escape(&p.name),
            level::json_escape(&p.script),
            level::json_escape(&p.header),
            level::json_escape(&p.objects_string()),
            p.template.is_some()
        ),
        Err(e) => level::error_json(&e),
    }
}

#[cfg(test)]
mod example_tests {
    /// Every script in examples/ must keep running, so the docs never show broken code.
    #[test]
    fn all_examples_run() {
        for (name, src) in [
            ("demo.gdl", include_str!("../../examples/demo.gdl")),
            ("blocks.gdl", include_str!("../../examples/blocks.gdl")),
            ("triggers.gdl", include_str!("../../examples/triggers.gdl")),
        ] {
            let out = crate::run(src).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!out.objects.is_empty(), "{name} made no objects");
        }
    }

    #[test]
    fn sample_level_round_trips_through_gdlrust() {
        let level = include_bytes!("../../examples/sample.gdlvl");
        let text = crate::level_to_project(level, "sample").unwrap();
        assert!(text.contains("name: Sample Level"));
        let back = crate::compile_project(&text).unwrap();
        assert_eq!(
            crate::codec::level_objects(level).unwrap(),
            crate::codec::level_objects(&back).unwrap()
        );
    }
}
