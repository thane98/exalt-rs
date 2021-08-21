use crate::Game;

pub enum Literal {
    Str(String),
    Int(i32),
    Float(f32),
}

pub enum Operator {
    Add,
    FloatAdd,
    Subtract,
    FloatSubtract,
    Multiply,
    FloatMultiply,
    Divide,
    FloatDivide,
    Modulo,
    IntNegate,
    FloatNegate,
    BinaryNot,
    LogicalNot,
    BinaryOr,
    BinaryAnd,
    Xor,
    LeftShift,
    RightShift,
    Equal,
    FloatEqual,
    NotEqual,
    FloatNotEqual,
    LessThan,
    FloatLessThan,
    GreaterThan,
    FloatGreaterThan,
    LessThanOrEqualTo,
    FloatLessThanOrEqualTo,
    GreaterThanOrEqualTo,
    FloatGreaterThanEqualTo,
    Increment,
    Decrement,
}

pub enum Expression {
    Literal(Literal),
    Grouped(Box<Expression>),
    Unary {
        operator: Operator,
        operand: Box<Expression>,
    },
    Binary {
        operator: Operator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    Funcall {
        name: String,
        args: Vec<Expression>,
    },
    VarReference { 
        identifier: String 
    },
    ArrayReference {
        identifier: String,
        index: Box<Expression>,
    },
    PointerReference {
        identifier: String,
        index: Box<Expression>,
    },
}

pub enum Statement {
    GameDeclaration(Game),
    ExpressionStatement(Expression),
}
