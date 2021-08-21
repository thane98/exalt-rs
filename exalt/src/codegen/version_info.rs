pub struct VersionInfo {
    pub event_table_pointer_address: u64,
    pub text_data_pointer_address: u64,
    pub text_first: bool,
    pub pad_last_event: bool,
}

impl VersionInfo {
    pub fn v1_or_v2() -> Self {
        VersionInfo {
            event_table_pointer_address: 0x28,
            text_data_pointer_address: 0x24,
            text_first: true,
            pad_last_event: true,
        }
    }

    pub fn v3() -> Self {
        VersionInfo {
            event_table_pointer_address: 0x1C,
            text_data_pointer_address: 0x20,
            text_first: false,
            pad_last_event: false,
        }
    }
}
