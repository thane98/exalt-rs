use derive_new::new;

use crate::{Literal, Location, Notation, Operator};

#[derive(Debug, Clone, new)]
pub struct Identifier {
    pub location: Location,
    pub value: String,
}

/// Raw representation of variables references / l-values
#[derive(Debug)]
pub enum Ref {
    Var(Identifier),
    Index(Identifier, Box<Expr>),
    Dereference(Identifier, Option<Box<Expr>>),
}

/// Raw representation of an expression
#[derive(Debug)]
pub enum Expr {
    Array(Location, Vec<Expr>),
    Literal(Location, Literal),
    EnumAccess(Location, Identifier, Identifier),
    Unary(Location, Box<Expr>, Operator),
    Binary(Location, Box<Expr>, Operator, Box<Expr>),
    FunctionCall(Location, Identifier, Vec<Expr>),
    Ref(Location, Ref),
    Grouped(Location, Box<Expr>),
    Increment(Location, Ref, Operator, Notation),
    AddressOf(Location, Ref),
}

impl Expr {
    pub fn location(&self) -> &Location {
        match self {
            Expr::Array(l, _) => l,
            Expr::Literal(l, _) => l,
            Expr::EnumAccess(l, _, _) => l,
            Expr::Unary(l, _, _) => l,
            Expr::Binary(l, _, _, _) => l,
            Expr::FunctionCall(l, _, _) => l,
            Expr::Ref(l, _) => l,
            Expr::Grouped(l, _) => l,
            Expr::Increment(l, _, _, _) => l,
            Expr::AddressOf(l, _) => l,
        }
    }
}

/// A single case in a match statement
#[derive(Debug, new)]
pub struct Case {
    pub conditions: Vec<Expr>,
    pub body: Stmt,
}

/// Raw representation of statements
#[derive(Debug)]
pub enum Stmt {
    Assignment {
        location: Location,
        left: Ref,
        op: Operator,
        right: Expr,
    },
    Block(Location, Vec<Stmt>),
    Break(Location),
    Continue(Location),
    ExprStmt(Location, Expr),
    For {
        location: Location,
        init: Box<Stmt>,
        check: Expr,
        step: Box<Stmt>,
        body: Box<Stmt>,
    },
    Goto(Location, Identifier),
    If {
        location: Location,
        condition: Expr,
        then_part: Box<Stmt>,
        else_part: Option<Box<Stmt>>,
    },
    Label(Location, Identifier),
    Match {
        location: Location,
        switch: Expr,
        cases: Vec<Case>,
        default: Option<Box<Stmt>>,
    },
    Printf(Location, Vec<Expr>),
    Return(Location, Option<Expr>),
    VarDecl(Location, Identifier, Option<Expr>),
    While {
        location: Location,
        condition: Expr,
        body: Box<Stmt>,
    },
    Yield(Location),
}

impl Stmt {
    pub fn location(&self) -> &Location {
        match self {
            Stmt::Assignment { location, .. } => location,
            Stmt::Block(location, _) => location,
            Stmt::Break(location) => location,
            Stmt::Continue(location) => location,
            Stmt::ExprStmt(location, _) => location,
            Stmt::For { location, .. } => location,
            Stmt::Goto(location, _) => location,
            Stmt::If { location, .. } => location,
            Stmt::Label(location, _) => location,
            Stmt::Match { location, .. } => location,
            Stmt::Printf(location, _) => location,
            Stmt::Return(location, _) => location,
            Stmt::VarDecl(location, _, _) => location,
            Stmt::While { location, .. } => location,
            Stmt::Yield(location) => location,
        }
    }
}

/// Raw representation of annotations
#[derive(Debug, new)]
pub struct Annotation {
    pub location: Location,
    pub identifier: Identifier,
    pub args: Vec<Expr>,
}

/// Raw representation of an enum variant
#[derive(Debug, new)]
pub struct EnumVariant {
    pub location: Location,
    pub identifier: Identifier,
    pub value: Expr,
}

#[derive(Debug)]
pub enum IncludePathComponent {
    Node(String),
    Parent,
}

/// Raw representation of declarations
#[derive(Debug)]
pub enum Decl {
    Constant {
        location: Location,
        identifier: Identifier,
        value: Expr,
    },
    Enum {
        location: Location,
        identifier: Identifier,
        variants: Vec<EnumVariant>,
    },
    Function {
        location: Location,
        annotations: Vec<Annotation>,
        identifier: Identifier,
        parameters: Vec<Identifier>,
        body: Stmt,
    },
    Global(Location, Identifier, Option<Expr>),
    Callback {
        location: Location,
        annotations: Vec<Annotation>,
        event_type: Expr,
        args: Vec<Expr>,
        body: Stmt,
    },
    Include {
        location: Location,
        path: Vec<IncludePathComponent>,
    },
    FunctionAlias {
        location: Location,
        identifier: Identifier,
        alias: Identifier,
    },
    FunctionExtern {
        location: Location,
        identifier: Identifier,
        parameters: Vec<Identifier>,
    },
}

impl Decl {
    pub fn location(&self) -> &Location {
        match self {
            Decl::Constant { location, .. } => location,
            Decl::Enum { location, .. } => location,
            Decl::Function { location, .. } => location,
            Decl::Global(location, _, _) => location,
            Decl::Callback { location, .. } => location,
            Decl::Include { location, .. } => location,
            Decl::FunctionAlias { location, .. } => location,
            Decl::FunctionExtern { location, .. } => location,
        }
    }

    pub fn is_function_like(&self) -> bool {
        matches!(self, Decl::Callback { .. } | Decl::Function { .. })
    }
}

/// Raw representation of an Exalt script
#[derive(Debug, new)]
pub struct Script(pub Vec<Decl>);
