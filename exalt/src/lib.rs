mod codegen;
mod common;
mod disassembly;
mod pretty_assembly;

pub use common::{
    load_opcodes, EventArg, EventArgType, FunctionData, Game, Opcode, PrettyScript, Script,
};

pub use codegen::CodeGenTextData;

pub fn disassemble(raw_script: &[u8], game: Game) -> anyhow::Result<Script> {
    disassembly::disassemble_script(raw_script, game)
}

pub fn pretty_disassemble(raw_script: &[u8], game: Game) -> anyhow::Result<String> {
    let script = disassemble(raw_script, game)?;
    let pretty = pretty_assembly::prettify(&script.functions);
    let yaml = serde_yaml::to_string(&PrettyScript {
        script_type: script.script_type,
        functions: pretty,
    })?;
    Ok(yaml)
}

pub fn assemble(script: &Script, script_name: &str, game: Game) -> anyhow::Result<Vec<u8>> {
    codegen::generate_script(script, game, script_name, CodeGenTextData::new())
}

pub fn pretty_assemble(script: &str, script_name: &str, game: Game) -> anyhow::Result<Vec<u8>> {
    pretty_assemble_with_text_data(script, script_name, game, CodeGenTextData::new())
}

pub fn pretty_assemble_with_text_data(
    script: &str,
    script_name: &str,
    game: Game,
    text_data: CodeGenTextData,
) -> anyhow::Result<Vec<u8>> {
    let script: PrettyScript = serde_yaml::from_str(script)?;
    let function_data = pretty_assembly::unprettify(&script.functions)?;
    let script = Script {
        script_type: script.script_type,
        functions: function_data,
    };
    let raw = codegen::generate_script(&script, game, script_name, text_data)?;
    Ok(raw)
}
