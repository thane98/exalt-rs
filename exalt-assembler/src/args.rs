use anyhow::{bail, Result};

use exalt_lir::{CallbackArg, Function, Game};
use crate::types::CodeGenTextData;

fn serialize_gcn_args(function: &Function, text_data: &mut CodeGenTextData) -> Result<Vec<u8>> {
    if function.event == 0 && !function.args.is_empty() {
        bail!("function/event arguments cannot be used with function type 0.");
    }
    let mut raw = Vec::new();
    for arg in &function.args {
        match arg {
            CallbackArg::Str(v) => {
                let offset = text_data.offset(v)? as u16;
                raw.extend(offset.to_le_bytes().iter());
            }
            CallbackArg::Int(v) => raw.extend((*v as u16).to_le_bytes().iter()),
            _ => bail!("GameCube/Wii versions do not support floats"),
        }
    }

    // Hack to deal with padding when prefix data is present.
    if !raw.is_empty() {
        while (raw.len() + function.prefix.len()) % 4 != 0 {
            raw.push(0);
        }
    }

    Ok(raw)
}

fn serialize_three_ds_args(
    function: &Function,
    text_data: &mut CodeGenTextData,
) -> Result<Vec<u8>> {
    if function.event == 0 && !function.args.is_empty() {
        bail!("function/event arguments cannot be used with function type 0.");
    }
    let mut bytes = Vec::new();
    for arg in &function.args {
        match arg {
            CallbackArg::Str(v) => {
                let offset = text_data.offset(v)? as u32;
                bytes.extend(offset.to_le_bytes().iter());
            }
            CallbackArg::Int(v) => bytes.extend(v.to_le_bytes().iter()),
            CallbackArg::Float(v) => bytes.extend(v.to_le_bytes().iter()),
        }
    }
    Ok(bytes)
}

pub fn serialize_args(function: &Function, text_data: &mut CodeGenTextData, game: Game) -> Result<Vec<u8>> {
    match game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => serialize_gcn_args(function, text_data),
        Game::FE13 | Game::FE14 | Game::FE15 => serialize_three_ds_args(function, text_data),
    }
}
