use std::io::Cursor;

use crate::util::read_shift_jis;
use anyhow::{bail, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use exalt_lir::{CallbackArg, Game};
use lazy_static::lazy_static;
use maplit::hashmap;
use std::collections::HashMap;

enum CallbackArgType {
    Int,
    Str,
}

lazy_static! {
    static ref FE9_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x1 => vec![CallbackArgType::Str],
            0x2 => vec![CallbackArgType::Str],
            0x4 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x5 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x8 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x9 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0xB => vec![CallbackArgType::Str],
            0xC => vec![CallbackArgType::Str],
            0xD => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
            0xE => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
        }
    };
    static ref FE10_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x1 => vec![CallbackArgType::Str],
            0x4 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x5 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x8 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x9 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0xB => vec![CallbackArgType::Str],
            0xC => vec![CallbackArgType::Str],
            0xE => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
            0x11 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x12 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x13 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x14 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
        }
    };
    static ref FE11_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x1 => vec![CallbackArgType::Str],
            0x4 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x5 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x8 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x9 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0xA => vec![CallbackArgType::Str],
            0xB => vec![CallbackArgType::Str],
            0xC => vec![CallbackArgType::Str],
            0xE => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
            0x11 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x12 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x13 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x14 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
        }
    };
    static ref FE12_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x1 => vec![CallbackArgType::Str],
            0x4 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x5 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x8 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x9 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0xC => vec![CallbackArgType::Str],
            0xE => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
            0x11 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x12 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x13 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x14 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
            0x16 => vec![
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
        }
    };
    static ref FE13_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x10 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x11 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x12 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x13 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x15 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x17 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ],
            0x18 => vec![CallbackArgType::Str],
            0x19 => vec![CallbackArgType::Str],
        }
    };
    static ref FE14_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x10 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
            ],
            0x11 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
            ],
            0x12 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
            ],
            0x13 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
            ],
            0x14 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x15 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x16 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
            ],
            0x17 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x18 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x19 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x1B => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
            0x1C => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
            0x1D => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x1E => vec![CallbackArgType::Str],
            0x1F => vec![CallbackArgType::Str],
            0x20 => vec![CallbackArgType::Str, CallbackArgType::Int],
        }
    };
    static ref FE15_EVENTS: HashMap<u32, Vec<CallbackArgType>> = {
        hashmap! {
            0x14 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x15 => vec![
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x17 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x1A => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
            0x1B => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
            0x1C => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
                CallbackArgType::Str,
            ],
            0x25 => vec![
                CallbackArgType::Str,
                CallbackArgType::Int,
            ],
            0x26 => vec![
                CallbackArgType::Str,
                CallbackArgType::Str,
            ]
        }
    };
}

fn read_gcn_args(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    signature: Option<&Vec<CallbackArgType>>,
    count: usize,
) -> Result<Vec<CallbackArg>> {
    let mut args = Vec::new();
    if let Some(sig) = signature {
        if sig.len() != count {
            bail!(
                "expected '{}' args but actual count is '{}'",
                sig.len(),
                count
            );
        }
        for arg in sig {
            match arg {
                CallbackArgType::Str => {
                    let raw = cursor.read_u16::<LittleEndian>()?;
                    let text = read_shift_jis(text_data, raw as u64)?;
                    args.push(CallbackArg::Str(text));
                }
                CallbackArgType::Int => {
                    args.push(CallbackArg::Int(cursor.read_i16::<LittleEndian>()? as i32));
                }
            }
        }
    } else {
        for _ in 0..count {
            args.push(CallbackArg::Int(cursor.read_i16::<LittleEndian>()? as i32));
        }
    }
    Ok(args)
}

fn read_three_ds_args(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    signature: Option<&Vec<CallbackArgType>>,
    count: usize,
) -> Result<Vec<CallbackArg>> {
    let mut args = Vec::new();
    if let Some(sig) = signature {
        if sig.len() != count {
            bail!(
                "expected '{}' args but actual count is '{}'",
                sig.len(),
                count
            );
        }
        for arg in sig {
            match arg {
                CallbackArgType::Str => {
                    let offset = cursor.read_u32::<LittleEndian>()? as usize;
                    let text = read_shift_jis(text_data, offset as u64)?;
                    args.push(CallbackArg::Str(text));
                }
                CallbackArgType::Int => {
                    args.push(CallbackArg::Int(cursor.read_i32::<LittleEndian>()?));
                }
            }
        }
    } else {
        for _ in 0..count {
            args.push(CallbackArg::Int(cursor.read_i32::<LittleEndian>()?));
        }
    }
    Ok(args)
}

pub fn read_args(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    game: Game,
    event: u32,
    count: usize,
) -> Result<Vec<CallbackArg>> {
    let signature = match game {
        Game::FE9 => FE9_EVENTS.get(&event),
        Game::FE10 => FE10_EVENTS.get(&event),
        Game::FE11 => FE11_EVENTS.get(&event),
        Game::FE12 => FE12_EVENTS.get(&event),
        Game::FE13 => FE13_EVENTS.get(&event),
        Game::FE14 => FE14_EVENTS.get(&event),
        Game::FE15 => FE15_EVENTS.get(&event),
    };
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => {
            read_gcn_args(cursor, text_data, signature, count)
        }
        Game::FE13 | Game::FE14 | Game::FE15 => {
            read_three_ds_args(cursor, text_data, signature, count)
        }
    }
}
