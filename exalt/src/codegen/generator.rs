use std::io::Cursor;

use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};

use crate::Script;

use super::{CodeGenTextData, FunctionSerializer, RawHeaderBuilder, VersionInfo};

fn dump_text_data(raw: &mut Vec<u8>, text_data: &CodeGenTextData) {
    raw.extend_from_slice(text_data.bytes());
    while raw.len() % 4 != 0 {
        raw.push(0);
    }
}

pub fn generate_script<T: RawHeaderBuilder, U: FunctionSerializer>(
    script: &Script,
    script_name: &str,
    header_builder: &T,
    function_serializer: &U,
    version_info: &VersionInfo,
) -> Result<Vec<u8>> {
    // Build the header.
    let mut raw = header_builder
        .build_header(script_name, script.script_type)
        .context("Failed to build script header.")?;

    // Assemble functions.
    // Can't place them in the output yet since some formats place text data first.
    // Unfortunately, we can't know the size of text data until we assemble every function...
    let mut text_data = CodeGenTextData::new();
    let mut raw_functions = Vec::new();
    for function in &script.functions {
        raw_functions.push(
            function_serializer
                .to_raw_function(function, &mut text_data)
                .with_context(|| format!("Failed to serialize function {:?}", function))?,
        );
    }

    // This will be set in the following code.
    let mut text_data_address = 0;

    // If text goes first, write it now.
    if version_info.text_first {
        text_data_address = raw.len();
        dump_text_data(&mut raw, &text_data);
    }

    // NOW we can place functions.
    let mut function_bytes = Vec::new();
    let mut function_addresses = Vec::new();
    let function_table_length = (raw_functions.len() + 1) * 4;
    for i in 0..raw_functions.len() {
        let function = &raw_functions[i];
        let base_address = (raw.len() + function_table_length + function_bytes.len()) as u32;
        function_addresses.push(base_address);
        function_bytes.extend(function_serializer.serialize_function(
            &function,
            i as u32,
            base_address,
        )?);
        if i != raw_functions.len() - 1 || version_info.pad_last_event {
            while function_bytes.len() % 4 != 0 {
                function_bytes.push(0);
            }
        }
    }
    let event_table_address = raw.len();
    for address in function_addresses {
        raw.extend(address.to_le_bytes().iter());
    }
    raw.extend((0 as u32).to_le_bytes().iter());
    raw.extend_from_slice(&function_bytes);

    // If text goes last, write it now.
    if !version_info.text_first {
        text_data_address = raw.len();
        dump_text_data(&mut raw, &text_data);
    }

    // Fix up the header now that we know where text and event sections were placed.
    let mut cursor = Cursor::new(&mut raw);
    cursor.set_position(version_info.text_data_pointer_address);
    cursor.write_u32::<LittleEndian>(text_data_address as u32)?;
    cursor.set_position(version_info.event_table_pointer_address);
    cursor.write_u32::<LittleEndian>(event_table_address as u32)?;

    Ok(raw)
}
