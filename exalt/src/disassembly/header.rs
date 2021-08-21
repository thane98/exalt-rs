use std::io::Cursor;

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub struct CmbHeader {
    pub magic_number: u32,
    pub revision: u32,
    pub script_type: u32,
    pub function_table_address: u32,
    pub text_data_address: u32,
}

pub trait CmbHeaderReader {
    fn read_cmb_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader>;
}

pub struct V1HeaderReader;
pub struct V2HeaderReader;
pub struct V3HeaderReader;

fn validate_header(header: &CmbHeader, expected_revision: u32) -> Result<()> {
    if header.magic_number != 0x626D63 {
        Err(anyhow::anyhow!("Bad CMB magic number."))
    } else if header.revision != expected_revision {
        Err(anyhow::anyhow!(
            "Unsupported revision '{:X}'",
            header.revision
        ))
    } else {
        Ok(())
    }
}

fn read_gcn_header(cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader> {
    let magic_number = cursor.read_u32::<LittleEndian>()?;
    cursor.set_position(0x18);
    let revision = cursor.read_u32::<LittleEndian>()?;
    cursor.set_position(0x22);
    let script_type = cursor.read_u16::<LittleEndian>()? as u32;
    let text_data_address = cursor.read_u32::<LittleEndian>()?;
    let function_table_address = cursor.read_u32::<LittleEndian>()?;
    Ok(CmbHeader {
        magic_number,
        revision,
        script_type,
        text_data_address,
        function_table_address,
    })
}

impl CmbHeaderReader for V1HeaderReader {
    fn read_cmb_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader> {
        let header = read_gcn_header(cursor)?;
        validate_header(&header, 0x20041125)?;
        Ok(header)
    }
}

impl CmbHeaderReader for V2HeaderReader {
    fn read_cmb_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader> {
        let header = read_gcn_header(cursor)?;
        validate_header(&header, 0x20061024)?;
        Ok(header)
    }
}

impl CmbHeaderReader for V3HeaderReader {
    fn read_cmb_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<CmbHeader> {
        let magic_number = cursor.read_u32::<LittleEndian>()?;
        let revision = cursor.read_u32::<LittleEndian>()?;
        cursor.set_position(0x1C);
        let function_table_address = cursor.read_u32::<LittleEndian>()?;
        let text_data_address = cursor.read_u32::<LittleEndian>()?;
        let script_type = cursor.read_u16::<LittleEndian>()? as u32;
        let header = CmbHeader {
            magic_number,
            revision,
            script_type,
            text_data_address,
            function_table_address,
        };
        validate_header(&header, 0x20110819)?;
        Ok(header)
    }
}
