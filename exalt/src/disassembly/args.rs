use crate::common::read_shift_jis_string;
use crate::{EventArg, EventArgType};
use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use lazy_static::lazy_static;
use maplit::hashmap;
use std::collections::HashMap;
use std::io::Cursor;

pub trait FunctionArgsReader {
    fn read_function_args(
        &self,
        cursor: &mut Cursor<&[u8]>,
        text_data: &[u8],
        function_type: u32,
        param_count: usize,
    ) -> Result<Vec<EventArg>>;
}

pub struct FE9FunctionArgsReader;
pub struct FE10FunctionArgsReader;
pub struct FE11FunctionArgsReader;
pub struct FE12FunctionArgsReader;
pub struct FE13FunctionArgsReader;
pub struct FE14FunctionArgsReader;
pub struct FE15FunctionArgsReader;

lazy_static! {
    pub static ref FE10_EVENTS: HashMap<u32, Vec<EventArgType>> = {
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

fn read_v1_and_v2_args(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    signature: Option<&Vec<EventArgType>>,
    param_count: usize,
) -> anyhow::Result<Vec<EventArg>> {
    let mut args = Vec::new();
    if let Some(sig) = signature {
        if sig.len() != param_count {
            return Err(anyhow!(
                "Known signature and function header disagree on arity."
            ));
        }
        for arg in sig {
            let raw = cursor.read_u16::<LittleEndian>()?;
            match arg {
                EventArgType::Str => {
                    let text = read_shift_jis_string(text_data, raw as usize)?;
                    args.push(EventArg::Str(text));
                }
                EventArgType::Int => {
                    args.push(EventArg::Int(raw as i32));
                }
                _ => return Err(anyhow::anyhow!("Unsupported arg type {:?}", arg)),
            }
        }
    } else {
        for _ in 0..param_count {
            args.push(EventArg::Int(cursor.read_i16::<LittleEndian>()? as i32));
        }
    }
    Ok(args)
}

fn read_v3_args(
    cursor: &mut Cursor<&[u8]>,
    text_data: &[u8],
    signature: Option<&Vec<EventArgType>>,
    param_count: usize,
) -> anyhow::Result<Vec<EventArg>> {
    let mut args = Vec::new();
    if let Some(sig) = signature {
        if sig.len() != param_count {
            return Err(anyhow!(
                "Known signature and function header disagree on arity."
            ));
        }
        for arg in sig {
            match arg {
                EventArgType::Str => {
                    let offset = cursor.read_u32::<LittleEndian>()? as usize;
                    let text = read_shift_jis_string(text_data, offset)?;
                    args.push(EventArg::Str(text));
                }
                EventArgType::Int => {
                    args.push(EventArg::Int(cursor.read_i32::<LittleEndian>()?));
                }
                EventArgType::Float => {
                    args.push(EventArg::Float(cursor.read_f32::<LittleEndian>()?));
                }
            }
        }
    } else {
        for _ in 0..param_count {
            args.push(EventArg::Int(cursor.read_i32::<LittleEndian>()?));
        }
    }
    Ok(args)
}

impl FunctionArgsReader for FE9FunctionArgsReader {
    fn read_function_args(
        &self,
        _cursor: &mut Cursor<&[u8]>,
        _text_data: &[u8],
        _function_type: u32,
        _param_count: usize,
    ) -> Result<Vec<EventArg>> {
        todo!()
    }
}

impl FunctionArgsReader for FE10FunctionArgsReader {
    fn read_function_args(
        &self,
        cursor: &mut Cursor<&[u8]>,
        text_data: &[u8],
        function_type: u32,
        param_count: usize,
    ) -> Result<Vec<EventArg>> {
        read_v1_and_v2_args(
            cursor,
            text_data,
            FE10_EVENTS.get(&function_type),
            param_count,
        )
    }
}

impl FunctionArgsReader for FE11FunctionArgsReader {
    fn read_function_args(
        &self,
        _cursor: &mut Cursor<&[u8]>,
        _text_data: &[u8],
        _function_type: u32,
        _param_count: usize,
    ) -> Result<Vec<EventArg>> {
        todo!()
    }
}

impl FunctionArgsReader for FE12FunctionArgsReader {
    fn read_function_args(
        &self,
        _cursor: &mut Cursor<&[u8]>,
        _text_data: &[u8],
        _function_type: u32,
        _param_count: usize,
    ) -> Result<Vec<EventArg>> {
        todo!()
    }
}

impl FunctionArgsReader for FE13FunctionArgsReader {
    fn read_function_args(
        &self,
        _cursor: &mut Cursor<&[u8]>,
        _text_data: &[u8],
        _function_type: u32,
        _param_count: usize,
    ) -> Result<Vec<EventArg>> {
        todo!()
    }
}

impl FunctionArgsReader for FE14FunctionArgsReader {
    fn read_function_args(
        &self,
        cursor: &mut Cursor<&[u8]>,
        text_data: &[u8],
        function_type: u32,
        param_count: usize,
    ) -> Result<Vec<EventArg>> {
        read_v3_args(
            cursor,
            text_data,
            FE14_EVENTS.get(&function_type),
            param_count,
        )
    }
}

impl FunctionArgsReader for FE15FunctionArgsReader {
    fn read_function_args(
        &self,
        _cursor: &mut Cursor<&[u8]>,
        _text_data: &[u8],
        _function_type: u32,
        _param_count: usize,
    ) -> Result<Vec<EventArg>> {
        todo!()
    }
}
