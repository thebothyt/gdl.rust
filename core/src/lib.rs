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

/// Run a script and return the objects it created plus anything it printed.
pub fn run(source: &str) -> Result<RunOutput, String> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    interp::execute(&program)
}
