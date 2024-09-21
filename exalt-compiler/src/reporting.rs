use std::borrow::Cow;
use std::path::PathBuf;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{self};
use exalt_ast::surface::Identifier;
use exalt_ast::{FileId, Location, Operator};

/// Aggregator for issues found while compiling
#[derive(Debug)]
pub struct CompilerLog {
    pub errors: Vec<ErrorMessage>,
    pub warnings: Vec<WarningMessage>,
    pub files: SimpleFiles<String, String>,
    next_file_id: usize,
}

impl CompilerLog {
    pub fn new() -> Self {
        CompilerLog {
            errors: Vec::new(),
            warnings: Vec::new(),
            files: SimpleFiles::new(),
            next_file_id: 0,
        }
    }

    pub fn file(&self, file_id: FileId) -> Option<String> {
        self.files.get(file_id).ok().map(|f| f.name().to_string())
    }

    pub fn peek_file_id(&self) -> FileId {
        self.next_file_id
    }

    pub fn add(&mut self, name: String, source: String) -> FileId {
        self.next_file_id += 1;
        self.files.add(name, source)
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn log_error(&mut self, error: ErrorMessage) {
        self.errors.push(error);
    }

    pub fn log_warning(&mut self, warning: WarningMessage) {
        self.warnings.push(warning)
    }

    pub fn print(&self) {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();
        for warning in &self.warnings {
            let diagnostic = warning.to_diagnostic();
            term::emit(&mut writer.lock(), &config, &self.files, &diagnostic).unwrap_or_default();
        }
        for error in &self.errors {
            let diagnostic = error.to_diagnostic();
            term::emit(&mut writer.lock(), &config, &self.files, &diagnostic).unwrap_or_default();
        }
    }
}

impl Default for CompilerLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Top-level error type
#[derive(Debug)]
pub enum ErrorMessage {
    Parser(ParserError),
    Semantic(SemanticError),
}

impl ErrorMessage {
    pub fn location(&self) -> Option<&Location> {
        match self {
            ErrorMessage::Parser(err) => err.location(),
            ErrorMessage::Semantic(err) => Some(err.location()),
        }
    }

