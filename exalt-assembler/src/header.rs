use crate::util;
use anyhow::{bail, Result};
use exalt_lir::{Game, RawScript};

fn build_gcn_header(revision: u32, script_name: &str, global_frame_size: u16) -> Result<Vec<u8>> {
    // Verify that name fits within the V1/V2 limit.
    let name_bytes = util::encode_shift_jis(script_name)?;
    if name_bytes.len() > 0x13 {
        bail!("script name is too long");
    }
    // Build the header without filling in event / text pointers since we don't know where they will go.
    let mut raw = Vec::new();
    raw.extend(0x626D63_u32.to_le_bytes().iter()); // Magic number
    raw.extend_from_slice(&name_bytes);
    while raw.len() < 0x18 {
        raw.push(0);
    }
    raw.extend(revision.to_le_bytes().iter());
    raw.resize(raw.len() + 6, 0);
    raw.extend((global_frame_size as u16).to_le_bytes().iter());
    raw.resize(raw.len() + 8, 0);
    Ok(raw)
}

fn build_three_ds_header(
    script_name: &str,
    global_frame_size: u32,
) -> Result<Vec<u8>> {
    let name_bytes = util::encode_shift_jis(script_name)?;
    let mut raw: Vec<u8> = Vec::new();
    raw.extend(0x626D63_u32.to_le_bytes().iter()); // Magic number
    raw.extend(0x20110819_u32.to_le_bytes().iter()); // Revision number.
    raw.extend(0_u32.to_le_bytes().iter());
    raw.extend(0x28_u32.to_le_bytes().iter()); // Name pointer, always 0x28
    raw.resize(0x18, 0);
    raw.extend(global_frame_size.to_le_bytes().iter());
    raw.resize(0x28, 0);
    raw.extend(name_bytes);
    raw.push(0);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }
    Ok(raw)
}

pub fn build(script: &RawScript, script_name: &str, game: Game) -> Result<Vec<u8>> {
    match game {
        Game::FE9 => build_gcn_header(0x20041125, script_name, script.global_frame_size as u16),
        Game::FE10 | Game::FE11 | Game::FE12 => {
            build_gcn_header(0x20061024, script_name, script.global_frame_size as u16)
        }
        Game::FE13 | Game::FE14 | Game::FE15 => {
            build_three_ds_header(script_name, script.global_frame_size as u32)
        }
    }
}
