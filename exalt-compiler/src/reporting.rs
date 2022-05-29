use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use exalt_ast::surface::Identifier;
use exalt_ast::{FileId, Location, Operator};

/// Aggregator for issues found while compiling
#[derive(Debug)]
pub struct CompilerLog<'source> {
    pub errors: Vec<ErrorMessage>,
    pub warnings: Vec<WarningMessage>,
    pub files: SimpleFiles<&'source str, &'source str>,
}

impl<'source> CompilerLog<'source> {
    pub fn new() -> Self {
        CompilerLog {
            errors: Vec::new(),
            warnings: Vec::new(),
            files: SimpleFiles::new(),
        }
    }

    pub fn add(&mut self, name: &'source str, source: &'source str) -> FileId {
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

impl<'source> Default for CompilerLog<'source> {
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
}

impl ParserError {
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
    ArrayReassignment(Identifier),
}

impl SemanticError {
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
                .with_labels(option_to_vec(
                    primary(l).map(|v| v.with_message("break can only be used inside a loop or match statement")),
                )),
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
            SemanticError::ArrayReassignment(id) => Diagnostic::error()
                .with_message("an array variable cannot have multiple assignments")
                .with_labels(option_to_vec(primary(&id.location).map(|v| {
                    v.with_message(format!(
                        "array variable '{}' has multiple assignments",
                        &id.value
                    ))
                }))),
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
    }
}

fn secondary(location: &Location) -> Option<Label<FileId>> {
    match location {
        Location::Source(file_id, range) => Some(Label::secondary(*file_id, range.clone())),
        Location::Generated => None,
    }
}

fn option_to_vec<T>(option: Option<T>) -> Vec<T> {
    match option {
        None => Vec::new(),
        Some(elem) => vec![elem],
    }
}
