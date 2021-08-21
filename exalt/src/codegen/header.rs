use anyhow::{anyhow, Result};

use crate::common::encode_shift_jis;

pub trait RawHeaderBuilder {
    fn build_header(&self, script_name: &str, script_type: u32) -> Result<Vec<u8>>;
}

pub struct V1RawHeaderBuilder;

pub struct V2RawHeaderBuilder;

pub struct V3RawHeaderBuilder;

fn build_v1_or_v2_header(revision: u32, script_name: &str, script_type: u32) -> Result<Vec<u8>> {
    // Verify that name fits within the V1/V2 limit.
    let name_bytes = encode_shift_jis(script_name)?;
    if name_bytes.len() > 0x13 {
        return Err(anyhow!("Script name is too long for this format."));
    }

    // Build the header without filling in event / text pointers.
    // Can't do these now since we need to serialize everything to
    // figure out the addresses.
    let mut raw = Vec::new();
    raw.extend((0x626D63 as u32).to_le_bytes().iter()); // Magic number
    raw.extend_from_slice(&name_bytes);
    while raw.len() < 0x18 {
        raw.push(0);
    }
    raw.extend(revision.to_le_bytes().iter());
    for _ in 0..6 {
        raw.push(0);
    }
    raw.extend((script_type as u16).to_le_bytes().iter());

    // Text + event pointers which we're skipping for now.
    for _ in 0..8 {
        raw.push(0);
    }
    Ok(raw)
}

impl RawHeaderBuilder for V1RawHeaderBuilder {
    fn build_header(&self, script_name: &str, script_type: u32) -> Result<Vec<u8>> {
        build_v1_or_v2_header(0x20041125, script_name, script_type)
    }
}

impl RawHeaderBuilder for V2RawHeaderBuilder {
    fn build_header(&self, script_name: &str, script_type: u32) -> Result<Vec<u8>> {
        build_v1_or_v2_header(0x20061024, script_name, script_type)
    }
}

impl RawHeaderBuilder for V3RawHeaderBuilder {
    fn build_header(&self, script_name: &str, script_type: u32) -> Result<Vec<u8>> {
        let name_bytes = encode_shift_jis(script_name)?;
        let mut raw: Vec<u8> = Vec::new();
        raw.extend((0x626D63 as u32).to_le_bytes().iter()); // Magic number
        raw.extend((0x20110819 as u32).to_le_bytes().iter()); // Revision number.
        raw.extend((0 as u32).to_le_bytes().iter());
        raw.extend((0x28 as u32).to_le_bytes().iter()); // Name pointer, always 0x28
        raw.resize(0x24, 0);
        raw.extend(script_type.to_le_bytes().iter());
        raw.extend(name_bytes);
        raw.push(0);
        while raw.len() % 4 != 0 {
            raw.push(0);
        }
        Ok(raw)
    }
}
