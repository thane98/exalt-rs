use std::io::Cursor;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

use crate::common::RawFunctionHeader;

pub trait FunctionHeaderReader {
    fn read_function_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<RawFunctionHeader>;
}

pub struct V1FunctionHeaderReader;
pub struct V3FunctionHeaderReader;

fn address_or_none(address: u32) -> Option<u32> {
    if address != 0 {
        Some(address)
    } else {
        None
    }
}

impl FunctionHeaderReader for V1FunctionHeaderReader {
    fn read_function_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<RawFunctionHeader> {
        let name_address = cursor.read_u32::<LittleEndian>()?;
        let code_address = cursor.read_u32::<LittleEndian>()?;
        let parent_address = cursor.read_u32::<LittleEndian>()?;
        let function_type = cursor.read_u8()?;
        let arity = cursor.read_u8()?;
        let param_count = cursor.read_u8()?;
        let _padding = cursor.read_u8()?;
        let _id = cursor.read_u16::<LittleEndian>()?;
        let frame_size = cursor.read_u16::<LittleEndian>()?;
        let args_address = cursor.position() as u32;
        Ok(RawFunctionHeader {
            name_address: address_or_none(name_address),
            code_address,
            parent_address: address_or_none(parent_address),
            args_address: Some(args_address),
            frame_size,
            function_type,
            arity,
            param_count,
        })
    }
}

impl FunctionHeaderReader for V3FunctionHeaderReader {
    fn read_function_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<RawFunctionHeader> {
        let _header_address = cursor.read_u32::<LittleEndian>()?;
        let code_address = cursor.read_u32::<LittleEndian>()?;
        let function_type = cursor.read_u8()?;
        let arity = cursor.read_u8()?;
        let frame_size = cursor.read_u16::<LittleEndian>()?;
        let _id = cursor.read_u32::<LittleEndian>()?;
        let name_address = cursor.read_u32::<LittleEndian>()?;
        let args_address = cursor.read_u32::<LittleEndian>()?;
        Ok(RawFunctionHeader {
            name_address: address_or_none(name_address),
            code_address,
            parent_address: None,
            args_address: address_or_none(args_address),
            frame_size,
            function_type,
            arity,
            param_count: if function_type == 0 { 0 } else { arity },
        })
    }
}
