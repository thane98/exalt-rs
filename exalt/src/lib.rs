mod codegen_common;
mod common;
mod pretty_assembly;
mod v3ds_codegen;
mod v3ds_common;
mod v3ds_disassembly;
mod vgcn_codegen;
mod vgcn_common;
mod vgcn_disassembly;

pub use common::{
    load_opcodes, DisassembledScript, EventArg, EventArgType, FunctionData, Game, Opcode,
    PrettyDisassembledScript,
};
pub use v3ds_codegen::gen_v3ds_code;
pub use v3ds_disassembly::disassemble as disassemble_v3ds;
pub use vgcn_codegen::gen_vgcn_code;
pub use vgcn_disassembly::disassemble as disassemble_vgcn;

pub fn pretty_disassemble_v3ds(raw_script: &[u8]) -> anyhow::Result<String> {
    let function_data = disassemble_v3ds(raw_script)?;
    let pretty = pretty_assembly::prettify(&function_data);
    let yaml = serde_yaml::to_string(&pretty)?;
    Ok(yaml)
}

pub fn pretty_disassemble_vgcn(raw_script: &[u8]) -> anyhow::Result<String> {
    let script = disassemble_vgcn(raw_script)?;
    let pretty = pretty_assembly::prettify(&script.functions);
    let prettified_script = PrettyDisassembledScript {
        script_type: script.script_type,
        functions: pretty,
    };
    let yaml = serde_yaml::to_string(&prettified_script)?;
    Ok(yaml)
}

pub fn pretty_assemble_v3ds(script_name: &str, raw_script: &str) -> anyhow::Result<Vec<u8>> {
    let script: Vec<common::PrettifiedFunctionData> = serde_yaml::from_str(raw_script)?;
    let function_data = pretty_assembly::unprettify(&script)?;
    let raw = gen_v3ds_code(script_name, &function_data)?;
    Ok(raw)
}

pub fn pretty_assemble_vgcn(script_name: &str, raw_script: &str) -> anyhow::Result<Vec<u8>> {
    let script: PrettyDisassembledScript = serde_yaml::from_str(raw_script)?;
    let function_data = pretty_assembly::unprettify(&script.functions)?;
    let raw = gen_vgcn_code(script_name, script.script_type, &function_data)?;
    Ok(raw)
}
