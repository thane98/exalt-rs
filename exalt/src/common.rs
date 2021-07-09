use serde::{Serialize, Deserialize};

pub enum EventArgType {
    Str,
    Int,
    Float,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum EventArg {
    Str(String),
    Int(i32),
    Float(f32),
}

pub enum Game {
    FE13,
    FE14,
    FE15,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Opcode {
    Done,
    VarLoad(u16),
    ArrLoad(u16),
    PtrLoad(u16),
    VarAddr(u16),
    ArrAddr(u16),
    PtrAddr(u16),
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
    CallById(u8),
    CallByName(String, u8),
    SetReturn,
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
}

pub fn load_opcodes(opcodes_yaml: &str) -> anyhow::Result<Vec<Opcode>> {
    let res: Vec<Opcode> = serde_yaml::from_str(opcodes_yaml)?;
    Ok(res)
}
