mod codegen;
mod eval;
mod lexer;
mod parser;
mod reporting;
mod semantic;
mod symbol;

pub use codegen::CodeGenerationError;
use exalt_assembler::CodeGenTextData;
use exalt_lir::Game;
use itertools::Itertools;
pub use lexer::{Peekable, Token};
pub use reporting::CompilerLog;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError<'source> {
    #[error("encountered errors during parsing")]
    ParseError(CompilerLog<'source>),

    #[error(transparent)]
    CodeGenerationError(#[from] CodeGenerationError),
}

pub struct CompileRequest {
    pub game: Game,
    pub includes: Vec<SourceFile>,
    pub target: SourceFile,
    pub text_data: Option<CodeGenTextData>,
}

pub struct SourceFile {
    pub name: String,
    pub contents: String,
}

pub fn compile(request: &CompileRequest) -> Result<(Vec<u8>, CompilerLog), CompilerError> {
    // Register source files with the error log
    let mut log = CompilerLog::new();
    let mut file_ids = request
        .includes
        .iter()
        .map(|file| log.add(&file.name, &file.contents))
        .collect_vec();
    file_ids.push(log.add(&request.target.name, &request.target.contents));

    // Parse sources
    let mut surface_scripts = Vec::new();
    for (i, source_file) in request.includes.iter().enumerate() {
        let surface_script = parser::parse(file_ids[i], &source_file.contents, &mut log);
        surface_scripts.push(surface_script);
    }
    surface_scripts.push(parser::parse(
        file_ids[file_ids.len() - 1],
        &request.target.contents,
        &mut log,
    ));
    if log.has_errors() {
        return Err(CompilerError::ParseError(log));
    }

    // Evaluate sources
    // Combining sources works
    let combined_sources = surface_scripts.into_iter().flat_map(|s| s.0).collect_vec();
    let combined_script = exalt_ast::surface::Script(combined_sources);
    let script = if let Some(script) = semantic::analyze(&combined_script, &mut log) {
        script
    } else {
        return Err(CompilerError::ParseError(log));
    };

    // Generate code
    match codegen::serialize(
        &request.target.name,
        &script,
        request.game,
        request.text_data.as_ref().cloned(),
    ) {
        Ok(raw) => Ok((raw, log)),
        Err(err) => Err(CompilerError::CodeGenerationError(err)),
    }
}
