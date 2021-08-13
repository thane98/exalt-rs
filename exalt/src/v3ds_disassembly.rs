use crate::common::read_shift_jis_string;
use crate::v3ds_common::{RawFunctionData, V3dsCmbHeader, FE14_EVENTS};
use crate::{EventArg, EventArgType, FunctionData, Opcode};
use anyhow::Context;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

struct ResolveState<'a> {
    text_data: &'a [u8],
    labels: HashMap<usize, String>,
    next_label: usize,
}

fn read_cmb_header(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<V3dsCmbHeader> {
    let magic_number = cursor.read_u32::<LittleEndian>()?;
    let revision = cursor.read_u32::<LittleEndian>()?;
    let unk1 = cursor.read_u32::<LittleEndian>()?;
    let script_name_address = cursor.read_u32::<LittleEndian>()?;
    let unk2 = cursor.read_u32::<LittleEndian>()?;
    let unk3 = cursor.read_u32::<LittleEndian>()?;
    let unk4 = cursor.read_u32::<LittleEndian>()?;
    let function_table_address = cursor.read_u32::<LittleEndian>()?;
    let text_data_address = cursor.read_u32::<LittleEndian>()?;
    let padding = cursor.read_u32::<LittleEndian>()?;
    Ok(V3dsCmbHeader {
        magic_number,
        revision,
        unk1,
        script_name_address,
        unk2,
        unk3,
        unk4,
        function_table_address,
        text_data_address,
        padding,
    })
}

fn read_function_table(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<Vec<usize>> {
    let mut addresses: Vec<usize> = Vec::new();
    let mut next = cursor.read_u32::<LittleEndian>()?;
    while next != 0 {
        addresses.push(next as usize);
        next = cursor.read_u32::<LittleEndian>()?;
    }
    Ok(addresses)
}

fn read_function_data(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<RawFunctionData> {
    let header_address = cursor.read_u32::<LittleEndian>()?;
    let code_address = cursor.read_u32::<LittleEndian>()?;
    let function_type = cursor.read_u8()?;
    let arity = cursor.read_u8()?;
    let frame_size = cursor.read_u16::<LittleEndian>()?;
    let id = cursor.read_u32::<LittleEndian>()?;
    let name_address = cursor.read_u32::<LittleEndian>()?;
    let args_address = cursor.read_u32::<LittleEndian>()?;
    Ok(RawFunctionData {
        header_address,
        code_address,
        function_type,
        arity,
        frame_size,
        id,
        name_address,
        args_address,
    })
}

fn read_function_args(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    function_type: u32,
    count: usize,
) -> anyhow::Result<Vec<EventArg>> {
    // TODO: Handle signatures for FE13 and FE15
    if function_type == 0 {
        Ok(Vec::new())
    } else {
        let mut args: Vec<EventArg> = Vec::new();
        match FE14_EVENTS.get(&function_type) {
            Some(signature) => {
                if count != signature.len() {
                    return Err(anyhow::anyhow!(
                        "Known signature and function header disagree on arity."
                    ));
                }
                for arg in signature {
                    match arg {
                        EventArgType::Str => {
                            let offset = cursor.read_u32::<LittleEndian>()? as usize;
                            let text = read_shift_jis_string(text_data, offset)?;
                            args.push(EventArg::Str(text));
                        }
                        EventArgType::Int => {
                            args.push(EventArg::Int(cursor.read_i32::<LittleEndian>()?));
                        }
                        EventArgType::Float => {
                            args.push(EventArg::Float(cursor.read_f32::<LittleEndian>()?));
                        }
                    }
                }
            }
            _ => {
                for _ in 0..count {
                    args.push(EventArg::Int(cursor.read_i32::<LittleEndian>()?));
                }
            }
        }
        Ok(args)
    }
}

fn disassemble_function(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
) -> anyhow::Result<Vec<Opcode>> {
    // First pass: Raw disassembly. Don't try to decode text or resolve jumps.
    let mut resolve_state = ResolveState::new(text_data);
    let mut ops: Vec<(usize, Opcode)> = Vec::new();
    loop {
        let (real_addr, raw_op) = Opcode::read_v3ds_opcode(cursor, &mut resolve_state)
            .context("Failed to read opcode.")?;
        match raw_op {
            Opcode::Done => break,
            _ => ops.push((real_addr, raw_op)),
        }
    }

    // Second pass: Place labels.
    let mut resolved_ops: Vec<Opcode> = Vec::new();
    let mut placed_labels: HashSet<String> = HashSet::new();
    for (addr, op) in ops {
        if let Some(label) = resolve_state.labels.get(&addr) {
            resolved_ops.push(Opcode::Label(label.to_owned()));
            placed_labels.insert(label.to_owned());
        }
        resolved_ops.push(op);
    }

    // Sanity check: Did we place every label?
    let unplaced_labels: Vec<String> = resolve_state
        .labels
        .values()
        .filter(|l| !placed_labels.contains(*l))
        .map(|l| l.to_owned())
        .collect();
    if !unplaced_labels.is_empty() {
        return Err(anyhow::anyhow!(
            "Failed to resolve the following jump positions: {}",
            unplaced_labels.join(", ")
        ));
    }

    Ok(resolved_ops)
}

fn read_function(
    data: &[u8],
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
) -> anyhow::Result<FunctionData> {
    let raw_function_data = read_function_data(cursor)?;
    let name = if raw_function_data.name_address != 0 {
        let name = read_shift_jis_string(data, raw_function_data.name_address as usize)
            .context("Failed to read function name.")?;
        Some(name)
    } else {
        None
    };
    let args = if raw_function_data.args_address != 0 {
        cursor.set_position(raw_function_data.args_address as u64);
        read_function_args(
            cursor,
            text_data,
            raw_function_data.function_type as u32,
            raw_function_data.arity as usize,
        )
        .context("Failed to read function args.")?
    } else {
        Vec::new()
    };
    let code = if raw_function_data.code_address != 0 {
        cursor.set_position(raw_function_data.code_address as u64);
        disassemble_function(cursor, text_data).context("Failed to disassemble function.")?
    } else {
        Vec::new()
    };
    Ok(FunctionData {
        function_type: raw_function_data.function_type,
        arity: raw_function_data.arity,
        frame_size: raw_function_data.frame_size as usize,
        name,
        args,
        code,
    })
}

fn calculate_jump_address(addr: usize, diff: i16) -> usize {
    ((addr as i64) + (diff as i64) + 1) as usize
}

pub fn disassemble(input: &[u8]) -> anyhow::Result<Vec<FunctionData>> {
    // First, read the file header.
    let mut cursor = Cursor::new(input);
    let header = read_cmb_header(&mut cursor).context("Failed to read CMB header.")?;
    header.validate()?;

    // Load text data.
    let text_data_address = header.text_data_address as usize;
    if text_data_address > input.len() {
        return Err(anyhow::anyhow!(
            "Text data location in header is out of bounds for the input data."
        ));
    }
    let text_data = &input[text_data_address..];

    // Read function addresses.
    let function_table_address = header.function_table_address as usize;
    if function_table_address >= input.len() {
        return Err(anyhow::anyhow!(
            "Function table location in header is out of bounds for the input data."
        ));
    }
    cursor.set_position(header.function_table_address as u64);
    let function_addresses =
        read_function_table(&mut cursor).context("Failed to read function table.")?;

    // Read function data.
    let mut functions: Vec<FunctionData> = Vec::new();
    for addr in function_addresses {
        cursor.set_position(addr as u64);
        let function = read_function(input, &mut cursor, text_data)
            .with_context(|| format!("Failed to read function at address '{:X}'.", addr))?;
        functions.push(function);
    }
    Ok(functions)
}

impl<'a> ResolveState<'a> {
    pub fn new(text_data: &'a [u8]) -> Self {
        ResolveState {
            text_data,
            labels: HashMap::new(),
            next_label: 0,
        }
    }

    pub fn label(&mut self, addr: usize) -> String {
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

    pub fn text(&self, offset: usize) -> anyhow::Result<String> {
        read_shift_jis_string(&self.text_data, offset)
    }
}

impl Opcode {
    fn read_v3ds_opcode(
        cursor: &mut Cursor<&[u8]>,
        state: &mut ResolveState,
    ) -> anyhow::Result<(usize, Opcode)> {
        let addr = cursor.position() as usize;
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
                Opcode::StrLoad(state.text(cursor.read_u8()? as usize)?),
            )),
            0x1D => Ok((
                addr,
                Opcode::StrLoad(state.text(cursor.read_u16::<BigEndian>()? as usize)?),
            )),
            0x1E => Ok((
                addr,
                Opcode::StrLoad(state.text(cursor.read_u32::<BigEndian>()? as usize)?),
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
            0x3A => Ok((addr, Opcode::Exlcall)),
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
            0x46 => Ok((addr, Opcode::CallById(cursor.read_u8()? as usize))),
            0x47 => Ok((
                addr,
                Opcode::CallByName(
                    state.text(cursor.read_u16::<BigEndian>()? as usize)?,
                    cursor.read_u8()?,
                ),
            )),
            0x48 => Ok((addr, Opcode::SetReturn)),
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
            _ => Err(anyhow::anyhow!("Unrecognized opcode {:X}", opcode)),
        }
    }
}
