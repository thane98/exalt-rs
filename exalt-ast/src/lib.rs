use std::cell::RefCell;
use std::fmt::Display;
use std::ops::Range;
use std::rc::Rc;

use derive_new::new;
use indexmap::IndexMap;

pub mod surface;

/// Unique identifier for a source file.
pub type FileId = usize;

/// Alias used to represent shared pointers.
pub type Shared<T> = Rc<RefCell<T>>;

/// Represents a representation in some source code.
/// Can be either text or generated (ex. by the decompiler)
#[derive(Debug, Clone)]
pub enum Location {
    Source(FileId, Range<usize>),
    Generated,
}

impl Location {
    pub fn merge(&self, other: &Location) -> Location {
        match (self, other) {
            (Location::Source(f1, r1), Location::Source(f2, r2)) => {
                if f1 != f2 {
                    panic!("file ID mismatch")
                } else {
                    Location::Source(*f1, r1.start.min(r2.start)..r1.end.max(r2.end))
                }
            }
            (Location::Generated, Location::Generated) => Location::Generated,
            _ => panic!(),
        }
    }
}

/// Container for literal values that can be represented directly in the source / binary.
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i32),
    Str(String),
    Float(f32),
}

impl Literal {
    pub fn data_type(&self) -> DataType {
        match self {
            Literal::Int(_) => DataType::Int,
            Literal::Str(_) => DataType::Str,
            Literal::Float(_) => DataType::Float,
        }
    }
}

/// Notation for increment expressions myVar++ vs. ++myVar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notation {
    Prefix,
    Postfix,
}

/// Precedence levels for operators
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Precedence {
    Lowest,
    Assignment,
    LogicalOr,
    LogicalAnd,
    Bitwise,
    Equality,
    Comparison,
    Shift,
    Term,
    Factor,
    Unary,
    Access,
}

impl From<Operator> for Precedence {
    fn from(op: Operator) -> Self {
        match op {
            Operator::Increment => Precedence::Unary,
            Operator::Decrement => Precedence::Unary,
            Operator::LogicalNot => Precedence::Unary,
            Operator::BitwiseNot => Precedence::Unary,
            Operator::Negate => Precedence::Unary,
            Operator::Multiply => Precedence::Factor,
            Operator::Divide => Precedence::Factor,
            Operator::Modulo => Precedence::Factor,
            Operator::Add => Precedence::Term,
            Operator::Subtract => Precedence::Term,
            Operator::LeftShift => Precedence::Shift,
            Operator::RightShift => Precedence::Shift,
            Operator::Equal => Precedence::Equality,
            Operator::NotEqual => Precedence::Equality,
            Operator::LessThan => Precedence::Comparison,
            Operator::GreaterThan => Precedence::Comparison,
            Operator::LessThanEqualTo => Precedence::Comparison,
            Operator::GreaterThanEqualTo => Precedence::Comparison,
            Operator::LogicalAnd => Precedence::LogicalAnd,
            Operator::LogicalOr => Precedence::LogicalOr,
            Operator::Assign => Precedence::Assignment,
            Operator::AssignAdd => Precedence::Assignment,
            Operator::AssignSubtract => Precedence::Assignment,
            Operator::AssignMultiply => Precedence::Assignment,
            Operator::AssignDivide => Precedence::Assignment,
            Operator::AssignModulo => Precedence::Assignment,
            Operator::AssignXor => Precedence::Assignment,
            Operator::AssignLeftShift => Precedence::Assignment,
            Operator::AssignRightShift => Precedence::Assignment,
            Operator::AssignBitwiseAnd => Precedence::Assignment,
            Operator::AssignBitwiseOr => Precedence::Assignment,
            Operator::BitwiseAnd => Precedence::Bitwise,
            Operator::BitwiseOr => Precedence::Bitwise,
            _ => Precedence::Lowest,
        }
    }
}

