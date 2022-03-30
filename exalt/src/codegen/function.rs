use anyhow::{Context, Result};

use crate::common::{encode_shift_jis, RawFunctionHeader};
use crate::FunctionData;

use super::{
    ArgSerializer, Assembler, CodeGenState, CodeGenTextData, V1ArgSerializer, V1Assembler,
    V2Assembler, V3ArgSerializer, V3Assembler,
};

const V1_AND_V2_FUNCTION_HEADER_SIZE: usize = 0x14;
const V3_FUNCTION_HEADER_SIZE: u32 = 0x18;

pub struct RawFunctionData {
    pub header: RawFunctionHeader,
    pub name: Vec<u8>,
    pub args: Vec<u8>,
    pub code: Vec<u8>,
}

pub trait FunctionSerializer {
    fn to_raw_function(
        &self,
        function: &FunctionData,
        text_data: &mut CodeGenTextData,
    ) -> Result<RawFunctionData>;

    fn serialize_function(
        &self,
        function: &RawFunctionData,
        function_id: u32,
        base_address: u32,
    ) -> Result<Vec<u8>>;
}

pub struct V1FunctionSerializer;

pub struct V2FunctionSerializer;

pub struct V3FunctionSerializer;

fn to_raw_v1_or_v2_with_assembler<T: Assembler>(
    function: &FunctionData,
    text_data: &mut CodeGenTextData,
) -> Result<RawFunctionData> {
    let name_bytes = match &function.name {
        Some(v) => {
            let mut bytes = encode_shift_jis(v)?;
            bytes.push(0);

            // Pad to the next word.
            // Unknown prefix goes between the null terminator and the next word,
            // so it may cover part of the padding.
            while (bytes.len() + function.unknown_prefix.len()) % 4 != 0 {
                bytes.push(0);
            }
            bytes
        }
        None => Vec::new(),
    };
    let raw_args = V1ArgSerializer::serialize_args(function, text_data)
        .context("Failed to write function arguments.")?;
    let code_address = if name_bytes.is_empty() {
        (V1_AND_V2_FUNCTION_HEADER_SIZE + raw_args.len() + function.unknown_prefix.len()) as u32
    } else {
        (V1_AND_V2_FUNCTION_HEADER_SIZE + name_bytes.len() + function.unknown_prefix.len()) as u32
    };
    let name_address = if !name_bytes.is_empty() {
        Some(V1_AND_V2_FUNCTION_HEADER_SIZE as u32)
    } else {
        None
    };
    let raw_function_header = RawFunctionHeader {
        name_address,
        code_address,
        parent_address: None, // TODO: Is this actually used anywhere in FE9-FE12???
        args_address: None,
        frame_size: function.frame_size as u16,
        function_type: function.function_type,
        arity: function.arity,
        param_count: function.args.len() as u8,
        unknown: function.unknown,
        unknown_prefix: function.unknown_prefix.clone(),
        unknown_suffix: function.unknown_suffix.clone(),
    };

    let mut code_gen_state = CodeGenState::new(text_data);
    let mut raw_code: Vec<u8> = Vec::new();
    for op in &function.code {
        T::to_bytes(&op, &mut raw_code, &mut code_gen_state).context(format!(
            "Failed to convert opcode to v3ds format: '{:?}'",
            op
        ))?;
    }
    raw_code.push(0);
    code_gen_state.backpatch(&mut raw_code)?;

    Ok(RawFunctionData {
        header: raw_function_header,
        name: name_bytes,
        args: raw_args,
        code: raw_code,
    })
}

fn serialize_v1_or_v2_function_data(
    function: &RawFunctionData,
    function_id: u32,
    base_address: u32,
) -> Result<Vec<u8>> {
    let header = &function.header;
    let mut raw = Vec::new();
    match &header.name_address {
        Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
        None => raw.extend((0 as u32).to_le_bytes().iter()),
    }
    raw.extend((header.code_address + base_address).to_le_bytes().iter());
    match &header.parent_address {
        Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
        None => raw.extend((0 as u32).to_le_bytes().iter()),
    }
    raw.push(header.function_type);
    raw.push(header.arity);
    raw.push(header.param_count);
    raw.push(header.unknown);
    raw.extend((function_id as u16).to_le_bytes().iter());
    raw.extend((header.frame_size as u16).to_le_bytes().iter());
    raw.extend_from_slice(&function.name);
    raw.extend_from_slice(&function.args);
    raw.extend_from_slice(&header.unknown_prefix);
    raw.extend_from_slice(&function.code);
    raw.extend_from_slice(&header.unknown_suffix);
    Ok(raw)
}

