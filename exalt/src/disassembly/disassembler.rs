use std::collections::HashSet;
use std::io::Cursor;

use anyhow::{anyhow, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::common::read_shift_jis_string;
use crate::disassembly::resolve_state::ResolveState;
use crate::{Script, FunctionData, Opcode};

use super::args::FunctionArgsReader;
use super::code::CodeDisassembler;
use super::function_header::FunctionHeaderReader;
use super::header::CmbHeaderReader;

fn read_function_table(cursor: &mut Cursor<&[u8]>) -> Result<Vec<usize>> {
    let mut addresses: Vec<usize> = Vec::new();
    let mut next = cursor.read_u32::<LittleEndian>()?;
    while next != 0 {
        addresses.push(next as usize);
        next = cursor.read_u32::<LittleEndian>()?;
    }
    Ok(addresses)
}

fn disassemble_code<T: CodeDisassembler>(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    disassembler: &T,
) -> Result<Vec<Opcode>> {
    // First pass: Raw disassembly. Don't try to decode text or resolve jumps.
    let mut state = ResolveState::new(text_data);
    let mut ops: Vec<(usize, Opcode)> = Vec::new();
    loop {
        let (real_addr, raw_op) = disassembler
            .read_opcode(cursor, &mut state)
            .with_context(|| format!("Failed to read opcode at '0x{:X}'", cursor.position()))?;
        match raw_op {
            Opcode::Done => break,
            _ => ops.push((real_addr, raw_op)),
        }
    }

    // Second pass: Place labels.
    let mut resolved_ops: Vec<Opcode> = Vec::new();
    let mut placed_labels: HashSet<String> = HashSet::new();
    for (addr, op) in ops {
        if let Some(label) = state.labels.get(&addr) {
            resolved_ops.push(Opcode::Label(label.to_owned()));
            placed_labels.insert(label.to_owned());
        }
        resolved_ops.push(op);
    }

    // Sanity check: Did we place every label?
    let unplaced_labels: Vec<String> = state
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

pub fn disassemble<
    T: CmbHeaderReader,
    U: FunctionHeaderReader,
    V: FunctionArgsReader,
    W: CodeDisassembler,
>(
    script: &[u8],
    cmb_header_reader: &T,
    function_header_reader: &U,
    function_args_reader: &V,
    disassembler: &W,
) -> Result<Script> {
    let mut cursor = Cursor::new(script);
    let header = cmb_header_reader.read_cmb_header(&mut cursor)?;

    // Load text data.
    let text_data_address = header.text_data_address as usize;
    if text_data_address > script.len() {
        return Err(anyhow!("Text data address is out of bounds."));
    }
    let text_data = &script[text_data_address..];

    // Load function addresses.
    let function_table_address = header.function_table_address as usize;
    if function_table_address >= script.len() {
        return Err(anyhow::anyhow!("Function table address is out of bounds."));
    }
    cursor.set_position(function_table_address as u64);
    let addresses = read_function_table(&mut cursor).context("Failed to read function table.")?;

    // Parse individual functions.
    let mut functions = Vec::new();
    for address in addresses {
        // Read the function header.
        cursor.set_position(address as u64);
        let raw_function = function_header_reader
            .read_function_header(&mut cursor)
            .with_context(|| format!("Failed to read function at address '0x{:X}'", address))?;

        // Read the function name (optional).
        let function_name = if let Some(name_address) = raw_function.name_address {
            if name_address as usize >= script.len() {
                return Err(anyhow::anyhow!(
                    "Name address is out of bounds, RawFunctionHeader={:?}",
                    raw_function
                ));
            }
            Some(
                read_shift_jis_string(script, name_address as usize)
                    .context("Failed to read function name.")?,
            )
        } else {
            None
        };

        // Read the function args (optional).
        let args = if let Some(args_address) = raw_function.args_address {
            if args_address as usize >= script.len() {
                return Err(anyhow::anyhow!(
                    "Args address is out of bounds, RawFunctionHeader={:?}",
                    raw_function
                ));
            }
            cursor.set_position(args_address as u64);
            function_args_reader
                .read_function_args(
                    &mut cursor,
                    text_data,
                    raw_function.function_type as u32,
                    raw_function.param_count as usize,
                )
                .with_context(|| {
                    format!(
                        "Failed to read function args, RawFunctionHeader={:?}",
                        raw_function
                    )
                })?
        } else {
            Vec::new()
        };

        // Read the code.
        if raw_function.code_address as usize >= script.len() {
            return Err(anyhow::anyhow!(
                "Code address is out of bounds, RawFunctionHeader={:?}",
                raw_function
            ));
        }
        cursor.set_position(raw_function.code_address as u64);
        let code = disassemble_code(&mut cursor, text_data, disassembler).with_context(|| {
            format!(
                "Function disassembly failed, RawFunctionHeader={:?}",
                raw_function
            )
        })?;

        functions.push(FunctionData {
            function_type: raw_function.function_type,
            arity: raw_function.arity,
            frame_size: raw_function.frame_size as usize,
            name: function_name,
            args,
            code,
        });
    }

    Ok(Script {
        functions,
        script_type: header.script_type,
    })
}
