use anyhow::{anyhow, Result};

use crate::Opcode;

use super::CodeGenState;

pub trait Assembler {
    fn to_bytes(opcode: &Opcode, bytes: &mut Vec<u8>, state: &mut CodeGenState) -> Result<()>;
}

pub struct V1Assembler;
pub struct V2Assembler;
pub struct V3Assembler;

pub fn write_byte_or_short(out: &mut Vec<u8>, value: u16, byte_opcode: u8, short_opcode: u8) {
    if value <= 0x7F {
        out.push(byte_opcode);
        out.push(value as u8);
    } else {
        out.push(short_opcode);
        out.extend(value.to_be_bytes().iter());
    }
}

pub fn write_byte_or_short_or_int(
    out: &mut Vec<u8>,
    value: u32,
    byte_opcode: u8,
    short_opcode: u8,
    int_opcode: u8,
) {
    if value <= 0x7F {
        out.push(byte_opcode);
        out.push(value as u8);
    } else if value <= 0x7FFF {
        out.push(short_opcode);
        out.extend((value as u16).to_be_bytes().iter());
    } else {
        out.push(int_opcode);
        out.extend(value.to_be_bytes().iter());
    }
}

pub fn write_jump(
    output: &mut Vec<u8>,
    state: &mut CodeGenState,
    label: &str,
    jump_addr: usize,
    opcode: u8,
) {
    // Can't always know the jump length yet, so don't try to figure it out.
    // We will note the location and backpatch later.
    output.push(opcode);
    state.add_jump(label, jump_addr);
    output.push(0);
    output.push(0);
}

impl Assembler for V1Assembler {
    fn to_bytes(opcode: &Opcode, bytes: &mut Vec<u8>, state: &mut CodeGenState) -> Result<()> {
        let addr = bytes.len();
        match opcode {
            Opcode::Done => bytes.push(0),
            Opcode::VarLoad(v) => write_byte_or_short(bytes, *v, 0x1, 0x2),
            Opcode::ArrLoad(v) => write_byte_or_short(bytes, *v, 0x3, 0x4),
            Opcode::PtrLoad(v) => write_byte_or_short(bytes, *v, 0x5, 0x6),
            Opcode::VarAddr(v) => write_byte_or_short(bytes, *v, 0x7, 0x8),
            Opcode::ArrAddr(v) => write_byte_or_short(bytes, *v, 0x9, 0xA),
            Opcode::PtrAddr(v) => write_byte_or_short(bytes, *v, 0xB, 0xC),
            Opcode::GlobalVarLoad(v) => write_byte_or_short(bytes, *v, 0xD, 0xE),
            Opcode::GlobalArrLoad(v) => write_byte_or_short(bytes, *v, 0xF, 0x10),
            Opcode::GlobalPtrLoad(v) => write_byte_or_short(bytes, *v, 0x11, 0x12),
            Opcode::GlobalVarAddr(v) => write_byte_or_short(bytes, *v, 0x13, 0x14),
            Opcode::GlobalArrAddr(v) => write_byte_or_short(bytes, *v, 0x15, 0x16),
            Opcode::GlobalPtrAddr(v) => write_byte_or_short(bytes, *v, 0x17, 0x18),
            Opcode::IntLoad(v) => {
                if *v >= i8::MIN as i32 && *v <= i8::MAX as i32 {
                    bytes.push(0x19);
                    bytes.extend((*v as i8).to_be_bytes().iter());
                } else if *v >= i16::MIN as i32 && *v <= i16::MAX as i32 {
                    bytes.push(0x1A);
                    bytes.extend((*v as i16).to_be_bytes().iter());
                } else {
                    bytes.push(0x1B);
                    bytes.extend((*v).to_be_bytes().iter());
                }
            }
            Opcode::StrLoad(v) => {
                let offset = state.text_data.offset(v)?;
                write_byte_or_short_or_int(bytes, offset as u32, 0x1C, 0x1D, 0x1E);
            }
            Opcode::Dereference => bytes.push(0x1F),
            Opcode::Consume => bytes.push(0x20),
            Opcode::CompleteAssign => bytes.push(0x21),
            Opcode::Add => bytes.push(0x22),
            Opcode::Subtract => bytes.push(0x23),
            Opcode::Multiply => bytes.push(0x24),
            Opcode::Divide => bytes.push(0x25),
            Opcode::Modulo => bytes.push(0x26),
            Opcode::IntNegate => bytes.push(0x27),
            Opcode::BinaryNot => bytes.push(0x28),
            Opcode::LogicalNot => bytes.push(0x29),
            Opcode::BinaryOr => bytes.push(0x2A),
            Opcode::BinaryAnd => bytes.push(0x2B),
            Opcode::Xor => bytes.push(0x2C),
            Opcode::LeftShift => bytes.push(0x2D),
            Opcode::RightShift => bytes.push(0x2E),
            Opcode::Equal => bytes.push(0x2F),
            Opcode::NotEqual => bytes.push(0x30),
            Opcode::LessThan => bytes.push(0x31),
            Opcode::LessThanEqualTo => bytes.push(0x32),
            Opcode::GreaterThan => bytes.push(0x33),
            Opcode::GreaterThanEqualTo => bytes.push(0x34),
            Opcode::StringEquals => bytes.push(0x35),
            Opcode::StringNotEquals => bytes.push(0x36),
            Opcode::CallById(v) => {
                // TODO: Does v1 support 16-bit call IDs?
                bytes.push(0x37);
                bytes.push(*v as u8);
            }
            Opcode::CallByName(n, c) => {
                let name_offset = state.text_data.offset(n)? as u16;
                bytes.push(0x38);
                bytes.extend(name_offset.to_be_bytes().iter());
                bytes.push(*c);
            }
            Opcode::Return => bytes.push(0x39),
            Opcode::Jump(v) => write_jump(bytes, state, v, addr + 1, 0x3A),
            Opcode::JumpNotZero(v) => write_jump(bytes, state, v, addr + 1, 0x3B),
            Opcode::Or(v) => write_jump(bytes, state, v, addr + 1, 0x3C),
            Opcode::JumpZero(v) => write_jump(bytes, state, v, addr + 1, 0x3D),
            Opcode::And(v) => write_jump(bytes, state, v, addr + 1, 0x3E),
            Opcode::Yield => bytes.push(0x3F),
            Opcode::Nop0x40 => bytes.push(0x40),
            Opcode::Format(v) => {
                bytes.push(0x41);
                bytes.push(*v);
            }
            Opcode::Label(l) => state.add_label(l, addr)?,
            _ => return Err(anyhow!("Unsupported VGCN opcode {:?}", opcode)),
        }
        Ok(())
    }
}

