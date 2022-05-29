use anyhow::{Context, Result};

use crate::{util, args, code};

const GCN_AND_WII_FUNCTION_HEADER_SIZE: usize = 0x14;
const THREE_DS_FUNCTION_HEADER_SIZE: u32 = 0x18;

use crate::types::{RawFunction, RawFunctionHeader, CodeGenTextData};
use exalt_lir::{Function, Game};

fn convert_to_raw_gcn_function(
    function: &Function,
    text_data: &mut CodeGenTextData,
    game: Game,
) -> Result<RawFunction> {
    let name_bytes = match &function.name {
        Some(v) => {
            let mut bytes = util::encode_shift_jis(v)?;
            bytes.push(0);
            // Pad to the next word.
            // Prefix goes between the null terminator and the next word, so it may cover part of the padding.
            while (bytes.len() + function.prefix.len()) % 4 != 0 {
                bytes.push(0);
            }
            bytes
        }
        None => Vec::new(),
    };
    let raw_args = args::serialize_args(function, text_data, game)
        .context("Failed to write function arguments.")?;
    let code_address = if name_bytes.is_empty() {
        (GCN_AND_WII_FUNCTION_HEADER_SIZE + raw_args.len() + function.prefix.len()) as u32
    } else {
        (GCN_AND_WII_FUNCTION_HEADER_SIZE + name_bytes.len() + function.prefix.len()) as u32
    };
    let name_address = if !name_bytes.is_empty() {
        Some(GCN_AND_WII_FUNCTION_HEADER_SIZE as u32)
    } else {
        None
    };
    let raw_function_header = RawFunctionHeader {
        name_address,
        code_address,
        parent_address: None,
        args_address: None,
        frame_size: function.frame_size as u16,
        event: function.event,
        arity: function.arity,
        param_count: function.args.len() as u8,
        unknown: function.unknown,
        prefix: function.prefix.clone(),
        suffix: function.suffix.clone(),
    };
    Ok(RawFunction {
        header: raw_function_header,
        name: name_bytes,
        args: raw_args,
        code: code::serialize_opcodes(&function.code, text_data, game)?,
    })
}

fn convert_to_raw_three_ds_function(
    function: &Function,
    text_data: &mut CodeGenTextData,
    game: Game,
) -> Result<RawFunction> {
    let name_bytes = if let Some(name) = &function.name {
        if function.event == 0 && name.contains("::") {
            let mut bytes = util::encode_shift_jis(name)?;
            bytes.push(0);
            bytes
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let raw_args = args::serialize_args(function, text_data, game)
        .context("failed to write function arguments")?;
    let extended_header_address = THREE_DS_FUNCTION_HEADER_SIZE;
    let code_address = if name_bytes.is_empty() {
        extended_header_address + (raw_args.len() as u32)
    } else {
        extended_header_address + (name_bytes.len() as u32)
    };
    let name_address = if !name_bytes.is_empty() {
        Some(extended_header_address as u32)
    } else {
        None
    };
    let args_address = if function.event == 0 {
        None
    } else {
        Some(extended_header_address)
    };
    let raw_function_header = RawFunctionHeader {
        name_address,
        code_address: code_address as u32,
        parent_address: None,
        args_address,
        frame_size: function.frame_size as u16,
        event: function.event,
        arity: function.arity,
        param_count: function.args.len() as u8,
        unknown: function.unknown,
        prefix: Vec::new(),
        suffix: Vec::new(),
    };
    Ok(RawFunction {
        header: raw_function_header,
        name: name_bytes,
        args: raw_args,
        code: code::serialize_opcodes(&function.code, text_data, game)?,
    })
}

pub fn convert_to_raw_function(function: &Function, text_data: &mut CodeGenTextData, game: Game) -> Result<RawFunction> {
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => convert_to_raw_gcn_function(function, text_data, game),
        Game::FE13 | Game::FE14 | Game::FE15 => convert_to_raw_three_ds_function(function, text_data, game),
    }
}

fn serialize_gcn_function(
    function: &RawFunction,
    function_id: u32,
    base_address: u32,
) -> Result<Vec<u8>> {
    let header = &function.header;
    let mut raw = Vec::new();
    match &header.name_address {
        Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
        None => raw.extend(0_u32.to_le_bytes().iter()),
    }
    raw.extend((header.code_address + base_address).to_le_bytes().iter());
    match &header.parent_address {
        Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
        None => raw.extend(0_u32.to_le_bytes().iter()),
    }
    raw.push(header.event);
    raw.push(header.arity);
    raw.push(header.param_count);
    raw.push(header.unknown);
    raw.extend((function_id as u16).to_le_bytes().iter());
    raw.extend((header.frame_size as u16).to_le_bytes().iter());
    raw.extend_from_slice(&function.name);
    raw.extend_from_slice(&function.args);
    raw.extend_from_slice(&header.prefix);
    raw.extend_from_slice(&function.code);
    raw.extend_from_slice(&header.suffix);
    Ok(raw)
}

fn serialize_three_ds_function(function: &RawFunction, id: u32, base_address: u32) -> Result<Vec<u8>> {
    let header = &function.header;
    let mut raw = Vec::new();
    raw.extend((base_address as u32).to_le_bytes().iter());
    raw.extend((header.code_address + base_address).to_le_bytes().iter());
    raw.push(header.event);
    raw.push(header.arity);
    raw.push(header.frame_size as u8);
    raw.push(0); // Padding
    raw.extend(id.to_le_bytes().iter());
    match &header.name_address {
        Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
        None => raw.extend(0_u32.to_le_bytes().iter()),
    }
    match &header.args_address {
        Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
        None => raw.extend(0_u32.to_le_bytes().iter()),
    }
    raw.extend_from_slice(&function.name);
    raw.extend_from_slice(&function.args);
    raw.extend_from_slice(&function.code);
    Ok(raw)
}

pub fn serialize_function(function: &RawFunction, id: u32, base_address: u32, game: Game) -> Result<Vec<u8>> {
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => serialize_gcn_function(function, id, base_address),
        Game::FE13 | Game::FE14 | Game::FE15 => serialize_three_ds_function(function, id, base_address),
    }
}
