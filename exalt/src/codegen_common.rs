use std::collections::HashMap;
use std::io::Cursor;

use byteorder::{BigEndian, WriteBytesExt};

use crate::common::encode_shift_jis;

pub struct CodeGenState<'a> {
    pub labels: HashMap<String, CodeGenLabelEntry>,
    pub text_data: &'a mut CodeGenTextData,
}

pub struct CodeGenLabelEntry {
    pub addr: Option<usize>,
    pub jumps: Vec<usize>,
}

pub struct CodeGenTextData {
    pub raw_text: Vec<u8>,
    pub offsets: HashMap<String, usize>,
}

impl<'a> CodeGenState<'a> {
    pub fn new(text_data: &'a mut CodeGenTextData) -> Self {
        CodeGenState {
            labels: HashMap::new(),
            text_data,
        }
    }

    pub fn add_label(&mut self, label: &str, addr: usize) -> anyhow::Result<()> {
        match self.labels.get_mut(label) {
            Some(label_data) => match label_data.addr {
                Some(_) => return Err(anyhow::anyhow!("Duplicate entries for label '{}'.", label)),
                None => {
                    label_data.addr = Some(addr);
                }
            },
            None => {
                let label_data = CodeGenLabelEntry {
                    addr: Some(addr),
                    jumps: Vec::new(),
                };
                self.labels.insert(label.to_owned(), label_data);
            }
        }
        Ok(())
    }

    pub fn add_jump(&mut self, label: &str, jump_addr: usize) {
        match self.labels.get_mut(label) {
            Some(label_data) => label_data.jumps.push(jump_addr),
            None => {
                let label_data = CodeGenLabelEntry {
                    addr: None,
                    jumps: vec![jump_addr],
                };
                self.labels.insert(label.to_owned(), label_data);
            }
        }
    }

    pub fn backpatch(&self, bytes: &mut [u8]) -> anyhow::Result<()> {
        let mut cursor = Cursor::new(bytes);
        for (label, label_data) in &self.labels {
            match label_data.addr {
                Some(addr) => {
                    for jump in &label_data.jumps {
                        let signed_label_addr = addr as i16;
                        let signed_jump_addr = *jump as i16;
                        let diff = signed_label_addr - signed_jump_addr;
                        cursor.set_position(*jump as u64);
                        cursor.write_i16::<BigEndian>(diff)?;
                    }
                }
                None => return Err(anyhow::anyhow!("Unresolved label '{}'", label)),
            }
        }
        Ok(())
    }
}

impl CodeGenTextData {
    pub fn new() -> Self {
        CodeGenTextData {
            raw_text: Vec::new(),
            offsets: HashMap::new(),
        }
    }

    pub fn offset(&mut self, text: &str) -> anyhow::Result<usize> {
        match self.offsets.get(text) {
            Some(offset) => Ok(*offset),
            None => {
                let bytes = encode_shift_jis(text)?;
                let offset = self.raw_text.len();
                self.raw_text.extend(bytes.into_iter());
                self.raw_text.push(0);
                self.offsets.insert(text.to_owned(), offset);
                Ok(offset)
            }
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.raw_text
    }
}

pub fn write_byte_or_short(out: &mut Vec<u8>, value: u16, byte_opcode: u8, short_opcode: u8) {
    if value <= 0x7F {
        out.push(byte_opcode);
        out.push(value as u8);
    } else {
        out.push(short_opcode);
        out.extend(value.to_be_bytes().iter());
    }
}

pub fn write_byte_or_short_or_int(
    out: &mut Vec<u8>,
    value: u32,
    byte_opcode: u8,
    short_opcode: u8,
    int_opcode: u8,
) {
    if value <= 0x7F {
        out.push(byte_opcode);
        out.push(value as u8);
    } else if value <= 0x7FFF {
        out.push(short_opcode);
        out.extend((value as u16).to_be_bytes().iter());
    } else {
        out.push(int_opcode);
        out.extend(value.to_be_bytes().iter());
    }
}

pub fn write_jump(
    output: &mut Vec<u8>,
    state: &mut CodeGenState,
    label: &str,
    jump_addr: usize,
    opcode: u8,
) {
    output.push(opcode);
    state.add_jump(label, jump_addr);
    output.push(0);
    output.push(0);
}
