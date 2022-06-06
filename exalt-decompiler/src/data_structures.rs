use crate::ir::{Expr, Stmt};
use anyhow::{bail, Result};

#[derive(Default)]
pub struct ExprStack<'a> {
    pub stack: Vec<Expr<'a>>,
}

impl<'a> ExprStack<'a> {
    pub fn top(&self) -> Option<&'a Expr> {
        self.stack.last()
    }

    pub fn push(&mut self, expr: Expr<'a>) {
        self.stack.push(expr);
    }

    pub fn pop(&mut self) -> Result<Expr<'a>> {
        if self.stack.is_empty() {
            bail!("attempted to pop from empty expr stack")
        } else {
            Ok(self.stack.pop().unwrap())
        }
    }

    pub fn pop_args(&mut self, count: usize) -> Result<Vec<Expr<'a>>> {
        if self.stack.len() < count {
            bail!(
                "wanted to pop '{}' expressions but stack size is '{}'",
                count,
                self.stack.len()
            )
        } else {
            Ok(self.stack.split_off(self.stack.len() - count))
        }
    }
}

#[derive(Default)]
pub struct BlockStack<'a> {
    pub stack: Vec<Vec<Stmt<'a>>>,
}

impl<'a> BlockStack<'a> {
    pub fn push(&mut self) {
        self.stack.push(Vec::new());
    }

    pub fn pop(&mut self) -> Result<Vec<Stmt<'a>>> {
        if self.stack.is_empty() {
            bail!("attempted to pop from empty block stack")
        } else {
            Ok(self.stack.pop().unwrap())
        }
    }

    pub fn line(&mut self, stmt: Stmt<'a>) -> Result<()> {
        if self.stack.is_empty() {
            bail!("attempted to pop from empty block stack")
        } else {
            self.stack.last_mut().unwrap().push(stmt);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum DeclarationRequest {
    Array(usize, usize),
    Var(usize),
}

#[derive(Debug, Default, Clone)]
pub struct VarMetaData {
    pub initialized: bool,
    pub reassigned: bool,
    pub used: bool,
    pub indexed: bool,
    pub parameter: bool,
    pub array_length: usize,
    pub static_array: bool,
}

#[derive(Debug, Clone)]
pub struct VarTracker {
    pub meta_data: Vec<VarMetaData>,
}

impl VarTracker {
    pub fn new(frame_size: usize) -> Self {
        Self {
            meta_data: vec![VarMetaData::default(); frame_size],
        }
    }

    pub fn mark_initialized(&mut self, frame_id: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].initialized = true;
        Ok(())
    }

    pub fn mark_reassigned(&mut self, frame_id: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].reassigned = true;
        Ok(())
    }

    pub fn mark_used(&mut self, frame_id: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].used = true;
        Ok(())
    }

    pub fn mark_indexed(&mut self, frame_id: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].indexed = true;
        Ok(())
    }

    pub fn mark_parameter(&mut self, frame_id: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].parameter = true;
        Ok(())
    }

    pub fn mark_static_array(&mut self, frame_id: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].static_array = true;
        Ok(())
    }

    pub fn set_array_length(&mut self, frame_id: usize, length: usize) -> Result<()> {
        self.check_index(frame_id)?;
        self.meta_data[frame_id].array_length = length;
        Ok(())
    }

    pub fn is_initialized(&self, frame_id: usize) -> Result<bool> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].initialized)
    }

    pub fn is_reassigned(&self, frame_id: usize) -> Result<bool> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].reassigned)
    }

    pub fn is_used(&self, frame_id: usize) -> Result<bool> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].used)
    }

    pub fn is_indexed(&self, frame_id: usize) -> Result<bool> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].indexed)
    }

    pub fn is_parameter(&self, frame_id: usize) -> Result<bool> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].parameter)
    }

    pub fn is_static_array(&self, frame_id: usize) -> Result<bool> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].static_array)
    }

    pub fn get_array_length(&self, frame_id: usize) -> Result<usize> {
        self.check_index(frame_id)?;
        Ok(self.meta_data[frame_id].array_length)
    }

    fn check_index(&self, index: usize) -> Result<()> {
        if index >= self.meta_data.len() {
            bail!("var index '{}' is out of bounds", index)
        } else {
            Ok(())
        }
    }

    pub fn find_empty_array_inits(&mut self) -> Result<()> {
        let mut i = 0;
        while i < self.meta_data.len() {
            let base = i;
            let info = &self.meta_data[i];
            if !info.initialized && info.indexed {
                i += 1;
                while i < self.meta_data.len() {
                    let info = &self.meta_data[i];
                    if !info.initialized && !info.used {
                        self.mark_used(i)?;
                        i += 1;
                    } else {
                        break;
                    }
                }
            } else {
                i += 1;
            }
            if i - base > 1 {
                self.set_array_length(base, i - base)?;
            }
        }
        Ok(())
    }

    pub fn build_declaration_requests(
        &self,
        include_static_arrays: bool,
    ) -> Vec<DeclarationRequest> {
        let mut requests = Vec::new();
        let mut i = 0;
        while i < self.meta_data.len() {
            if self.is_parameter(i).unwrap_or_default() {
                i += 1;
                continue;
            }
            let array_length = self.get_array_length(i).unwrap_or_default();
            if array_length == 0 {
                requests.push(DeclarationRequest::Var(i));
                i += 1;
            } else {
                if !self.is_static_array(i).unwrap_or_default() || include_static_arrays {
                    requests.push(DeclarationRequest::Array(i, array_length));
                }
                i += array_length;
            }
        }
        requests
    }
}
