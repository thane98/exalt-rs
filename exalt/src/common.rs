use std::io::Cursor;

use byteorder::ReadBytesExt;
use encoding_rs::SHIFT_JIS;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Debug)]
pub enum EventArgType {
    Str,
    Int,
    Float,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum EventArg {
    Int(i32),
    Float(f32),
    Str(String),
}

#[derive(Debug, Clone, Copy, EnumString)]
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
    StringEquals,
    StringNotEquals,
    Return,
    Nop0x40,
    Assign,
}

#[derive(Debug)]
pub struct RawFunctionHeader {
    pub name_address: Option<u32>,
    pub code_address: u32,
    pub parent_address: Option<u32>,
    pub args_address: Option<u32>,
    pub frame_size: u16,
    pub function_type: u8,
    pub arity: u8,
    pub param_count: u8,

    // Unknown byte in FE9/FE10 after type/arity/param_count
    pub unknown: u8,

    // Appears to be "junk" data between the name's null terminator and opcodes.
    // Only shows up in FE9/FE10
    pub unknown_prefix: Vec<u8>,

    // Appears to be "junk" data between the terminating opcode and the next word.
    // Only shows up in FE9/FE10
    pub unknown_suffix: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct FunctionData {
    pub function_type: u8,
    pub arity: u8,
    pub frame_size: usize,
    pub unknown: u8,
    pub unknown_prefix: Vec<u8>,
    pub unknown_suffix: Vec<u8>,
    pub name: Option<String>,
    pub args: Vec<EventArg>,
    pub code: Vec<Opcode>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrettyFunctionData {
    pub function_type: u8,
    pub arity: u8,
    pub frame_size: usize,
    pub unknown: u8,
    pub name: Option<String>,
    pub args: Vec<EventArg>,
    pub code: Vec<String>,
    pub unknown_prefix: Vec<u8>,
    pub unknown_suffix: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Script {
    pub script_type: u32,
    pub functions: Vec<FunctionData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrettyScript {
    pub script_type: u32,
    pub functions: Vec<PrettyFunctionData>,
}

pub fn load_opcodes(opcodes_yaml: &str) -> anyhow::Result<Vec<Opcode>> {
    let res: Vec<Opcode> = serde_yaml::from_str(opcodes_yaml)?;
    Ok(res)
}

pub fn read_shift_jis_string(data: &[u8], start: usize) -> anyhow::Result<String> {
    if start > data.len() {
        return Err(anyhow::anyhow!("Out of bounds text pointer."));
    }
    let mut end = start;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    if start == end {
        Ok(String::new())
    } else {
        let (v, _, failure) = SHIFT_JIS.decode(&data[start..end]);
        if !failure {
            Ok(v.to_string())
        } else {
            Err(anyhow::anyhow!(
                "Malformed shift-jis sequence addr={:X}",
                start
            ))
        }
    }
}

pub fn read_shift_jis_from_cursor(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<String> {
    let start = cursor.position();
    let mut buffer = Vec::new();
    let mut next = cursor.read_u8()?;
    while next != 0 {
        buffer.push(next);
        next = cursor.read_u8()?;
    }
    let (v, _, failure) = SHIFT_JIS.decode(&buffer);
    if !failure {
        Ok(v.to_string())
    } else {
        Err(anyhow::anyhow!(
            "Malformed shift-jis sequence addr={:X}",
            start
        ))
    }
}

pub fn encode_shift_jis(text: &str) -> anyhow::Result<Vec<u8>> {
    let (bytes, _, errors) = SHIFT_JIS.encode(text);
    if errors {
        return Err(anyhow::anyhow!(
            "Failed to encode string '{}' as SHIFT-JIS.",
            text
        ));
    }
    Ok(bytes.into())
}
