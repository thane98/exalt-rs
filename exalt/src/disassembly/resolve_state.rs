use std::collections::HashMap;

use crate::common::read_shift_jis_string;

pub struct ResolveState<'a> {
    pub text_data: &'a [u8],
    pub labels: HashMap<usize, String>,
    next_label: usize,
}

impl<'a> ResolveState<'a> {
    pub fn new(text_data: &'a [u8]) -> Self {
        ResolveState {
            text_data,
            labels: HashMap::new(),
            next_label: 0,
        }
    }

    pub fn label(&mut self, addr: usize) -> String {
        match self.labels.get(&addr) {
            Some(l) => l.clone(),
            None => {
                let label = format!("l{}", self.next_label);
                self.next_label += 1;
                self.labels.insert(addr, label.clone());
                label
            }
        }
    }

    pub fn text(&self, offset: usize) -> anyhow::Result<String> {
        read_shift_jis_string(&self.text_data, offset)
    }
}