    pub fn message(&self) -> Cow<str> {
        match self {
            ErrorMessage::Parser(err) => err.message(),
            ErrorMessage::Semantic(err) => err.message(),
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic<FileId> {
        match self {
            ErrorMessage::Parser(err) => err.to_diagnostic(),
            ErrorMessage::Semantic(err) => err.to_diagnostic(),
        }
    }
}

impl From<ParserError> for ErrorMessage {
    fn from(err: ParserError) -> Self {
        ErrorMessage::Parser(err)
    }
}

impl From<SemanticError> for ErrorMessage {
    fn from(err: SemanticError) -> Self {
        ErrorMessage::Semantic(err)
    }
}

/// Parser-specific error messages
#[derive(Debug)]
pub enum ParserError {
    InvalidToken(Location),
    UnexpectedEof,
    UnexpectedToken(Location, String, String),
    InvalidInt(Location),
    InvalidFloat(Location),
    ExpectedAssignment(Location),
    ExpectedExpression(Location),
    ExpectedReference(Location),
    ExpectedLoopRange(Location),
    ExpectedStmt(Location),
    ExpectedDecl(Location),
    MultipleDefaultCases(Location, Location),
    DoubleDereference(Location),
    ExpectedIncludePathComponent(Location),
    PathNormalizationError(Location, PathBuf),
    IncludeNotFound(Location),
    IncludeError(Location),
}

impl ParserError {
    pub fn location(&self) -> Option<&Location> {
        match self {
            ParserError::InvalidToken(l) => Some(l),
            ParserError::UnexpectedEof => None,
            ParserError::UnexpectedToken(l, _, _) => Some(l),
            ParserError::InvalidInt(l) => Some(l),
            ParserError::InvalidFloat(l) => Some(l),
            ParserError::ExpectedAssignment(l) => Some(l),
            ParserError::ExpectedExpression(l) => Some(l),
            ParserError::ExpectedReference(l) => Some(l),
            ParserError::ExpectedLoopRange(l) => Some(l),
            ParserError::ExpectedStmt(l) => Some(l),
            ParserError::ExpectedDecl(l) => Some(l),
            ParserError::MultipleDefaultCases(l, _) => Some(l),
            ParserError::DoubleDereference(l) => Some(l),
            ParserError::ExpectedIncludePathComponent(l) => Some(l),
            ParserError::PathNormalizationError(l, _) => Some(l),
            ParserError::IncludeNotFound(l) => Some(l),
            ParserError::IncludeError(l) => Some(l),
        }
    }

    pub fn message(&self) -> Cow<str> {
        match self {
            ParserError::InvalidToken(_) => Cow::Borrowed("invalid token"),
            ParserError::UnexpectedEof => Cow::Borrowed("unexpected end of file"),
            ParserError::UnexpectedToken(_, e, a) => {
                Cow::Owned(format!("expected token '{}' found '{}'", e, a))
            }
            ParserError::InvalidInt(_) => Cow::Borrowed("int value must fit in 32 bits"),
            ParserError::InvalidFloat(_) => Cow::Borrowed("float value must fit in 32 bits"),
            ParserError::ExpectedAssignment(_) => Cow::Borrowed("expected assignment"),
            ParserError::ExpectedExpression(_) => Cow::Borrowed("expected expression"),
            ParserError::ExpectedReference(_) => Cow::Borrowed("expected reference"),
            ParserError::ExpectedLoopRange(_) => Cow::Borrowed("expected loop range"),
            ParserError::ExpectedStmt(_) => Cow::Borrowed("expected statement"),
            ParserError::ExpectedDecl(_) => Cow::Borrowed("expected declaration"),
            ParserError::MultipleDefaultCases(_, _) => {
                Cow::Borrowed("match can only have one default case")
            }
            ParserError::DoubleDereference(_) => {
                Cow::Borrowed("Exalt does not support double dereferences")
            }
            ParserError::ExpectedIncludePathComponent(_) => {
                Cow::Borrowed("expected include path component")
            }
            ParserError::PathNormalizationError(_, p) => {
                Cow::Owned(format!("unable to normalize path {}", p.display()))
            }
            ParserError::IncludeNotFound(_) => Cow::Borrowed("unable to resolve path"),
            ParserError::IncludeError(_) => Cow::Borrowed("undefined include error"),
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic<FileId> {
        match self {
            ParserError::InvalidToken(l) => Diagnostic::error()
                .with_message("invalid token")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("invalid token")),
                )),
            ParserError::UnexpectedEof => {
                Diagnostic::error().with_message("unexpected end of file")
            }
            ParserError::UnexpectedToken(l, e, a) => Diagnostic::error()
                .with_message(format!("expected token '{}' found '{}'", e, a))
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("invalid token")),
                )),
            ParserError::InvalidInt(l) => Diagnostic::error()
                .with_message("int value must fit in 32 bits")
                .with_labels(option_to_vec(primary(l))),
            ParserError::InvalidFloat(l) => Diagnostic::error()
                .with_message("float value must fit in 32 bits")
                .with_labels(option_to_vec(primary(l))),
            ParserError::ExpectedAssignment(l) => Diagnostic::error()
                .with_message("expected assignment")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected assignment")),
                )),
            ParserError::ExpectedExpression(l) => Diagnostic::error()
                .with_message("expected expression")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected expression")),
                )),
            ParserError::ExpectedReference(l) => Diagnostic::error()
                .with_message("expected reference")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected reference")),
                )),
            ParserError::ExpectedLoopRange(l) => Diagnostic::error()
                .with_message("expected loop range")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected loop range")),
                )),
            ParserError::ExpectedStmt(l) => Diagnostic::error()
                .with_message("expected statement")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected statement")),
                )),
            ParserError::ExpectedDecl(l) => Diagnostic::error()
                .with_message("expected declaration")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected declaration")),
                )),
            ParserError::MultipleDefaultCases(first, second) => Diagnostic::error()
                .with_message("match can only have one default case")
                .with_labels({
                    let mut labels = option_to_vec(
                        primary(second)
                            .map(|v| v.with_message("second default case declared here")),
                    );
                    labels.extend(option_to_vec(
                        secondary(first)
                            .map(|v| v.with_message("first default case declared here")),
                    ));
                    labels
                }),
            ParserError::DoubleDereference(l) => Diagnostic::error()
                .with_message("Exalt does not support double dereferences")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("attempted to dereference twice")),
                )),
            ParserError::ExpectedIncludePathComponent(l) => Diagnostic::error()
                .with_message("expected include path component")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("expected include path component")),
                )),
            ParserError::PathNormalizationError(l, p) => Diagnostic::error()
                .with_message(format!("unable to normalize path {}", p.display()))
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("unable to normalize path")),
                )),
            ParserError::IncludeNotFound(l) => Diagnostic::error()
                .with_message("unable to resolve path")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("could not find this file")),
                )),
            ParserError::IncludeError(l) => Diagnostic::error()
                .with_message("undefined include error")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("undefined include error")),
                )),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SemanticError {
    ExpectedConstExpr(Location),
    SymbolRedefinition(Location, Location, String),
    UndefinedVariable(Identifier),
    UndefinedAnnotation(Identifier),
    UndefinedEnum(Identifier),
    UndefinedVariant(Identifier),
    IncompatibleOperator(Location, String, Operator),
    IncompatibleOperands(Location, String, String),
    DivideByZero(Location),
    ExpectedReferenceOperand(Location),
    BadBreak(Location),
    BadContinue(Location),
    UnresolvedLabel(Location, usize),
    InvalidType(Location, String, String),
    SignatureDisagreement(Location, String),
    BadArgCount(Location, usize, usize),
    BadExlCall(Location),
    NegativeArrayLength(Location),
}

