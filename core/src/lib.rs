//! gdl_core: a tiny scripting language that generates Geometry Dash level objects.
//!
//! Pipeline: source -> lexer -> parser (AST) -> interpreter -> objects -> level string.
//! `codec` handles the file side (.gdlvl: base64 / gzip / XML patching).

pub mod ast;
pub mod codec;
pub mod interp;
pub mod lexer;
pub mod level;
pub mod parser;

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
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    interp::execute(&program)
}
