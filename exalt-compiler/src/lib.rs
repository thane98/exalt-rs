mod codegen;
mod completion;
mod eval;
mod includes;
mod lexer;
pub mod parser;
mod reporting;
mod semantic;
mod symbol;

use std::path::PathBuf;

pub use codegen::CodeGenerationError;
use exalt_assembler::CodeGenTextData;
use exalt_ast::Script;
use exalt_lir::Game;
pub use lexer::{Peekable, Token};
pub use reporting::CompilerLog;
pub use symbol::{Scope, SymbolTable};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("encountered errors during parsing")]
    ParseError(CompilerLog),

    #[error("could not find target file '{0}'")]
    FileNotFound(PathBuf),

    #[error("bad target file name '{0}'")]
    BadTargetFile(PathBuf),

    #[error(transparent)]
    CodeGenerationError(#[from] CodeGenerationError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct CompileRequest {
    pub game: Game,
    pub target: PathBuf,
    pub output: Option<PathBuf>,
    pub text_data: Option<CodeGenTextData>,
    pub additional_includes: Vec<PathBuf>,
}

pub struct ParseRequest {
    pub game: Game,
    pub target: PathBuf,
    pub source: Option<String>,
    pub additional_includes: Vec<PathBuf>,
}

impl ParseRequest {
    pub fn source_name(&self) -> Result<String, CompilerError> {
        self.target
            .to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CompilerError::BadTargetFile(self.target.clone()))
    }
}

pub struct ParseResult {
    pub parse_tree: exalt_ast::surface::Script,
    pub script: Script,
    pub symbol_table: SymbolTable,
    pub log: CompilerLog,
}

impl CompileRequest {
    pub fn source_name(&self) -> Result<String, CompilerError> {
        self.target
            .to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CompilerError::BadTargetFile(self.target.clone()))
    }

    pub fn script_name(&self) -> Result<String, CompilerError> {
        let path = if let Some(path) = self.output.as_deref() {
            path
        } else {
            &self.target
        };
        path.file_stem()
            .map(|f| f.to_string_lossy().to_string() + ".cmb")
            .ok_or_else(|| CompilerError::BadTargetFile(path.to_path_buf()))
    }

    pub fn output_path(&self) -> Result<PathBuf, CompilerError> {
        if let Some(path) = self.output.as_deref() {
            Ok(path.to_owned())
        } else {
            let parent = self
                .target
                .parent()
                .ok_or_else(|| CompilerError::BadTargetFile(self.target.clone()))?;
            Ok(parent.join(self.script_name()?))
        }
    }
}

pub fn compile(request: &CompileRequest) -> Result<(), CompilerError> {
    let output_path = request.output_path()?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| CompilerError::BadTargetFile(output_path))?;
    }
    std::fs::write(request.output_path()?, compile_to_vec(request)?)?;
    Ok(())
}

pub fn compile_to_vec(request: &CompileRequest) -> Result<Vec<u8>, CompilerError> {
    // Load input
    let contents = std::fs::read_to_string(&request.target)
        .map_err(|_| CompilerError::FileNotFound(request.target.clone()))?;

    // Parse sources
    let mut log = CompilerLog::new();
    let script = parser::parse(log.peek_file_id(), &contents, &mut log);
    log.add(request.source_name()?, contents.clone());

    let script = match includes::build_script_with_includes(
        request.target.clone(),
        script,
        &mut log,
        &request.additional_includes,
    ) {
        Ok(script) => script,
        Err(err) => {
            log.log_error(err.into());
            log.print();
            return Err(CompilerError::ParseError(log));
        }
    };
    if log.has_errors() {
        log.print();
        return Err(CompilerError::ParseError(log));
    }

    // Evaluate sources
    let (script, symbol_table) = if let Some(script) = semantic::analyze(&script, &mut log) {
        script
    } else {
        log.print();
        return Err(CompilerError::ParseError(log));
    };

    // Generate code
    let script_name = request.script_name()?;
    match codegen::serialize(
        &script_name,
        &script,
        &symbol_table,
        request.game,
        request.text_data.as_ref().cloned(),
    ) {
        Ok(raw) => Ok(raw),
        Err(err) => Err(CompilerError::CodeGenerationError(err)),
    }
}

pub fn parse(request: &ParseRequest) -> Result<ParseResult, CompilerError> {
    // Load input
    let contents = if let Some(source) = &request.source {
        source.clone()
    } else {
        std::fs::read_to_string(&request.target)
            .map_err(|_| CompilerError::FileNotFound(request.target.clone()))?
    };

    // Parse sources
    let mut log = CompilerLog::new();
    let parse_tree = parser::parse(log.peek_file_id(), &contents, &mut log);
    log.add(request.source_name()?, contents.clone());
    let parse_tree = match includes::build_script_with_includes(
        request.target.clone(),
        parse_tree,
        &mut log,
        &request.additional_includes,
    ) {
        Ok(parse_tree) => parse_tree,
        Err(err) => {
            log.log_error(err.into());
            return Err(CompilerError::ParseError(log));
        }
    };
    if log.has_errors() {
        return Err(CompilerError::ParseError(log));
    }

    // Evaluate sources
    if let Some((script, symbol_table)) = semantic::analyze(&parse_tree, &mut log) {
        Ok(ParseResult {
            parse_tree,
            script,
            symbol_table,
            log,
        })
    } else {
        Err(CompilerError::ParseError(log))
    }
}
