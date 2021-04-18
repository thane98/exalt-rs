use crate::{Opcode, V3dsFunctionData};
use anyhow::Context;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::io::Cursor;

const FUNCTION_HEADER_SIZE: usize = 0x18;

struct CodeGenState<'a> {
    labels: HashMap<String, CodeGenLabelEntry>,
    text_data: &'a mut CodeGenTextData,
}

struct CodeGenLabelEntry {
    addr: Option<usize>,
    jumps: Vec<usize>,
}

struct CodeGenTextData {
    raw_text: Vec<u8>,
    offsets: HashMap<String, usize>,
}

fn encode_shift_jis(text: &str) -> anyhow::Result<Vec<u8>> {
    let (bytes, _, errors) = SHIFT_JIS.encode(text);
    if errors {
        println!("{:X?}", bytes);
        return Err(anyhow::anyhow!(
            "Failed to encode string '{}' as SHIFT-JIS.",
            text
        ));
    }
    Ok(bytes.into())
}

fn get_function_name_bytes(function: &V3dsFunctionData) -> anyhow::Result<Vec<u8>> {
    if let Some(name) = &function.name {
        if function.function_type == 0 && name.contains("::") {
            let mut bytes = encode_shift_jis(name)?;
            bytes.push(0);
            Ok(bytes)
        } else {
            Ok(Vec::new())
        }
    } else {
        Ok(Vec::new())
    }
}

fn get_function_arg_bytes(
    function: &V3dsFunctionData,
    text_data: &mut CodeGenTextData,
) -> anyhow::Result<Vec<u8>> {
    if function.function_type == 0 && !function.args.is_empty() {
        return Err(anyhow::anyhow!(
            "Function/event arguments cannot be used with function type 0."
        ));
    }
    let mut bytes: Vec<u8> = Vec::new();
    for arg in &function.args {
        match arg {
            crate::EventArg::Str(v) => {
                let offset = text_data.offset(v)? as u32;
                bytes.extend(offset.to_le_bytes().iter());
            }
            crate::EventArg::Int(v) => bytes.extend(v.to_le_bytes().iter()),
            crate::EventArg::Float(v) => bytes.extend(v.to_le_bytes().iter()),
        }
    }
    Ok(bytes)
}

fn gen_function_code(
    function: &V3dsFunctionData,
    function_id: u32,
    base_address: usize,
    text_data: &mut CodeGenTextData,
) -> anyhow::Result<Vec<u8>> {
    // Calculate addresses and serialize name / args.
    let name_bytes: Vec<u8> = get_function_name_bytes(function)
        .context("Failed to write function name.")?;
    let arg_bytes: Vec<u8> = get_function_arg_bytes(function, text_data)
        .context("Failed to write function arguments.")?;
    let extended_header_address = base_address + FUNCTION_HEADER_SIZE;
    let code_address = if name_bytes.is_empty() {
        extended_header_address + arg_bytes.len()
    } else {
        extended_header_address + name_bytes.len()
    };
    let name_address = if !name_bytes.is_empty() {
        extended_header_address
    } else {
        0
    };
    let args_address = if function.function_type == 0 {
        0
    } else {
        extended_header_address
    };

    // Generate the actual code.
    let mut code_gen_state = CodeGenState::new(text_data);
    let mut raw_code: Vec<u8> = Vec::new();
    for op in &function.code {
        op.to_v3ds_bytes(&mut raw_code, &mut code_gen_state)
            .context(format!("Failed to convert opcode to v3ds format: '{:?}'", op))?;
    }
    raw_code.push(0);
    code_gen_state.backpatch(&mut raw_code)?;

    // Combine everything and return.
    let mut raw: Vec<u8> = Vec::new();
    raw.extend((base_address as u32).to_le_bytes().iter());
    raw.extend((code_address as u32).to_le_bytes().iter());
    raw.push(function.function_type);
    raw.push(function.arity);
    raw.push(function.frame_size);
    raw.push(0); // Padding
    raw.extend(function_id.to_le_bytes().iter());
    raw.extend((name_address as u32).to_le_bytes().iter());
    raw.extend((args_address as u32).to_le_bytes().iter());
    raw.extend(name_bytes.into_iter());
    raw.extend(arg_bytes.into_iter());
    raw.extend(raw_code);
    Ok(raw)
}

pub fn gen_v3ds_code(script_name: &str, functions: &[V3dsFunctionData]) -> anyhow::Result<Vec<u8>> {
    gen_code(script_name, functions).context("Code generation failed.")
}

