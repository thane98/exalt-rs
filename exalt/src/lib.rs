mod common;
mod v3ds_codegen;
mod v3ds_common;
mod v3ds_disassembly;
mod vgcn_common;
mod vgcn_disassembly;

pub use common::{load_opcodes, EventArg, EventArgType, FunctionData, Game, Opcode};
pub use v3ds_codegen::gen_v3ds_code;
pub use v3ds_disassembly::disassemble as disassemble_v3ds;
pub use vgcn_disassembly::disassemble as disassemble_vgcn;
