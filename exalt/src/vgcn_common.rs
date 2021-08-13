use lazy_static::lazy_static;
use maplit::hashmap;

use crate::EventArgType;

use std::collections::HashMap;

#[derive(Debug)]
pub struct VGcnCmbHeader {
    pub magic_number: u32,
    pub revision: u32,
    pub script_type: u32,
    pub function_table_address: u32,
    pub text_data_address: u32,
}

#[derive(Debug)]
pub struct RawFunctionData {
    pub name_address: u32,
    pub code_address: u32,
    pub parent_address: u32,
    pub function_type: u8,
    pub arity: u8,
    pub param_count: u8,
    pub padding: u8,
    pub id: u16,
    pub frame_size: u16,
    pub params: Vec<u16>,
}

lazy_static! {
    pub static ref FE10_EVENTS: HashMap<u8, Vec<EventArgType>> = {
        hashmap! {
            0x4 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x5 => vec![
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x8 => vec![
                EventArgType::Str,
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0x9 => vec![
                EventArgType::Str,
                EventArgType::Int,
                EventArgType::Int,
                EventArgType::Str,
            ],
            0xE => vec![
                EventArgType::Str,
                EventArgType::Str,
            ]
        }
    };
}

impl VGcnCmbHeader {
    // TODO: FE9 revision number
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.magic_number != 0x626D63 {
            Err(anyhow::anyhow!("Bad CMB magic number."))
        } else if self.revision != 0x20061024 {
            Err(anyhow::anyhow!(
                "Unsupported revision '{:X}'",
                self.revision
            ))
        } else {
            Ok(())
        }
    }
}