impl SemanticError {
    pub fn location(&self) -> &Location {
        match self {
            SemanticError::ExpectedConstExpr(l) => l,
            SemanticError::SymbolRedefinition(l, _, _) => l,
            SemanticError::UndefinedVariable(i) => &i.location,
            SemanticError::UndefinedAnnotation(i) => &i.location,
            SemanticError::UndefinedEnum(i) => &i.location,
            SemanticError::UndefinedVariant(i) => &i.location,
            SemanticError::IncompatibleOperator(l, _, _) => l,
            SemanticError::IncompatibleOperands(l, _, _) => l,
            SemanticError::DivideByZero(l) => l,
            SemanticError::ExpectedReferenceOperand(l) => l,
            SemanticError::BadBreak(l) => l,
            SemanticError::BadContinue(l) => l,
            SemanticError::UnresolvedLabel(l, _) => l,
            SemanticError::InvalidType(l, _, _) => l,
            SemanticError::SignatureDisagreement(l, _) => l,
            SemanticError::BadArgCount(l, _, _) => l,
            SemanticError::BadExlCall(l) => l,
            SemanticError::NegativeArrayLength(l) => l,
        }
    }

    pub fn message(&self) -> Cow<str> {
        match self {
            SemanticError::ExpectedConstExpr(_) => Cow::Borrowed("expected constant expression"),
            SemanticError::SymbolRedefinition(_, _, _) => {
                Cow::Borrowed("symbol redefined in the same scope")
            }
            SemanticError::UndefinedVariable(_) => Cow::Borrowed("undefined variable"),
            SemanticError::UndefinedAnnotation(_) => Cow::Borrowed("undefined annotation"),
            SemanticError::UndefinedEnum(_) => Cow::Borrowed("undefined enum"),
            SemanticError::UndefinedVariant(_) => Cow::Borrowed("undefined variant"),
            SemanticError::IncompatibleOperator(_, _, _) => {
                Cow::Borrowed("operator has incompatible operand")
            }
            SemanticError::IncompatibleOperands(_, _, _) => {
                Cow::Borrowed("operator has incompatible types")
            }
            SemanticError::DivideByZero(_) => Cow::Borrowed("division by zero"),
            SemanticError::ExpectedReferenceOperand(_) => {
                Cow::Borrowed("operation requires a variable")
            }
            SemanticError::BadBreak(_) => Cow::Borrowed("break cannot be used in this context"),
            SemanticError::BadContinue(_) => {
                Cow::Borrowed("continue cannot be used in this context")
            }
            SemanticError::UnresolvedLabel(_, _) => Cow::Borrowed("unresolved label"),
            SemanticError::InvalidType(_, _, _) => Cow::Borrowed("type mismatch"),
            SemanticError::SignatureDisagreement(_, _) => {
                Cow::Borrowed("actual and expected signatures differ")
            }
            SemanticError::BadArgCount(_, _, _) => Cow::Borrowed("incorrect number of arguments"),
            SemanticError::BadExlCall(_) => {
                Cow::Borrowed("exlcall takes an integer call id as its first argument")
            }
            SemanticError::NegativeArrayLength(_) => {
                Cow::Borrowed("array length cannot be negative")
            }
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic<FileId> {
        match self {
            SemanticError::ExpectedConstExpr(l) => Diagnostic::error()
                .with_message("expected constant expression")
                .with_labels(option_to_vec(primary(l))),
            SemanticError::SymbolRedefinition(original, new, name) => Diagnostic::error()
                .with_message("symbol redefined in the same scope")
                .with_labels({
                    let mut labels = option_to_vec(
                        primary(new).map(|v| v.with_message(format!("redefined '{}' here", name))),
                    );
                    labels.extend(option_to_vec(secondary(original).map(|v| {
                        v.with_message(format!("originally defined '{}' here", name))
                    })));
                    labels
                }),
            SemanticError::UndefinedVariable(id) => Diagnostic::error()
                .with_message("undefined variable")
                .with_labels(option_to_vec(primary(&id.location).map(|v| {
                    v.with_message(format!("variable '{}' is undefined", &id.value))
                }))),
            SemanticError::UndefinedAnnotation(id) => Diagnostic::error()
                .with_message("undefined annotation")
                .with_labels(option_to_vec(primary(&id.location).map(|v| {
                    v.with_message(format!("annotation '{}' is undefined", &id.value))
                }))),
            SemanticError::UndefinedEnum(id) => Diagnostic::error()
                .with_message("undefined enum")
                .with_labels(option_to_vec(primary(&id.location).map(|v| {
                    v.with_message(format!("enum '{}' is undefined", &id.value))
                }))),
            SemanticError::UndefinedVariant(id) => Diagnostic::error()
                .with_message("undefined enum variant")
                .with_labels(option_to_vec(primary(&id.location).map(|v| {
                    v.with_message(format!("enum has no variant named '{}'", &id.value))
                }))),
            SemanticError::IncompatibleOperator(l, d1, d2) => Diagnostic::error()
                .with_message("operator has incompatible operand")
                .with_labels(option_to_vec(primary(l).map(|v| {
                    v.with_message(format!("operator '{}' does not support type '{}'", d2, d1))
                }))),
            SemanticError::IncompatibleOperands(l, d1, d2) => Diagnostic::error()
                .with_message("operator has incompatible types")
                .with_labels(option_to_vec(primary(l).map(|v| {
                    v.with_message(format!("left side is '{}' but right side is '{}'", d1, d2))
                })))
                .with_notes(vec![format!(
                    "consider casting from '{}' to '{}' or vice versa",
                    d1, d2
                )]),
            SemanticError::DivideByZero(l) => Diagnostic::error()
                .with_message("division by zero")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("division by zero")),
                )),
            SemanticError::ExpectedReferenceOperand(l) => Diagnostic::error()
                .with_message("operation requires a variable")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("operand is not a variable")),
                )),
            SemanticError::BadBreak(l) => Diagnostic::error()
                .with_message("break cannot be used in this context")
                .with_labels(option_to_vec(primary(l).map(|v| {
                    v.with_message("break can only be used inside a loop or match statement")
                }))),
            SemanticError::BadContinue(l) => Diagnostic::error()
                .with_message("continue cannot be used in this context")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("continue can only be used inside a loop")),
                )),
            SemanticError::UnresolvedLabel(l, count) => Diagnostic::error()
                .with_message("unresolved label")
                .with_labels(option_to_vec(primary(l).map(|v| {
                    v.with_message({
                        if *count == 1 {
                            "referenced here".to_owned()
                        } else if *count == 2 {
                            "referenced here and 1 more location".to_owned()
                        } else {
                            format!("referenced here and {} more locations", count - 1)
                        }
                    })
                }))),
            SemanticError::InvalidType(l, t1, t2) => Diagnostic::error()
                .with_message("type mismatch")
                .with_labels(option_to_vec(primary(l).map(|v| {
                    v.with_message(format!("expected type '{}' but found '{}'", t1, t2))
                }))),
            SemanticError::SignatureDisagreement(l, message) => Diagnostic::error()
                .with_message("actual and expected signatures differ")
                .with_labels(option_to_vec(primary(l).map(|v| v.with_message(message)))),
            SemanticError::BadArgCount(loc, expected, actual) => Diagnostic::error()
                .with_message("incorrect number of arguments")
                .with_labels(option_to_vec(primary(loc).map(|v| {
                    v.with_message(format!(
                        "function takes {} arguments but found {}",
                        expected, actual
                    ))
                }))),
            SemanticError::BadExlCall(l) => Diagnostic::error()
                .with_message("exlcall takes an integer call id as its first argument")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("missing call id")),
                )),
            SemanticError::NegativeArrayLength(l) => Diagnostic::error()
                .with_message("array length cannot be negative")
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("array length cannot be negative")),
                )),
        }
    }
}

