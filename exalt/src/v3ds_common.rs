use crate::EventArgType;
use lazy_static::lazy_static;
use maplit::hashmap;
use std::collections::HashMap;

pub struct V3dsCmbHeader {
    pub magic_number: u32,
    pub revision: u32,
    pub unk1: u32,
    pub script_name_address: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub function_table_address: u32,
    pub text_data_address: u32,
    pub padding: u32,
}

#[derive(Debug)]
pub struct RawFunctionData {
    pub header_address: u32,
    pub code_address: u32,
    pub function_type: u8,
    pub arity: u8,
    pub frame_size: u8,
    pub pad: u8,
    pub id: u32,
    pub name_address: u32,
    pub args_address: u32,
}

lazy_static! {
    pub static ref FE14_EVENTS: HashMap<u32, Vec<EventArgType>> = {
        hashmap! {
            0x10 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
            ],
            0x11 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
            ],
            0x12 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
            ],
            0x13 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
            ],
            0x14 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x15 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x16 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
            ],
            0x17 => vec![
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x18 => vec![
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x19 => vec![
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x1B => vec![
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Str,
                EventArgType::Int,
            ],
            0x1C => vec![
                EventArgType::Str,
                EventArgType::Int,
            ],
            0x1D => vec![
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x1E => vec![EventArgType::Str],
            0x1F => vec![EventArgType::Str],
            0x20 => vec![EventArgType::Str, EventArgType::Int],
        }
    };
}

impl V3dsCmbHeader {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.magic_number != 0x626D63 {
            Err(anyhow::anyhow!("Bad CMB magic number."))
        } else if self.revision != 0x20110819 {
            Err(anyhow::anyhow!(
                "Unsupported revision '{:X}'",
                self.revision
            ))
        } else {
            Ok(())
        }
    }
}