fn gen_code(script_name: &str, functions: &[V3dsFunctionData]) -> anyhow::Result<Vec<u8>> {
    // Write the header. Will need to revisit later to write function table and text data pointers.
    let name_bytes = encode_shift_jis(script_name)?;
    let mut raw: Vec<u8> = Vec::new();
    raw.extend((0x626D63 as u32).to_le_bytes().iter()); // Magic number
    raw.extend((0x20110819 as u32).to_le_bytes().iter()); // Revision number.
    raw.extend((0 as u32).to_le_bytes().iter());
    raw.extend((0x28 as u32).to_le_bytes().iter()); // Name pointer, always 0x28
    raw.resize(0x28, 0);
    raw.extend(name_bytes);
    raw.push(0);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }

    // Reserve space for the function table.
    let function_table_address = raw.len();
    for _ in 0..(functions.len() + 1) {
        raw.extend((0 as u32).to_le_bytes().iter());
    }

    // Write function data + text data.
    let mut text_data = CodeGenTextData::new();
    let mut function_addrs: Vec<u32> = Vec::new();
    for i in 0..functions.len() {
        // Pad.
        while raw.len() % 4 != 0 {
            raw.push(0);
        }

        // Write function data.
        let function = &functions[i];
        let base_address = raw.len();
        function_addrs.push(base_address as u32);
        raw.extend(gen_function_code(
            function,
            i as u32,
            base_address,
            &mut text_data,
        )?);
    }

    let text_data_address = raw.len();
    raw.extend_from_slice(text_data.bytes());
    while raw.len() % 4 != 0 {
        raw.push(0);
    }

    // Fix pointers.
    let mut cursor = Cursor::new(&mut raw);
    cursor.set_position(0x1C);
    cursor.write_u32::<LittleEndian>(function_table_address as u32)?;
    cursor.write_u32::<LittleEndian>(text_data_address as u32)?;
    cursor.set_position(function_table_address as u64);
    for i in 0..function_addrs.len() {
        cursor.write_u32::<LittleEndian>(function_addrs[i] as u32)?;
    }
    Ok(raw)
}

impl<'a> CodeGenState<'a> {
    pub fn new(text_data: &'a mut CodeGenTextData) -> Self {
        CodeGenState {
            labels: HashMap::new(),
            text_data,
        }
    }

    pub fn add_label(&mut self, label: &str, addr: usize) -> anyhow::Result<()> {
        match self.labels.get_mut(label) {
            Some(label_data) => match label_data.addr {
                Some(_) => return Err(anyhow::anyhow!("Duplicate entries for label '{}'.", label)),
                None => {
                    label_data.addr = Some(addr);
                }
            },
            None => {
                let label_data = CodeGenLabelEntry {
                    addr: Some(addr),
                    jumps: Vec::new(),
                };
                self.labels.insert(label.to_owned(), label_data);
            }
        }
        Ok(())
    }

    pub fn add_jump(&mut self, label: &str, jump_addr: usize) {
        match self.labels.get_mut(label) {
            Some(label_data) => label_data.jumps.push(jump_addr),
            None => {
                let label_data = CodeGenLabelEntry {
                    addr: None,
                    jumps: vec![jump_addr],
                };
                self.labels.insert(label.to_owned(), label_data);
            }
        }
    }

    pub fn backpatch(&self, bytes: &mut [u8]) -> anyhow::Result<()> {
        let mut cursor = Cursor::new(bytes);
        for (label, label_data) in &self.labels {
            match label_data.addr {
                Some(addr) => {
                    for jump in &label_data.jumps {
                        let signed_label_addr = addr as i16;
                        let signed_jump_addr = *jump as i16;
                        let diff = signed_label_addr - signed_jump_addr;
                        cursor.set_position(*jump as u64);
                        cursor.write_i16::<BigEndian>(diff)?;
                    }
                }
                None => return Err(anyhow::anyhow!("Unresolved label '{}'", label)),
            }
        }
        Ok(())
    }
}

impl CodeGenTextData {
    pub fn new() -> Self {
        CodeGenTextData {
            raw_text: Vec::new(),
            offsets: HashMap::new(),
        }
    }

    pub fn offset(&mut self, text: &str) -> anyhow::Result<usize> {
        match self.offsets.get(text) {
            Some(offset) => Ok(*offset),
            None => {
                let bytes = encode_shift_jis(text)?;
                let offset = self.raw_text.len();
                self.raw_text.extend(bytes.into_iter());
                self.raw_text.push(0);
                self.offsets.insert(text.to_owned(), offset);
                Ok(offset)
            }
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.raw_text
    }
}

fn write_byte_or_short(out: &mut Vec<u8>, value: u16, byte_opcode: u8, short_opcode: u8) {
    if value <= 0x7F {
        out.push(byte_opcode);
        out.push(value as u8);
    } else {
        out.push(short_opcode);
        out.extend(value.to_be_bytes().iter());
    }
}

fn write_byte_or_short_or_int(
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

fn write_jump(
    output: &mut Vec<u8>,
    state: &mut CodeGenState,
    label: &str,
    jump_addr: usize,
    opcode: u8,
) {
    output.push(opcode);
    state.add_jump(label, jump_addr);
    output.push(0);
    output.push(0);
}

impl Opcode {
    fn to_v3ds_bytes(&self, bytes: &mut Vec<u8>, state: &mut CodeGenState) -> anyhow::Result<()> {
        let addr = bytes.len();
        match self {
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
                bytes.push(*v);
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
        }
        Ok(())
    }
}
