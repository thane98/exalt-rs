mod args;
mod code;
mod function;
mod header;
mod types;
mod util;

use std::io::Cursor;

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use exalt_lir::{Function, Game, RawScript};
use types::CmbHeader;

// The FE9/FE10 compiler seems to leave junk between null terminators and the next word boundary.
// Ex. Name ends at 0x4, we expect 0x5 until 0x8 to be all zeroes, but there is actually non-zero values for some reason.
// There are no pointers or offsets to this data and loading it is a hassle, so chances are this is unused.
// We hold on to this data anyways just to be safe.
fn read_junk_until_word_boundary(cursor: &mut Cursor<&[u8]>, game: Game) -> Result<Vec<u8>> {
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => {
            let mut buffer = Vec::new();
            let mut has_non_zero_values = false;
            while cursor.position() % 4 != 0 {
                let b = cursor.read_u8()?;
                has_non_zero_values = has_non_zero_values || b != 0;
                buffer.push(b);
            }
            if !has_non_zero_values {
                buffer.clear();
            }
            Ok(buffer)
        }
        _ => Ok(vec![]),
    }
}

fn read_function_table(cursor: &mut Cursor<&[u8]>) -> Result<Vec<usize>> {
    let mut addresses: Vec<usize> = Vec::new();
    let mut next = cursor.read_u32::<LittleEndian>()?;
    while next != 0 {
        addresses.push(next as usize);
        next = cursor.read_u32::<LittleEndian>()?;
    }
    Ok(addresses)
}

fn validate_header(header: &CmbHeader, game: Game) -> Result<()> {
    if header.magic_number != 0x626D63 {
        bail!("invalid magic number");
    }
    let valid_revision = match game {
        Game::FE9 => header.revision == 0x20041125,
        Game::FE10 | Game::FE11 | Game::FE12 => header.revision == 0x20061024,
        Game::FE13 | Game::FE14 | Game::FE15 => header.revision >= 0x20080801,
    };
    if !valid_revision {
        bail!("invalid revision '0x{:X}'", header.revision);
    }
    Ok(())
}

/// Disassemble a script for the target game
pub fn disassemble(script: &[u8], game: Game) -> Result<RawScript> {
    let mut cursor = Cursor::new(script);
    let header =
        header::read_header(&mut cursor, game).with_context(|| "failed to read CMB header")?;
    validate_header(&header, game)?;

    // Load text data.
    let text_data_address = header.text_data_address as usize;
    if text_data_address > script.len() {
        bail!("text data address is out of bounds.");
    }
    let text_data = &script[text_data_address..];

    // Load function addresses.
    let function_table_address = header.function_table_address as usize;
    if function_table_address >= script.len() {
        return Err(anyhow::anyhow!("function table address is out of bounds"));
    }
    cursor.set_position(function_table_address as u64);
    let addresses = read_function_table(&mut cursor).context("failed to read function table")?;

    // Parse individual functions.
    let mut functions = Vec::new();
    for address in addresses {
        // Read the function header.
        cursor.set_position(address as u64);
        let raw_function = function::read_function(&mut cursor, text_data, game)
            .with_context(|| format!("failed to read function at address '0x{:X}'", address))?;

        // Hack to deal with "junk" data after the name/args in FE9/FE10.
        // Doesn't seem like it's referenced anywhere, but we preserve it just in case.
        let prefix = read_junk_until_word_boundary(&mut cursor, game)?;

        // Read the code.
        if raw_function.code as usize >= script.len() {
            return Err(anyhow::anyhow!(
                "Code address is out of bounds, RawFunctionHeader={:?}",
                raw_function
            ));
        }
        cursor.set_position(raw_function.code.into());
        let code = code::disassemble(&mut cursor, text_data, game).with_context(|| {
            format!(
                "function disassembly failed, RawFunctionHeader={:?}",
                raw_function
            )
        })?;

        // Hack to deal with "junk" data after the terminating opcode in FE9/FE10.
        // Doesn't seem like it's referenced anywhere, but we preserve it just in case.
        let suffix = read_junk_until_word_boundary(&mut cursor, game)?;

        functions.push(Function {
            event: raw_function.event,
            arity: raw_function.arity,
            frame_size: raw_function.frame_size as usize,
            name: raw_function.name,
            args: raw_function.args,
            code,
            unknown: raw_function.unknown,
            prefix,
            suffix,
        });
    }

    Ok(RawScript {
        functions,
        global_frame_size: header.global_frame_size as usize,
    })
}
