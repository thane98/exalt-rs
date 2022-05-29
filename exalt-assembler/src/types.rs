use std::io::Cursor;

use crate::util;
use anyhow::Result;
use byteorder::{BigEndian, WriteBytesExt};
use exalt_lir::Game;
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct RawFunctionHeader {
    pub name_address: Option<u32>,
    pub code_address: u32,
    pub parent_address: Option<u32>,
    pub args_address: Option<u32>,
    pub frame_size: u16,
    pub event: u8,
    pub arity: u8,
    pub param_count: u8,
    pub unknown: u8,
    pub prefix: Vec<u8>,
    pub suffix: Vec<u8>,
}

pub struct RawFunction {
    pub header: RawFunctionHeader,
    pub name: Vec<u8>,
    pub args: Vec<u8>,
    pub code: Vec<u8>,
}

pub struct CodeGenState<'a> {
    pub labels: FxHashMap<String, CodeGenLabelEntry>,
    pub text_data: &'a mut CodeGenTextData,
}

pub struct CodeGenLabelEntry {
    pub addr: Option<usize>,
    pub jumps: Vec<usize>,
}

pub enum CodeGenTextStrategy {
    Dynamic,
    HardCoded,
}

pub struct CodeGenTextData {
    pub raw_text: Vec<u8>,
    pub offsets: FxHashMap<String, usize>,
    pub strategy: CodeGenTextStrategy,
}

impl<'a> CodeGenState<'a> {
    pub fn new(text_data: &'a mut CodeGenTextData) -> Self {
        CodeGenState {
            labels: FxHashMap::default(),
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
    pub fn hard_coded(raw_text: Vec<u8>, offsets: FxHashMap<String, usize>) -> Self {
        CodeGenTextData {
            raw_text,
            offsets,
            strategy: CodeGenTextStrategy::HardCoded,
        }
    }

    pub fn offset(&mut self, text: &str) -> Result<usize> {
        match &self.strategy {
            CodeGenTextStrategy::Dynamic => match self.offsets.get(text) {
                Some(offset) => Ok(*offset),
                None => {
                    let bytes = util::encode_shift_jis(text)?;
                    let offset = self.raw_text.len();
                    self.raw_text.extend(bytes.into_iter());
                    self.raw_text.push(0);
                    self.offsets.insert(text.to_owned(), offset);
                    Ok(offset)
                }
            },
            CodeGenTextStrategy::HardCoded => self.offsets.get(text).copied().ok_or_else(|| {
                anyhow::anyhow!("'{}' does not exist in hard coded text data", text)
            }),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.raw_text
    }
}

impl Default for CodeGenTextData {
    fn default() -> Self {
        Self {
            raw_text: Vec::new(),
            offsets: FxHashMap::default(),
            strategy: CodeGenTextStrategy::Dynamic,
        }
    }
}

pub struct VersionInfo {
    pub event_table_pointer_address: u64,
    pub text_data_pointer_address: u64,
    pub text_first: bool,
    pub pad_last_event: bool,
}

impl VersionInfo {
    pub fn for_game(game: Game) -> Self {
        match game {
            Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => VersionInfo {
                event_table_pointer_address: 0x28,
                text_data_pointer_address: 0x24,
                text_first: true,
                pad_last_event: true,
            },
            Game::FE13 | Game::FE14 | Game::FE15 => VersionInfo {
                event_table_pointer_address: 0x1C,
                text_data_pointer_address: 0x20,
                text_first: false,
                pad_last_event: false,
            }
        }
    }
}
