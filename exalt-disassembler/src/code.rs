use anyhow::{Result, Context, bail};
use byteorder::{BigEndian, ReadBytesExt};
use exalt_lir::{Opcode, Game};
use std::io::Cursor;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::util::read_shift_jis;

struct ResolveState<'a> {
    pub text_data: &'a [u8],
    pub labels: FxHashMap<u64, String>,
    next_label: usize,
}

impl<'a> ResolveState<'a> {
    pub fn new(text_data: &'a [u8]) -> Self {
        ResolveState {
            text_data,
            labels: FxHashMap::default(),
            next_label: 0,
        }
    }

    pub fn label(&mut self, addr: u64) -> String {
        match self.labels.get(&addr) {
            Some(l) => l.clone(),
            None => {
                let label = format!("l{}", self.next_label);
                self.next_label += 1;
                self.labels.insert(addr, label.clone());
                label
            }
        }
    }

    pub fn text(&self, offset: u64) -> anyhow::Result<String> {
        read_shift_jis(self.text_data, offset)
    }
}

fn calculate_jump_address(addr: u64, diff: i16) -> u64 {
    ((addr as i64) + (diff as i64) + 1) as u64
}

fn read_gcn_opcode(
    cursor: &mut Cursor<&[u8]>,
    state: &mut ResolveState,
) -> Result<(u64, Opcode)> {
    let addr = cursor.position();
    let opcode = cursor.read_u8()?;
    match opcode {
        0x0 => Ok((addr, Opcode::Done)),
        0x1 => Ok((addr, Opcode::VarLoad(cursor.read_u8()? as u16))),
        0x2 => Ok((addr, Opcode::VarLoad(cursor.read_u16::<BigEndian>()?))),
        0x3 => Ok((addr, Opcode::ArrLoad(cursor.read_u8()? as u16))),
        0x4 => Ok((addr, Opcode::ArrLoad(cursor.read_u16::<BigEndian>()?))),
        0x5 => Ok((addr, Opcode::PtrLoad(cursor.read_u8()? as u16))),
        0x6 => Ok((addr, Opcode::PtrLoad(cursor.read_u16::<BigEndian>()?))),
        0x7 => Ok((addr, Opcode::VarAddr(cursor.read_u8()? as u16))),
        0x8 => Ok((addr, Opcode::VarAddr(cursor.read_u16::<BigEndian>()?))),
        0x9 => Ok((addr, Opcode::ArrAddr(cursor.read_u8()? as u16))),
        0xA => Ok((addr, Opcode::ArrAddr(cursor.read_u16::<BigEndian>()?))),
        0xB => Ok((addr, Opcode::PtrAddr(cursor.read_u8()? as u16))),
        0xC => Ok((addr, Opcode::PtrAddr(cursor.read_u16::<BigEndian>()?))),
        0xD => Ok((addr, Opcode::GlobalVarLoad(cursor.read_u8()? as u16))),
        0xE => Ok((addr, Opcode::GlobalVarLoad(cursor.read_u16::<BigEndian>()?))),
        0xF => Ok((addr, Opcode::GlobalArrLoad(cursor.read_u8()? as u16))),
        0x10 => Ok((addr, Opcode::GlobalArrLoad(cursor.read_u16::<BigEndian>()?))),
        0x11 => Ok((addr, Opcode::GlobalPtrLoad(cursor.read_u8()? as u16))),
        0x12 => Ok((addr, Opcode::GlobalPtrLoad(cursor.read_u16::<BigEndian>()?))),
        0x13 => Ok((addr, Opcode::GlobalVarAddr(cursor.read_u8()? as u16))),
        0x14 => Ok((addr, Opcode::GlobalVarAddr(cursor.read_u16::<BigEndian>()?))),
        0x15 => Ok((addr, Opcode::GlobalArrAddr(cursor.read_u8()? as u16))),
        0x16 => Ok((addr, Opcode::GlobalArrAddr(cursor.read_u16::<BigEndian>()?))),
        0x17 => Ok((addr, Opcode::GlobalPtrAddr(cursor.read_u8()? as u16))),
        0x18 => Ok((addr, Opcode::GlobalPtrAddr(cursor.read_u16::<BigEndian>()?))),
        0x19 => Ok((addr, Opcode::IntLoad(cursor.read_i8()? as i32))),
        0x1A => Ok((
            addr,
            Opcode::IntLoad(cursor.read_i16::<BigEndian>()? as i32),
        )),
        0x1B => Ok((addr, Opcode::IntLoad(cursor.read_i32::<BigEndian>()?))),
        0x1C => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u8()? as u64)?),
        )),
        0x1D => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u16::<BigEndian>()? as u64)?),
        )),
        0x1E => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u32::<BigEndian>()? as u64)?),
        )),
        0x1F => Ok((addr, Opcode::Dereference)),
        0x20 => Ok((addr, Opcode::Consume)),
        0x21 => Ok((addr, Opcode::CompleteAssign)),
        0x22 => Ok((addr, Opcode::Add)),
        0x23 => Ok((addr, Opcode::Subtract)),
        0x24 => Ok((addr, Opcode::Multiply)),
        0x25 => Ok((addr, Opcode::Divide)),
        0x26 => Ok((addr, Opcode::Modulo)),
        0x27 => Ok((addr, Opcode::IntNegate)),
        0x28 => Ok((addr, Opcode::BinaryNot)),
        0x29 => Ok((addr, Opcode::LogicalNot)),
        0x2A => Ok((addr, Opcode::BinaryOr)),
        0x2B => Ok((addr, Opcode::BinaryAnd)),
        0x2C => Ok((addr, Opcode::Xor)),
        0x2D => Ok((addr, Opcode::LeftShift)),
        0x2E => Ok((addr, Opcode::RightShift)),
        0x2F => Ok((addr, Opcode::Equal)),
        0x30 => Ok((addr, Opcode::NotEqual)),
        0x31 => Ok((addr, Opcode::LessThan)),
        0x32 => Ok((addr, Opcode::LessThanEqualTo)),
        0x33 => Ok((addr, Opcode::GreaterThan)),
        0x34 => Ok((addr, Opcode::GreaterThanEqualTo)),
        0x35 => Ok((addr, Opcode::StringEquals)),
        0x36 => Ok((addr, Opcode::StringNotEquals)),
        0x37 => Ok((addr, Opcode::CallById(cursor.read_u8()? as usize))),
        0x38 => Ok((
            addr,
            Opcode::CallByName(
                state.text(cursor.read_u16::<BigEndian>()? as u64)?,
                cursor.read_u8()?,
            ),
        )),
        0x39 => Ok((addr, Opcode::Return)),
        0x3A => Ok((
            addr,
            Opcode::Jump(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3B => Ok((
            addr,
            Opcode::JumpNotZero(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3C => Ok((
            addr,
            Opcode::Or(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3D => Ok((
            addr,
            Opcode::JumpZero(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3E => Ok((
            addr,
            Opcode::And(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3F => Ok((addr, Opcode::Yield)),
        0x40 => Ok((addr, Opcode::Nop0x40)),
        0x41 => Ok((addr, Opcode::Format(cursor.read_u8()?))),
        _ => bail!("unrecognized opcode 0x{:X}", opcode),
    }
}

fn read_wii_opcode(
    cursor: &mut Cursor<&[u8]>,
    state: &mut ResolveState,
) -> Result<(u64, Opcode)> {
    let addr = cursor.position();
    let opcode = cursor.read_u8()?;
    match opcode {
        0x0 => Ok((addr, Opcode::Done)),
        0x1 => Ok((addr, Opcode::VarLoad(cursor.read_u8()? as u16))),
        0x2 => Ok((addr, Opcode::VarLoad(cursor.read_u16::<BigEndian>()?))),
        0x3 => Ok((addr, Opcode::ArrLoad(cursor.read_u8()? as u16))),
        0x4 => Ok((addr, Opcode::ArrLoad(cursor.read_u16::<BigEndian>()?))),
        0x5 => Ok((addr, Opcode::PtrLoad(cursor.read_u8()? as u16))),
        0x6 => Ok((addr, Opcode::PtrLoad(cursor.read_u16::<BigEndian>()?))),
        0x7 => Ok((addr, Opcode::VarAddr(cursor.read_u8()? as u16))),
        0x8 => Ok((addr, Opcode::VarAddr(cursor.read_u16::<BigEndian>()?))),
        0x9 => Ok((addr, Opcode::ArrAddr(cursor.read_u8()? as u16))),
        0xA => Ok((addr, Opcode::ArrAddr(cursor.read_u16::<BigEndian>()?))),
        0xB => Ok((addr, Opcode::PtrAddr(cursor.read_u8()? as u16))),
        0xC => Ok((addr, Opcode::PtrAddr(cursor.read_u16::<BigEndian>()?))),
        0xD => Ok((addr, Opcode::GlobalVarLoad(cursor.read_u8()? as u16))),
        0xE => Ok((addr, Opcode::GlobalVarLoad(cursor.read_u16::<BigEndian>()?))),
        0xF => Ok((addr, Opcode::GlobalArrLoad(cursor.read_u8()? as u16))),
        0x10 => Ok((addr, Opcode::GlobalArrLoad(cursor.read_u16::<BigEndian>()?))),
        0x11 => Ok((addr, Opcode::GlobalPtrLoad(cursor.read_u8()? as u16))),
        0x12 => Ok((addr, Opcode::GlobalPtrLoad(cursor.read_u16::<BigEndian>()?))),
        0x13 => Ok((addr, Opcode::GlobalVarAddr(cursor.read_u8()? as u16))),
        0x14 => Ok((addr, Opcode::GlobalVarAddr(cursor.read_u16::<BigEndian>()?))),
        0x15 => Ok((addr, Opcode::GlobalArrAddr(cursor.read_u8()? as u16))),
        0x16 => Ok((addr, Opcode::GlobalArrAddr(cursor.read_u16::<BigEndian>()?))),
        0x17 => Ok((addr, Opcode::GlobalPtrAddr(cursor.read_u8()? as u16))),
        0x18 => Ok((addr, Opcode::GlobalPtrAddr(cursor.read_u16::<BigEndian>()?))),
        0x19 => Ok((addr, Opcode::IntLoad(cursor.read_i8()? as i32))),
        0x1A => Ok((
            addr,
            Opcode::IntLoad(cursor.read_i16::<BigEndian>()? as i32),
        )),
        0x1B => Ok((addr, Opcode::IntLoad(cursor.read_i32::<BigEndian>()?))),
        0x1C => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u8()? as u64)?),
        )),
        0x1D => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u16::<BigEndian>()? as u64)?),
        )),
        0x1E => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u32::<BigEndian>()? as u64)?),
        )),
        0x1F => Ok((addr, Opcode::Dereference)),
        0x20 => Ok((addr, Opcode::Consume)),
        0x21 => Ok((addr, Opcode::CompleteAssign)),
        0x22 => Ok((addr, Opcode::Add)),
        0x23 => Ok((addr, Opcode::Subtract)),
        0x24 => Ok((addr, Opcode::Multiply)),
        0x25 => Ok((addr, Opcode::Divide)),
        0x26 => Ok((addr, Opcode::Modulo)),
        0x27 => Ok((addr, Opcode::IntNegate)),
        0x28 => Ok((addr, Opcode::BinaryNot)),
        0x29 => Ok((addr, Opcode::LogicalNot)),
        0x2A => Ok((addr, Opcode::BinaryOr)),
        0x2B => Ok((addr, Opcode::BinaryAnd)),
        0x2C => Ok((addr, Opcode::Xor)),
        0x2D => Ok((addr, Opcode::LeftShift)),
        0x2E => Ok((addr, Opcode::RightShift)),
        0x2F => Ok((addr, Opcode::Equal)),
        0x30 => Ok((addr, Opcode::NotEqual)),
        0x31 => Ok((addr, Opcode::LessThan)),
        0x32 => Ok((addr, Opcode::LessThanEqualTo)),
        0x33 => Ok((addr, Opcode::GreaterThan)),
        0x34 => Ok((addr, Opcode::GreaterThanEqualTo)),
        0x35 => Ok((addr, Opcode::StringEquals)),
        0x36 => Ok((addr, Opcode::StringNotEquals)),
        0x37 => {
            let b1 = cursor.read_u8()?;
            let value = if (b1 & 0x80) != 0 {
                ((b1 as u16 & 0x7F) << 8) | cursor.read_u8()? as u16
            } else {
                b1 as u16
            };
            Ok((addr, Opcode::CallById(value as usize)))
        }
        0x38 => Ok((
            addr,
            Opcode::CallByName(
                state.text(cursor.read_u16::<BigEndian>()? as u64)?,
                cursor.read_u8()?,
            ),
        )),
        0x39 => Ok((addr, Opcode::Return)),
        0x3A => Ok((
            addr,
            Opcode::Jump(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3B => Ok((
            addr,
            Opcode::JumpNotZero(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3C => Ok((
            addr,
            Opcode::Or(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3D => Ok((
            addr,
            Opcode::JumpZero(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3E => Ok((
            addr,
            Opcode::And(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x3F => Ok((addr, Opcode::Yield)),
        0x40 => Ok((addr, Opcode::Nop0x40)),
        0x41 => Ok((addr, Opcode::Format(cursor.read_u8()?))),
        0x42 => Ok((addr, Opcode::Inc)),
        0x43 => Ok((addr, Opcode::Dec)),
        0x44 => Ok((addr, Opcode::Copy)),
        0x45 => Ok((addr, Opcode::ReturnFalse)),
        0x46 => Ok((addr, Opcode::ReturnTrue)),
        0x47 => Ok((addr, Opcode::Assign)),
        _ => bail!("unrecognized opcode 0x{:X}", opcode),
    }
}

fn read_three_ds_opcode(
    cursor: &mut Cursor<&[u8]>,
    state: &mut ResolveState,
) -> Result<(u64, Opcode)> {
    let addr = cursor.position();
    let opcode = cursor.read_u8()?;
    match opcode {
        0x0 => Ok((addr, Opcode::Done)),
        0x1 => Ok((addr, Opcode::VarLoad(cursor.read_u8()? as u16))),
        0x2 => Ok((addr, Opcode::VarLoad(cursor.read_u16::<BigEndian>()?))),
        0x3 => Ok((addr, Opcode::ArrLoad(cursor.read_u8()? as u16))),
        0x4 => Ok((addr, Opcode::ArrLoad(cursor.read_u16::<BigEndian>()?))),
        0x5 => Ok((addr, Opcode::PtrLoad(cursor.read_u8()? as u16))),
        0x6 => Ok((addr, Opcode::PtrLoad(cursor.read_u16::<BigEndian>()?))),
        0x7 => Ok((addr, Opcode::VarAddr(cursor.read_u8()? as u16))),
        0x8 => Ok((addr, Opcode::VarAddr(cursor.read_u16::<BigEndian>()?))),
        0x9 => Ok((addr, Opcode::ArrAddr(cursor.read_u8()? as u16))),
        0xA => Ok((addr, Opcode::ArrAddr(cursor.read_u16::<BigEndian>()?))),
        0xB => Ok((addr, Opcode::PtrAddr(cursor.read_u8()? as u16))),
        0xC => Ok((addr, Opcode::PtrAddr(cursor.read_u16::<BigEndian>()?))),
        0x19 => Ok((addr, Opcode::IntLoad(cursor.read_i8()? as i32))),
        0x1A => Ok((
            addr,
            Opcode::IntLoad(cursor.read_i16::<BigEndian>()? as i32),
        )),
        0x1B => Ok((addr, Opcode::IntLoad(cursor.read_i32::<BigEndian>()?))),
        0x1C => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u8()? as u64)?),
        )),
        0x1D => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u16::<BigEndian>()? as u64)?),
        )),
        0x1E => Ok((
            addr,
            Opcode::StrLoad(state.text(cursor.read_u32::<BigEndian>()? as u64)?),
        )),
        0x1F => Ok((addr, Opcode::FloatLoad(cursor.read_f32::<BigEndian>()?))),
        0x20 => Ok((addr, Opcode::Dereference)),
        0x21 => Ok((addr, Opcode::Consume)),
        0x23 => Ok((addr, Opcode::CompleteAssign)),
        0x24 => Ok((addr, Opcode::Fix)),
        0x25 => Ok((addr, Opcode::Float)),
        0x26 => Ok((addr, Opcode::Add)),
        0x27 => Ok((addr, Opcode::FloatAdd)),
        0x28 => Ok((addr, Opcode::Subtract)),
        0x29 => Ok((addr, Opcode::FloatSubtract)),
        0x2A => Ok((addr, Opcode::Multiply)),
        0x2B => Ok((addr, Opcode::FloatMultiply)),
        0x2C => Ok((addr, Opcode::Divide)),
        0x2D => Ok((addr, Opcode::FloatDivide)),
        0x2E => Ok((addr, Opcode::Modulo)),
        0x2F => Ok((addr, Opcode::IntNegate)),
        0x30 => Ok((addr, Opcode::FloatNegate)),
        0x31 => Ok((addr, Opcode::BinaryNot)),
        0x32 => Ok((addr, Opcode::LogicalNot)),
        0x33 => Ok((addr, Opcode::BinaryOr)),
        0x34 => Ok((addr, Opcode::BinaryAnd)),
        0x35 => Ok((addr, Opcode::Xor)),
        0x36 => Ok((addr, Opcode::LeftShift)),
        0x37 => Ok((addr, Opcode::RightShift)),
        0x38 => Ok((addr, Opcode::Equal)),
        0x39 => Ok((addr, Opcode::FloatEqual)),
        0x3B => Ok((addr, Opcode::NotEqual)),
        0x3C => Ok((addr, Opcode::FloatNotEqual)),
        0x3D => Ok((addr, Opcode::Nop0x3D)),
        0x3E => Ok((addr, Opcode::LessThan)),
        0x3F => Ok((addr, Opcode::FloatLessThan)),
        0x40 => Ok((addr, Opcode::LessThanEqualTo)),
        0x41 => Ok((addr, Opcode::FloatLessThanEqualTo)),
        0x42 => Ok((addr, Opcode::GreaterThan)),
        0x43 => Ok((addr, Opcode::FloatGreaterThan)),
        0x44 => Ok((addr, Opcode::GreaterThanEqualTo)),
        0x45 => Ok((addr, Opcode::FloatGreaterThanEqualTo)),
        0x46 => {
            let b1 = cursor.read_u8()?;
            let value = if (b1 & 0x80) != 0 {
                ((b1 as u16 & 0x7F) << 7) | cursor.read_u8()? as u16
            } else {
                b1 as u16
            };
            Ok((addr, Opcode::CallById(value as usize)))
        }
        0x47 => Ok((
            addr,
            Opcode::CallByName(
                state.text(cursor.read_u16::<BigEndian>()? as u64)?,
                cursor.read_u8()?,
            ),
        )),
        0x48 => Ok((addr, Opcode::Return)),
        0x49 => Ok((
            addr,
            Opcode::Jump(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x4A => Ok((
            addr,
            Opcode::JumpNotZero(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x4B => Ok((
            addr,
            Opcode::Or(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x4C => Ok((
            addr,
            Opcode::JumpZero(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x4D => Ok((
            addr,
            Opcode::And(state.label(calculate_jump_address(
                addr,
                cursor.read_i16::<BigEndian>()?,
            ))),
        )),
        0x4E => Ok((addr, Opcode::Yield)),
        0x50 => Ok((addr, Opcode::Format(cursor.read_u8()?))),
        0x51 => Ok((addr, Opcode::Inc)),
        0x52 => Ok((addr, Opcode::Dec)),
        0x53 => Ok((addr, Opcode::Copy)),
        0x54 => Ok((addr, Opcode::ReturnFalse)),
        0x55 => Ok((addr, Opcode::ReturnTrue)),
        _ => bail!("unrecognized opcode 0x{:X}", opcode),
    }
}

pub fn disassemble(cursor: &mut Cursor<&[u8]>, text_data: &[u8], game: Game) -> Result<Vec<Opcode>> {
    let disassembler = match game {
        Game::FE9 => read_gcn_opcode,
        Game::FE10 | Game::FE11 | Game::FE12 => read_wii_opcode,
        Game::FE13 | Game::FE14 | Game::FE15 => read_three_ds_opcode,
    };

    // First pass: just read the opcodes
    let mut state = ResolveState::new(text_data);
    let mut opcodes = Vec::new();
    loop {
        let (real_addr, raw_op) = disassembler(cursor, &mut state)
            .with_context(|| format!("failed to read opcode at '0x{:X}'", cursor.position()))?;
        match raw_op {
            Opcode::Done => break,
            _ => opcodes.push((real_addr, raw_op)),
        }
    }

    // Second pass: place labels
    let mut resolved_opcodes = Vec::new();
    let mut placed_labels = FxHashSet::default();
    for (addr, op) in opcodes {
        if let Some(label) = state.labels.get(&addr) {
            resolved_opcodes.push(Opcode::Label(label.to_owned()));
            placed_labels.insert(label);
        }
        resolved_opcodes.push(op);
    }

    // Sanity check: Did we place every label?
    let unplaced_labels: Vec<&str> = state
        .labels
        .values()
        .filter(|l| !placed_labels.contains(*l))
        .map(|l| l.as_str())
        .collect();
    if !unplaced_labels.is_empty() {
        bail!(
            "Failed to resolve the following jump positions: {}",
            unplaced_labels.join(", ")
        );
    }
    
    Ok(resolved_opcodes)
}
