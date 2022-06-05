use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct IrTransform {
    pub strings: HashMap<String, String>,
    pub functions: HashMap<String, String>,
    pub events: HashMap<usize, String>,
}

impl IrTransform {
    pub fn transform_string(&self, value: &str) -> Option<&str> {
        self.strings.get(value).map(|v| v.as_str())
    }

    pub fn transform_function_name(&self, name: &str) -> Option<&str> {
        self.functions.get(name).map(|v| v.as_str())
    }

    pub fn transform_event(&self, value: usize) -> Option<&str> {
        self.events.get(&value).map(|v| v.as_str())
    }
}
