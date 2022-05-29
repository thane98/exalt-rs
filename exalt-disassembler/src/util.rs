use std::io::{Cursor, BufRead};

use anyhow::{bail, Result};
use encoding_rs::SHIFT_JIS;

pub fn address_or_none(address: u32) -> Option<u32> {
    if address != 0 {
        Some(address)
    } else {
        None
    }
}

pub fn read_shift_jis(data: &[u8], start: u64) -> anyhow::Result<String> {
    if start > data.len().try_into()? {
        bail!("Out of bounds text pointer.");
    }
    let mut cursor = Cursor::new(data);
    cursor.set_position(start);
    read_shift_jis_from_cursor(&mut cursor)
}

pub fn read_shift_jis_from_cursor(cursor: &mut Cursor<&[u8]>) -> Result<String> {
    let start = cursor.position();
    let mut buffer = Vec::new();
    cursor.read_until(0, &mut buffer)?;
    buffer.pop(); // Get rid of the null terminator
    let (v, _, failure) = SHIFT_JIS.decode(&buffer);
    if failure {
        bail!("Malformed shift-jis sequence addr={:X}", start)
    } else {
        Ok(v.to_string())
    }
}
