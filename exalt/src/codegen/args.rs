use anyhow::Result;

use crate::{EventArg, FunctionData};

use super::state::CodeGenTextData;

pub trait ArgSerializer {
    fn serialize_args(function: &FunctionData, text_data: &mut CodeGenTextData) -> Result<Vec<u8>>;
}

pub struct V1ArgSerializer;

pub struct V3ArgSerializer;

impl ArgSerializer for V1ArgSerializer {
    fn serialize_args(function: &FunctionData, text_data: &mut CodeGenTextData) -> Result<Vec<u8>> {
        if function.function_type == 0 && !function.args.is_empty() {
            return Err(anyhow::anyhow!(
                "Function/event arguments cannot be used with function type 0."
            ));
        }
        let mut raw: Vec<u8> = Vec::new();
        for arg in &function.args {
            match arg {
                EventArg::Str(v) => {
                    let offset = text_data.offset(v)? as u16;
                    raw.extend(offset.to_le_bytes().iter());
                }
                EventArg::Int(v) => raw.extend((*v as u16).to_le_bytes().iter()),
                _ => {
                    return Err(anyhow::anyhow!(
                        "Script format does not support float arguments."
                    ))
                }
            }
        }
        while raw.len() % 4 != 0 {
            raw.push(0);
        }
        Ok(raw)
    }
}

impl ArgSerializer for V3ArgSerializer {
    fn serialize_args(function: &FunctionData, text_data: &mut CodeGenTextData) -> Result<Vec<u8>> {
        if function.function_type == 0 && !function.args.is_empty() {
            return Err(anyhow::anyhow!(
                "Function/event arguments cannot be used with function type 0."
            ));
        }
        let mut bytes: Vec<u8> = Vec::new();
        for arg in &function.args {
            match arg {
                crate::EventArg::Str(v) => {
                    let offset = text_data.offset(v)? as u32;
                    bytes.extend(offset.to_le_bytes().iter());
                }
                crate::EventArg::Int(v) => bytes.extend(v.to_le_bytes().iter()),
                crate::EventArg::Float(v) => bytes.extend(v.to_le_bytes().iter()),
            }
        }
        Ok(bytes)
    }
}