impl FunctionSerializer for V1FunctionSerializer {
    fn to_raw_function(
        &self,
        function: &FunctionData,
        text_data: &mut CodeGenTextData,
    ) -> Result<RawFunctionData> {
        to_raw_v1_or_v2_with_assembler::<V1Assembler>(function, text_data)
    }

    fn serialize_function(
        &self,
        function: &RawFunctionData,
        function_id: u32,
        base_address: u32,
    ) -> Result<Vec<u8>> {
        serialize_v1_or_v2_function_data(function, function_id, base_address)
    }
}

impl FunctionSerializer for V2FunctionSerializer {
    fn to_raw_function(
        &self,
        function: &FunctionData,
        text_data: &mut CodeGenTextData,
    ) -> Result<RawFunctionData> {
        to_raw_v1_or_v2_with_assembler::<V2Assembler>(function, text_data)
    }

    fn serialize_function(
        &self,
        function: &RawFunctionData,
        function_id: u32,
        base_address: u32,
    ) -> Result<Vec<u8>> {
        serialize_v1_or_v2_function_data(function, function_id, base_address)
    }
}

impl FunctionSerializer for V3FunctionSerializer {
    fn to_raw_function(
        &self,
        function: &FunctionData,
        text_data: &mut CodeGenTextData,
    ) -> Result<RawFunctionData> {
        let name_bytes = if let Some(name) = &function.name {
            if function.function_type == 0 && name.contains("::") {
                let mut bytes = encode_shift_jis(name)?;
                bytes.push(0);
                bytes
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        let arg_bytes = V3ArgSerializer::serialize_args(function, text_data)
            .context("Failed to write function arguments.")?;
        let extended_header_address = V3_FUNCTION_HEADER_SIZE;
        let code_address = if name_bytes.is_empty() {
            extended_header_address + (arg_bytes.len() as u32)
        } else {
            extended_header_address + (name_bytes.len() as u32)
        };
        let name_address = if !name_bytes.is_empty() {
            Some(extended_header_address as u32)
        } else {
            None
        };
        let args_address = if function.function_type == 0 {
            None
        } else {
            Some(extended_header_address)
        };
        let raw_function_header = RawFunctionHeader {
            name_address,
            code_address: code_address as u32,
            parent_address: None,
            args_address,
            frame_size: function.frame_size as u16,
            function_type: function.function_type,
            arity: function.arity,
            param_count: function.args.len() as u8,
            unknown: function.unknown,
            unknown_prefix: Vec::new(),
            unknown_suffix: Vec::new(),
        };

        let mut code_gen_state = CodeGenState::new(text_data);
        let mut raw_code: Vec<u8> = Vec::new();
        for op in &function.code {
            V3Assembler::to_bytes(&op, &mut raw_code, &mut code_gen_state).context(format!(
                "Failed to convert opcode to v3ds format: '{:?}'",
                op
            ))?;
        }
        raw_code.push(0);
        code_gen_state.backpatch(&mut raw_code)?;

        Ok(RawFunctionData {
            header: raw_function_header,
            name: name_bytes,
            args: arg_bytes,
            code: raw_code,
        })
    }

    fn serialize_function(
        &self,
        function: &RawFunctionData,
        function_id: u32,
        base_address: u32,
    ) -> Result<Vec<u8>> {
        let header = &function.header;
        let mut raw = Vec::new();
        raw.extend((base_address as u32).to_le_bytes().iter());
        raw.extend((header.code_address + base_address).to_le_bytes().iter());
        raw.push(header.function_type);
        raw.push(header.arity);
        raw.push(header.frame_size as u8);
        raw.push(0); // Padding
        raw.extend(function_id.to_le_bytes().iter());
        match &header.name_address {
            Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
            None => raw.extend((0 as u32).to_le_bytes().iter()),
        }
        match &header.args_address {
            Some(addr) => raw.extend((*addr + base_address).to_le_bytes().iter()),
            None => raw.extend((0 as u32).to_le_bytes().iter()),
        }
        raw.extend_from_slice(&function.name);
        raw.extend_from_slice(&function.args);
        raw.extend_from_slice(&function.code);
        Ok(raw)
    }
}