impl Assembler for V2Assembler {
    fn to_bytes(opcode: &Opcode, bytes: &mut Vec<u8>, state: &mut CodeGenState) -> Result<()> {
        let addr = bytes.len();
        match opcode {
            Opcode::Done => bytes.push(0),
            Opcode::VarLoad(v) => write_byte_or_short(bytes, *v, 0x1, 0x2),
            Opcode::ArrLoad(v) => write_byte_or_short(bytes, *v, 0x3, 0x4),
            Opcode::PtrLoad(v) => write_byte_or_short(bytes, *v, 0x5, 0x6),
            Opcode::VarAddr(v) => write_byte_or_short(bytes, *v, 0x7, 0x8),
            Opcode::ArrAddr(v) => write_byte_or_short(bytes, *v, 0x9, 0xA),
            Opcode::PtrAddr(v) => write_byte_or_short(bytes, *v, 0xB, 0xC),
            Opcode::GlobalVarLoad(v) => write_byte_or_short(bytes, *v, 0xD, 0xE),
            Opcode::GlobalArrLoad(v) => write_byte_or_short(bytes, *v, 0xF, 0x10),
            Opcode::GlobalPtrLoad(v) => write_byte_or_short(bytes, *v, 0x11, 0x12),
            Opcode::GlobalVarAddr(v) => write_byte_or_short(bytes, *v, 0x13, 0x14),
            Opcode::GlobalArrAddr(v) => write_byte_or_short(bytes, *v, 0x15, 0x16),
            Opcode::GlobalPtrAddr(v) => write_byte_or_short(bytes, *v, 0x17, 0x18),
            Opcode::IntLoad(v) => {
                if *v >= i8::MIN as i32 && *v <= i8::MAX as i32 {
                    bytes.push(0x19);
                    bytes.extend((*v as i8).to_be_bytes().iter());
                } else if *v >= i16::MIN as i32 && *v <= i16::MAX as i32 {
                    bytes.push(0x1A);
                    bytes.extend((*v as i16).to_be_bytes().iter());
                } else {
                    bytes.push(0x1B);
                    bytes.extend((*v).to_be_bytes().iter());
                }
            }
            Opcode::StrLoad(v) => {
                let offset = state.text_data.offset(v)?;
                write_byte_or_short_or_int(bytes, offset as u32, 0x1C, 0x1D, 0x1E);
            }
            Opcode::Dereference => bytes.push(0x1F),
            Opcode::Consume => bytes.push(0x20),
            Opcode::CompleteAssign => bytes.push(0x21),
            Opcode::Add => bytes.push(0x22),
            Opcode::Subtract => bytes.push(0x23),
            Opcode::Multiply => bytes.push(0x24),
            Opcode::Divide => bytes.push(0x25),
            Opcode::Modulo => bytes.push(0x26),
            Opcode::IntNegate => bytes.push(0x27),
            Opcode::BinaryNot => bytes.push(0x28),
            Opcode::LogicalNot => bytes.push(0x29),
            Opcode::BinaryOr => bytes.push(0x2A),
            Opcode::BinaryAnd => bytes.push(0x2B),
            Opcode::Xor => bytes.push(0x2C),
            Opcode::LeftShift => bytes.push(0x2D),
            Opcode::RightShift => bytes.push(0x2E),
            Opcode::Equal => bytes.push(0x2F),
            Opcode::NotEqual => bytes.push(0x30),
            Opcode::LessThan => bytes.push(0x31),
            Opcode::LessThanEqualTo => bytes.push(0x32),
            Opcode::GreaterThan => bytes.push(0x33),
            Opcode::GreaterThanEqualTo => bytes.push(0x34),
            Opcode::StringEquals => bytes.push(0x35),
            Opcode::StringNotEquals => bytes.push(0x36),
            Opcode::CallById(v) => {
                bytes.push(0x37);
                if *v <= 0x7F {
                    bytes.push(*v as u8);
                } else {
                    let v = (1 << 15) | (*v as u16);
                    bytes.extend(v.to_be_bytes().iter());
                }
            }
            Opcode::CallByName(n, c) => {
                let name_offset = state.text_data.offset(n)? as u16;
                bytes.push(0x38);
                bytes.extend(name_offset.to_be_bytes().iter());
                bytes.push(*c);
            }
            Opcode::Return => bytes.push(0x39),
            Opcode::Jump(v) => write_jump(bytes, state, v, addr + 1, 0x3A),
            Opcode::JumpNotZero(v) => write_jump(bytes, state, v, addr + 1, 0x3B),
            Opcode::Or(v) => write_jump(bytes, state, v, addr + 1, 0x3C),
            Opcode::JumpZero(v) => write_jump(bytes, state, v, addr + 1, 0x3D),
            Opcode::And(v) => write_jump(bytes, state, v, addr + 1, 0x3E),
            Opcode::Yield => bytes.push(0x3F),
            Opcode::Nop0x40 => bytes.push(0x40),
            Opcode::Format(v) => {
                bytes.push(0x41);
                bytes.push(*v);
            }
            Opcode::Inc => bytes.push(0x42),
            Opcode::Dec => bytes.push(0x43),
            Opcode::Copy => bytes.push(0x44),
            Opcode::ReturnFalse => bytes.push(0x45),
            Opcode::ReturnTrue => bytes.push(0x46),
            Opcode::Assign => bytes.push(0x47),
            Opcode::Label(l) => state.add_label(l, addr)?,
            _ => return Err(anyhow!("Unsupported VGCN opcode {:?}", opcode)),
        }
        Ok(())
    }
}