/// Operators that can act on expressions like "+", "-", etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    // Factor ops
    Divide,
    Multiply,
    Modulo,
    FloatDivide,
    FloatMultiply,

    // Term ops
    Add,
    Subtract,
    FloatAdd,
    FloatSubtract,

    // Shift ops
    LeftShift,
    RightShift,

    // Comparison ops
    LessThan,
    LessThanEqualTo,
    GreaterThan,
    GreaterThanEqualTo,
    FloatLessThan,
    FloatLessThanEqualTo,
    FloatGreaterThan,
    FloatGreaterThanEqualTo,

    // Equality ops
    Equal,
    NotEqual,
    FloatEqual,
    FloatNotEqual,

    // Bitwise ops
    BitwiseAnd,
    Xor,
    BitwiseOr,

    // Logical ops
    LogicalAnd,
    LogicalOr,

    // Unary ops
    LogicalNot,
    BitwiseNot,
    Negate,
    FloatNegate,

    // Increment
    Increment,
    Decrement,

    // Assignment and compound assignment
    Assign,
    AssignMultiply,
    AssignDivide,
    AssignModulo,
    AssignAdd,
    AssignSubtract,
    AssignLeftShift,
    AssignRightShift,
    AssignBitwiseAnd,
    AssignXor,
    AssignBitwiseOr,
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Operator::Divide => "/",
                Operator::Multiply => "*",
                Operator::FloatDivide => "/f",
                Operator::FloatMultiply => "*f",
                Operator::Modulo => "%",
                Operator::Add => "+",
                Operator::Subtract => "-",
                Operator::FloatAdd => "+f",
                Operator::FloatSubtract => "-f",
                Operator::LeftShift => "<<",
                Operator::RightShift => ">>",
                Operator::LessThan => "<",
                Operator::LessThanEqualTo => "<=",
                Operator::GreaterThan => ">",
                Operator::GreaterThanEqualTo => ">=",
                Operator::FloatLessThan => "<f",
                Operator::FloatLessThanEqualTo => "<=f",
                Operator::FloatGreaterThan => ">f",
                Operator::FloatGreaterThanEqualTo => ">=f",
                Operator::Equal => "==",
                Operator::NotEqual => "!=",
                Operator::FloatEqual => "==f",
                Operator::FloatNotEqual => "!=f",
                Operator::BitwiseAnd => "&",
                Operator::Xor => "^",
                Operator::BitwiseOr => "|",
                Operator::LogicalAnd => "&&",
                Operator::LogicalOr => "||",
                Operator::LogicalNot => "!",
                Operator::BitwiseNot => "~",
                Operator::Negate => "-",
                Operator::FloatNegate => "-f",
                Operator::Increment => "++",
                Operator::Decrement => "--",
                Operator::Assign => "=",
                Operator::AssignMultiply => "*=",
                Operator::AssignDivide => "/=",
                Operator::AssignModulo => "%=",
                Operator::AssignAdd => "+=",
                Operator::AssignSubtract => "-=",
                Operator::AssignLeftShift => "<<=",
                Operator::AssignRightShift => ">>=",
                Operator::AssignBitwiseAnd => "&=",
                Operator::AssignXor => "^=",
                Operator::AssignBitwiseOr => "|=",
            }
        )
    }
}

impl Operator {
    pub fn to_shorthand(&self) -> Option<Self> {
        match self {
            Operator::Divide => Some(Operator::AssignDivide),
            Operator::Multiply => Some(Operator::AssignMultiply),
            Operator::Modulo => Some(Operator::AssignModulo),
            Operator::Add => Some(Operator::AssignAdd),
            Operator::Subtract => Some(Operator::AssignSubtract),
            Operator::LeftShift => Some(Operator::AssignLeftShift),
            Operator::RightShift => Some(Operator::AssignRightShift),
            Operator::BitwiseAnd => Some(Operator::AssignBitwiseAnd),
            Operator::Xor => Some(Operator::AssignXor),
            Operator::BitwiseOr => Some(Operator::AssignBitwiseOr),
            _ => None,
        }
    }

    pub fn unwrap_shorthand(&self) -> Option<Self> {
        match self {
            Operator::AssignMultiply => Some(Operator::Multiply),
            Operator::AssignDivide => Some(Operator::Divide),
            Operator::AssignModulo => Some(Operator::Modulo),
            Operator::AssignAdd => Some(Operator::Add),
            Operator::AssignSubtract => Some(Operator::Subtract),
            Operator::AssignLeftShift => Some(Operator::LeftShift),
            Operator::AssignRightShift => Some(Operator::RightShift),
            Operator::AssignBitwiseAnd => Some(Operator::BitwiseAnd),
            Operator::AssignXor => Some(Operator::Xor),
            Operator::AssignBitwiseOr => Some(Operator::BitwiseOr),
            _ => None,
        }
    }
}

/// Sum of all Exalt data types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Str,
    Any,
}

