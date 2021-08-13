use crate::codegen_common::*;
use crate::common::encode_shift_jis;
use crate::vgcn_common::RawFunctionData;
use crate::{EventArg, FunctionData, Opcode};
use anyhow::Context;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Cursor;

const FUNCTION_HEADER_SIZE: usize = 0x14;

fn get_raw_function_args(
    function: &FunctionData,
    text_data: &mut CodeGenTextData,
) -> anyhow::Result<Vec<u16>> {
    if function.function_type == 0 && !function.args.is_empty() {
        return Err(anyhow::anyhow!(
            "Function/event arguments cannot be used with function type 0."
        ));
    }
    let mut raw = Vec::new();
    for arg in &function.args {
        match arg {
            EventArg::Str(v) => {
                let offset = text_data.offset(v)?;
                raw.push(offset as u16);
            }
            EventArg::Int(v) => raw.push(*v as u16),
            _ => {
                return Err(anyhow::anyhow!(
                    "GCN script format does not support float arguments."
                ))
            }
        }
    }
    if raw.len() % 2 != 0 {
        raw.push(0);
    }
    Ok(raw)
}

fn gen_function_code(
    function: &FunctionData,
    function_id: u16,
    text_data: &mut CodeGenTextData,
) -> anyhow::Result<(RawFunctionData, Vec<u8>, Vec<u8>)> {
    // Calculate addresses and serialize name / args.
    let name_bytes = match &function.name {
        Some(v) => {
            let mut bytes = encode_shift_jis(v)?;
            bytes.push(0);
            while bytes.len() % 4 != 0 {
                bytes.push(0);
            }
            bytes
        }
        None => Vec::new(),
    };
    let raw_args = get_raw_function_args(function, text_data)
        .context("Failed to write function arguments.")?;
    let code_address = if name_bytes.is_empty() {
        FUNCTION_HEADER_SIZE + raw_args.len() * 2
    } else {
        FUNCTION_HEADER_SIZE + name_bytes.len()
    };
    let name_address = if !name_bytes.is_empty() {
        FUNCTION_HEADER_SIZE
    } else {
        0
    };

    // Generate the actual code.
    let mut code_gen_state = CodeGenState::new(text_data);
    let mut raw_code: Vec<u8> = Vec::new();
    for op in &function.code {
        op.to_vgcn_bytes(&mut raw_code, &mut code_gen_state)
            .context(format!(
                "Failed to convert opcode to v3ds format: '{:?}'",
                op
            ))?;
    }
    raw_code.push(0);
    code_gen_state.backpatch(&mut raw_code)?;

    // Combine everything and return.
    Ok((
        RawFunctionData {
            name_address: name_address as u32,
            code_address: code_address as u32,
            parent_address: 0,
            function_type: function.function_type,
            arity: function.arity,
            param_count: function.args.len() as u8,
            padding: 0,
            id: function_id,
            frame_size: function.frame_size as u16,
            params: raw_args,
        },
        name_bytes,
        raw_code,
    ))
}

fn serialize_function(
    func: &RawFunctionData,
    name: &[u8],
    code: &[u8],
    base_address: u32,
) -> Vec<u8> {
    let mut raw = Vec::new();
    if func.name_address != 0 {
        raw.extend((func.name_address + base_address).to_le_bytes().iter());
    } else {
        raw.extend((0 as u32).to_le_bytes().iter());
    }
    raw.extend((func.code_address + base_address).to_le_bytes().iter());
    raw.extend((0 as u32).to_le_bytes().iter());
    raw.push(func.function_type);
    raw.push(func.arity);
    raw.push(func.param_count);
    raw.push(func.padding);
    raw.extend(func.id.to_le_bytes().iter());
    raw.extend(func.frame_size.to_le_bytes().iter());
    raw.extend_from_slice(name);
    for p in &func.params {
        // TODO: These params should not be signed by default
        raw.extend((*p).to_le_bytes().iter());
    }
    raw.extend_from_slice(code);
    raw
}

fn gen_code(
    script_name: &str,
    script_type: u16,
    functions: &[FunctionData],
) -> anyhow::Result<Vec<u8>> {
    // Convert name to bytes and check length.
    let name_bytes = encode_shift_jis(script_name)?;
    if name_bytes.len() > 0x13 {
        return Err(anyhow::anyhow!("Script name is too long for GCN format."));
    }

    // Write the header. Will need to revisit later to write function table and text data pointers.
    let mut raw: Vec<u8> = Vec::new();
    raw.extend((0x626D63 as u32).to_le_bytes().iter()); // Magic number
    raw.extend_from_slice(&name_bytes);
    while raw.len() < 0x18 {
        raw.push(0);
    }
    raw.extend((0x20061024 as u32).to_le_bytes().iter()); // Revision number.
    for _ in 0..6 {
        raw.push(0);
    }
    raw.extend(script_type.to_le_bytes().iter());
    raw.extend((0x2C as u32).to_le_bytes().iter()); // Text data pointer, always 0x2C
    raw.extend((0 as u32).to_le_bytes().iter()); // Function table address, need to revisit later.

    // In GCN functions go after text data, but we won't know how long
    // text data is until we serialize all of the functions.
    //
    // So, we serialize the functions with base address = 0 and then
    // do a second pass to get the absolute address.
    let mut text_data = CodeGenTextData::new();
    let mut raw_functions = Vec::new();
    for i in 0..functions.len() {
        let function = &functions[i];
        raw_functions.push(gen_function_code(function, i as u16, &mut text_data)?);
    }

    // Now we can write text data.
    raw.extend_from_slice(text_data.bytes());
    while raw.len() % 4 != 0 {
        raw.push(0);
    }

    // Serialize functions.
    let mut function_bytes = Vec::new();
    let mut function_addresses = Vec::new();
    let function_table_length = (raw_functions.len() + 1) * 4;
    for (func, name, code) in raw_functions {
        let base_address = (raw.len() + function_table_length + function_bytes.len()) as u32;
        function_addresses.push(base_address);
        function_bytes.extend(serialize_function(&func, &name, &code, base_address));
        while function_bytes.len() % 4 != 0 {
            function_bytes.push(0);
        }
    }

    // TODO: Write the function table and extend raw with function bytes
    let function_table_address = raw.len();
    for address in function_addresses {
        raw.extend(address.to_le_bytes().iter());
    }
    raw.extend((0 as u32).to_le_bytes().iter());
    raw.extend_from_slice(&function_bytes);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }

    // Fix pointers.
    let mut cursor = Cursor::new(&mut raw);
    cursor.set_position(0x28);
    cursor.write_u32::<LittleEndian>(function_table_address as u32)?;
    Ok(raw)
}

pub fn gen_vgcn_code(
    script_name: &str,
    script_type: u32,
    functions: &[FunctionData],
) -> anyhow::Result<Vec<u8>> {
    gen_code(script_name, script_type as u16, functions).context("Code generation failed.")
}

impl Opcode {
    fn to_vgcn_bytes(&self, bytes: &mut Vec<u8>, state: &mut CodeGenState) -> anyhow::Result<()> {
        let addr = bytes.len();
        match self {
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
            _ => return Err(anyhow::anyhow!("Unsupported VGCN opcode {:?}", self)),
        }
        Ok(())
    }
}
