pub struct VGcnCmbHeader {
    pub magic_number: u32,
    pub revision: u32,
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
    pub params: Vec<i16>,
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