impl DataType {
    pub fn name(&self) -> String {
        match self {
            DataType::Int => "Int".to_owned(),
            DataType::Float => "Float".to_owned(),
            DataType::Str => "String".to_owned(),
            DataType::Any => "Any".to_owned(),
        }
    }
}

/// Metadata for a constant
#[derive(Debug, new)]
pub struct ConstSymbol {
    pub name: String,
    pub location: Location,
    pub value: Literal,
}

/// Metadata for an enum
#[derive(Debug, new)]
pub struct EnumSymbol {
    pub name: String,
    pub location: Location,
    pub variants: IndexMap<String, ConstSymbol>,
}

/// Metadata for an Exalt function or method
#[derive(Debug, new)]
pub struct FunctionSymbol {
    pub name: String,
    pub location: Location,
    pub arity: usize,
}

impl FunctionSymbol {
    pub fn shared(name: String, location: Location, arity: usize) -> Shared<FunctionSymbol> {
        Rc::new(RefCell::new(FunctionSymbol {
            name,
            location,
            arity,
        }))
    }
}

/// Metadata for a label
#[derive(Debug, new)]
pub struct LabelSymbol {
    pub name: String,
    pub location: Location,
    pub references: Vec<Location>,
    pub resolved: bool,
}

/// Metadata for a variable
#[derive(Debug, new)]
pub struct VarSymbol {
    pub name: String,
    pub location: Location,
    pub global: bool,
    #[new(default)]
    pub array: bool,
    #[new(default)]
    pub frame_id: Option<usize>,
    #[new(default)]
    pub assignments: usize,
}

/// Exalt l-values
#[derive(Debug, Clone)]
pub enum Ref {
    Var(Shared<VarSymbol>),
    Index(Shared<VarSymbol>, Box<Expr>),
    Dereference(Shared<VarSymbol>, Option<Box<Expr>>),
}

/// An expression that initializes a new array
#[derive(Debug, Clone)]
pub enum ArrayInit {
    Empty(usize),
    Static(Vec<Expr>),
}

/// Exalt expressions after semantic analysis
#[derive(Debug, Clone)]
pub enum Expr {
    Array(ArrayInit),
    Literal(Literal),
    Grouped(Box<Expr>),
    Unary(Operator, Box<Expr>),
    Binary(Box<Expr>, Operator, Box<Expr>),
    FunctionCall(Shared<FunctionSymbol>, Vec<Expr>),
    Ref(Ref),
    Increment(Ref, Operator, Notation),
    AddressOf(Ref),
}

/// A single case in a match statement
#[derive(Debug, new)]
pub struct Case {
    pub conditions: Vec<Expr>,
    pub body: Stmt,
}

/// Exalt statements
#[derive(Debug)]
pub enum Stmt {
    Assignment {
        left: Ref,
        op: Operator,
        right: Expr,
    },
    Block(Vec<Stmt>),
    Break,
    Continue,
    ExprStmt(Expr),
    For {
        init: Box<Stmt>,
        check: Expr,
        step: Box<Stmt>,
        body: Box<Stmt>,
    },
    Goto(Shared<LabelSymbol>),
    If {
        condition: Expr,
        then_part: Box<Stmt>,
        else_part: Option<Box<Stmt>>,
    },
    Label(Shared<LabelSymbol>),
    Match {
        switch: Expr,
        cases: Vec<Case>,
        default: Option<Box<Stmt>>,
    },
    Printf(Vec<Expr>),
    Return(Option<Expr>),
    VarDecl(Shared<VarSymbol>),
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
    Yield,
}

/// Exalt annotations. These are hard coded because they are only used to control code generation.
#[derive(Debug)]
pub enum Annotation {
    NoDefaultReturn,
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    Unknown(usize),
}

/// Exalt declarations
/// Currently only functions/callbacks because everything else is erased during analysis
#[derive(Debug)]
pub enum Decl {
    Function {
        annotations: Vec<Annotation>,
        symbol: Shared<FunctionSymbol>,
        parameters: Vec<Shared<VarSymbol>>,
        body: Stmt,
    },
    Callback {
        annotations: Vec<Annotation>,
        event_type: usize,
        args: Vec<Literal>,
        body: Stmt,
    },
}

#[derive(Debug, new)]
pub struct Script {
    pub decls: Vec<Decl>,
    pub globals: usize,
}