impl Assembler for V3Assembler {
    fn to_bytes(opcode: &Opcode, bytes: &mut Vec<u8>, state: &mut CodeGenState) -> Result<()> {
        let addr = bytes.len();
        match opcode {
            Opcode::Done => bytes.push(0),
            Opcode::VarLoad(v) => write_byte_or_short(bytes, *v, 0x1, 0x2),
            Opcode::ArrLoad(v) => write_byte_or_short(bytes, *v, 0x3, 0x4),
            Opcode::PtrLoad(v) => write_byte_or_short(bytes, *v, 0x5, 0x6),
            Opcode::VarAddr(v) => write_byte_or_short(bytes, *v, 0x7, 0x8),
            Opcode::ArrAddr(v) => write_byte_or_short(bytes, *v, 0x9, 0xA),
            Opcode::PtrAddr(v) => write_byte_or_short(bytes, *v, 0xB, 0xC),
            Opcode::IntLoad(v) => {
                if *v >= i8::MIN as i32 && *v <= i8::MAX as i32 {
                    bytes.push(0x19);
                    bytes.extend((*v as i8).to_be_bytes().iter());
                } else if *v >= i16::MIN as i32 && *v <= i16::MAX as i32 {
                    bytes.push(0x1A);
                    bytes.extend((*v as i16).to_be_bytes().iter());
                } else {
                    bytes.push(0x1B);
                    bytes.extend((*v).to_be_bytes().iter());
                }
            }
            Opcode::StrLoad(v) => {
                let offset = state.text_data.offset(v)?;
                write_byte_or_short_or_int(bytes, offset as u32, 0x1C, 0x1D, 0x1E);
            }
            Opcode::FloatLoad(v) => {
                bytes.push(0x1F);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            Opcode::Dereference => bytes.push(0x20),
            Opcode::Consume => bytes.push(0x21),
            Opcode::CompleteAssign => bytes.push(0x23),
            Opcode::Fix => bytes.push(0x24),
            Opcode::Float => bytes.push(0x25),
            Opcode::Add => bytes.push(0x26),
            Opcode::FloatAdd => bytes.push(0x27),
            Opcode::Subtract => bytes.push(0x28),
            Opcode::FloatSubtract => bytes.push(0x29),
            Opcode::Multiply => bytes.push(0x2A),
            Opcode::FloatMultiply => bytes.push(0x2B),
            Opcode::Divide => bytes.push(0x2C),
            Opcode::FloatDivide => bytes.push(0x2D),
            Opcode::Modulo => bytes.push(0x2E),
            Opcode::IntNegate => bytes.push(0x2F),
            Opcode::FloatNegate => bytes.push(0x30),
            Opcode::BinaryNot => bytes.push(0x31),
            Opcode::LogicalNot => bytes.push(0x32),
            Opcode::BinaryOr => bytes.push(0x33),
            Opcode::BinaryAnd => bytes.push(0x34),
            Opcode::Xor => bytes.push(0x35),
            Opcode::LeftShift => bytes.push(0x36),
            Opcode::RightShift => bytes.push(0x37),
            Opcode::Equal => bytes.push(0x38),
            Opcode::FloatEqual => bytes.push(0x39),
            Opcode::Exlcall => todo!(),
            Opcode::NotEqual => bytes.push(0x3B),
            Opcode::FloatNotEqual => bytes.push(0x3C),
            Opcode::Nop0x3D => bytes.push(0x3D),
            Opcode::LessThan => bytes.push(0x3E),
            Opcode::FloatLessThan => bytes.push(0x3F),
            Opcode::LessThanEqualTo => bytes.push(0x40),
            Opcode::FloatLessThanEqualTo => bytes.push(0x41),
            Opcode::GreaterThan => bytes.push(0x42),
            Opcode::FloatGreaterThan => bytes.push(0x43),
            Opcode::GreaterThanEqualTo => bytes.push(0x44),
            Opcode::FloatGreaterThanEqualTo => bytes.push(0x45),
            Opcode::CallById(v) => {
                bytes.push(0x46);
                if *v <= 0x7F {
                    bytes.push(*v as u8);
                } else {
                    let v = (1 << 15) | (*v as u16);
                    bytes.extend(v.to_be_bytes().iter());
                }
            }
            Opcode::CallByName(n, c) => {
                let name_offset = state.text_data.offset(n)? as u16;
                bytes.push(0x47);
                bytes.extend(name_offset.to_be_bytes().iter());
                bytes.push(*c);
            }
            Opcode::SetReturn => bytes.push(0x48),
            Opcode::Jump(v) => write_jump(bytes, state, v, addr + 1, 0x49),
            Opcode::JumpNotZero(v) => write_jump(bytes, state, v, addr + 1, 0x4A),
            Opcode::Or(v) => write_jump(bytes, state, v, addr + 1, 0x4B),
            Opcode::JumpZero(v) => write_jump(bytes, state, v, addr + 1, 0x4C),
            Opcode::And(v) => write_jump(bytes, state, v, addr + 1, 0x4D),
            Opcode::Yield => bytes.push(0x4E),
            Opcode::Format(v) => {
                bytes.push(0x50);
                bytes.push(*v);
            }
            Opcode::Inc => bytes.push(0x51),
            Opcode::Dec => bytes.push(0x52),
            Opcode::Copy => bytes.push(0x53),
            Opcode::ReturnFalse => bytes.push(0x54),
            Opcode::ReturnTrue => bytes.push(0x55),
            Opcode::Label(l) => state.add_label(l, addr)?,
            _ => return Err(anyhow::anyhow!("Unsupported V3DS opcode {:?}", opcode)),
        }
        Ok(())
    }
}
