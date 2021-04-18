mod v3ds_codegen;
mod v3ds_common;
mod v3ds_disassembly;
mod common;

pub use common::{EventArg, EventArgType, Opcode, Game};
pub use v3ds_common::FunctionData as V3dsFunctionData;
pub use v3ds_disassembly::disassemble as disassemble_v3ds;
pub use v3ds_codegen::gen_v3ds_code;
