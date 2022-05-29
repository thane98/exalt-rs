use exalt_lir::CallbackArg;

#[derive(Debug)]
pub struct CmbHeader {
    pub magic_number: u32,
    pub revision: u32,
    pub function_table_address: u32,
    pub text_data_address: u32,
    pub global_frame_size: u32,
    pub init_function_index: Option<u16>,
}

#[derive(Debug)]
pub struct CommonFunctionHeader {
    pub name: Option<String>,
    pub args: Vec<CallbackArg>,
    pub code: u32,
    pub id: u32,
    pub frame_size: u16,
    pub event: u8,
    pub arity: u8,
    pub unknown: u8,
}
