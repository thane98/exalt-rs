use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Debug, Clone, Copy, EnumString, Deserialize, Serialize, PartialEq)]
pub enum Game {
    FE9,
    FE10,
    FE11,
    FE12,
    FE13,
    FE14,
    FE15,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct RawScript {
    #[serde(default)]
    pub global_frame_size: usize,
    pub functions: Vec<Function>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Function {
    pub frame_size: usize,
    pub event: u8,
    pub arity: u8,
    pub unknown: u8,
    pub prefix: Vec<u8>,
    pub suffix: Vec<u8>,
    pub name: Option<String>,
    pub args: Vec<CallbackArg>,
    pub code: Vec<Opcode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum CallbackArg {
    Int(i32),
    Str(String),
    Float(f32),
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum Opcode {
    Done,
    VarLoad(u16),
    ArrLoad(u16),
    PtrLoad(u16),
    VarAddr(u16),
    ArrAddr(u16),
    PtrAddr(u16),
    GlobalVarLoad(u16),
    GlobalArrLoad(u16),
    GlobalPtrLoad(u16),
    GlobalVarAddr(u16),
    GlobalArrAddr(u16),
    GlobalPtrAddr(u16),
    IntLoad(i32),
    StrLoad(String),
    FloatLoad(f32),
    Dereference,
    Consume,
    CompleteAssign,
    Fix,
    Float,
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
    Exlcall,
    NotEqual,
    FloatNotEqual,
    Nop0x3D,
    LessThan,
    FloatLessThan,
    LessThanEqualTo,
    FloatLessThanEqualTo,
    GreaterThan,
    FloatGreaterThan,
    GreaterThanEqualTo,
    FloatGreaterThanEqualTo,
    CallById(usize),
    CallByName(String, u8),
    Return,
    Jump(String),
    JumpNotZero(String),
    Or(String),
    JumpZero(String),
    And(String),
    Yield,
    Format(u8),
    Inc,
    Dec,
    Copy,
    ReturnFalse,
    ReturnTrue,
    Label(String),
    StringEquals,
    StringNotEquals,
    Nop0x40,
    Assign,
}
