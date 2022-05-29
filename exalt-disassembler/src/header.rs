use std::io::Cursor;
use crate::types::CmbHeader;
use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use exalt_lir::Game;

fn read_gcn_header(cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader> {
    let magic_number = cursor.read_u32::<LittleEndian>()?;
    cursor.set_position(0x18);
    let revision = cursor.read_u32::<LittleEndian>()?;
    cursor.set_position(0x22);
    let global_frame_size = cursor.read_u16::<LittleEndian>()? as u32;
    let text_data_address = cursor.read_u32::<LittleEndian>()?;
    let function_table_address = cursor.read_u32::<LittleEndian>()?;
    Ok(CmbHeader {
        magic_number,
        revision,
        text_data_address,
        function_table_address,
        global_frame_size,
        init_function_index: None,
    })
}

fn read_three_ds_header(cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader> {
    let magic_number = cursor.read_u32::<LittleEndian>()?;
    let revision = cursor.read_u32::<LittleEndian>()?;
    cursor.set_position(0x18);
    let global_frame_size = cursor.read_u32::<LittleEndian>()?;
    let function_table_address = cursor.read_u32::<LittleEndian>()?;
    let text_data_address = cursor.read_u32::<LittleEndian>()?;
    cursor.set_position(0x26);
    let init_function_index = cursor.read_u16::<LittleEndian>()?;
    let header = CmbHeader {
        magic_number,
        revision,
        text_data_address,
        function_table_address,
        global_frame_size,
        init_function_index: if init_function_index > 0 {
            Some(init_function_index - 1)
        } else {
            None
        },
    };
    Ok(header)
}

pub fn read_header(cursor: &mut Cursor<&[u8]>, game: Game) -> Result<CmbHeader> {
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => read_gcn_header(cursor),
        Game::FE13 | Game::FE14 | Game::FE15 => read_three_ds_header(cursor),
    }
}