/// Top-level warning type
#[derive(Debug)]
pub enum WarningMessage {
    DeadCode(Location),
    UnusedLabel(Location),
}

impl WarningMessage {
    pub fn location(&self) -> &Location {
        match self {
            WarningMessage::DeadCode(l) => l,
            WarningMessage::UnusedLabel(l) => l,
        }
    }

    pub fn message(&self) -> Cow<str> {
        match self {
            WarningMessage::DeadCode(_) => Cow::Borrowed("unreachable code"),
            WarningMessage::UnusedLabel(_) => Cow::Borrowed("label is never used"),
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic<FileId> {
        match self {
            WarningMessage::DeadCode(l) => Diagnostic::warning()
                .with_message("unreachable code")
                .with_labels(option_to_vec(primary(l))),
            WarningMessage::UnusedLabel(l) => Diagnostic::warning()
                .with_message("label is never used")
                .with_labels(option_to_vec(primary(l))),
        }
    }
}

fn primary(location: &Location) -> Option<Label<FileId>> {
    match location {
        Location::Source(file_id, range) => Some(Label::primary(*file_id, range.clone())),
        Location::Generated => None,
        _ => None,
    }
}

fn secondary(location: &Location) -> Option<Label<FileId>> {
    match location {
        Location::Source(file_id, range) => Some(Label::secondary(*file_id, range.clone())),
        Location::Generated => None,
        _ => None,
    }
}

fn option_to_vec<T>(option: Option<T>) -> Vec<T> {
    match option {
        None => Vec::new(),
        Some(elem) => vec![elem],
    }
}
