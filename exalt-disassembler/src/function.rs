use std::io::Cursor;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use exalt_lir::Game;

use crate::args;
use crate::types::CommonFunctionHeader;
use crate::util::{address_or_none, read_shift_jis_from_cursor};

fn read_gcn_function_header(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    game: Game,
) -> Result<CommonFunctionHeader> {
    let name_address = address_or_none(cursor.read_u32::<LittleEndian>()?);
    let code = cursor.read_u32::<LittleEndian>()?;
    let _parent_address = cursor.read_u32::<LittleEndian>()?;
    let event = cursor.read_u8()?;
    let arity = cursor.read_u8()?;
    let arg_count = cursor.read_u8()?;
    let unknown = cursor.read_u8()?;
    let id = cursor.read_u16::<LittleEndian>()? as u32;
    let frame_size = cursor.read_u16::<LittleEndian>()?;
    let args = if event != 0 {
        args::read_args(cursor, text_data, game, event.into(), arg_count.into())?
    } else {
        Vec::new()
    };
    let name = if let Some(address) = name_address {
        cursor.set_position(address as u64);
        Some(read_shift_jis_from_cursor(cursor)?)
    } else {
        None
    };
    Ok(CommonFunctionHeader {
        name,
        args,
        code,
        id,
        frame_size,
        event,
        arity,
        unknown,
    })
}

fn read_three_ds_function_header(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    game: Game,
) -> Result<CommonFunctionHeader> {
    let _header_address = cursor.read_u32::<LittleEndian>()?;
    let code = cursor.read_u32::<LittleEndian>()?;
    let event = cursor.read_u8()?;
    let arity = cursor.read_u8()?;
    let frame_size = cursor.read_u16::<LittleEndian>()?;
    let id = cursor.read_u32::<LittleEndian>()?;
    let name_address = address_or_none(cursor.read_u32::<LittleEndian>()?);
    let args_address = address_or_none(cursor.read_u32::<LittleEndian>()?);
    let args = if let Some(address) = args_address {
        cursor.set_position(address as u64);
        args::read_args(cursor, text_data, game, event.into(), arity.into())?
    } else {
        Vec::new()
    };
    let name = if let Some(address) = name_address {
        cursor.set_position(address as u64);
        Some(read_shift_jis_from_cursor(cursor)?)
    } else {
        None
    };
    Ok(CommonFunctionHeader {
        name,
        args,
        code,
        id,
        frame_size,
        event,
        arity,
        unknown: 0,
    })
}

pub fn read_function(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    game: Game,
) -> Result<CommonFunctionHeader> {
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => {
            read_gcn_function_header(cursor, text_data, game)
        }
        Game::FE13 | Game::FE14 | Game::FE15 => {
            read_three_ds_function_header(cursor, text_data, game)
        }
    }
}
