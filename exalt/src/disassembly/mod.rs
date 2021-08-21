mod args;
mod code;
mod disassembler;
mod function_header;
mod header;
mod resolve_state;

use resolve_state::ResolveState;
use args::{FE9FunctionArgsReader, FE10FunctionArgsReader, FE11FunctionArgsReader, FE12FunctionArgsReader, FE13FunctionArgsReader, FE14FunctionArgsReader, FE15FunctionArgsReader};
use code::{V1CodeDisassembler, V2CodeDisassembler, V3CodeDisassembler};
use function_header::{V1FunctionHeaderReader, V3FunctionHeaderReader};
use header::{V1HeaderReader, V2HeaderReader, V3HeaderReader};

use crate::{Script, Game};

pub fn disassemble_script(script: &[u8], game: Game) -> anyhow::Result<Script> {
    match game {
        Game::FE9 => disassembler::disassemble(
            script, 
            &V1HeaderReader {}, 
            &V1FunctionHeaderReader{}, 
            &FE9FunctionArgsReader{}, 
            &V1CodeDisassembler{},
        ),
        Game::FE10 => disassembler::disassemble(
            script, 
            &V2HeaderReader {}, 
            &V1FunctionHeaderReader{}, 
            &FE10FunctionArgsReader{}, 
            &V2CodeDisassembler{},
        ),
        Game::FE11 => disassembler::disassemble(
            script, 
            &V2HeaderReader {}, 
            &V1FunctionHeaderReader{}, 
            &FE11FunctionArgsReader{}, 
            &V2CodeDisassembler{},
        ),
        Game::FE12 => disassembler::disassemble(
            script, 
            &V2HeaderReader {}, 
            &V1FunctionHeaderReader{}, 
            &FE12FunctionArgsReader{}, 
            &V2CodeDisassembler{},
        ),
        Game::FE13 => disassembler::disassemble(
            script, 
            &V3HeaderReader {}, 
            &V3FunctionHeaderReader{}, 
            &FE13FunctionArgsReader{}, 
            &V3CodeDisassembler{},
        ),
        Game::FE14 => disassembler::disassemble(
            script, 
            &V3HeaderReader {}, 
            &V3FunctionHeaderReader{}, 
            &FE14FunctionArgsReader{}, 
            &V3CodeDisassembler{},
        ),
        Game::FE15 => disassembler::disassemble(
            script, 
            &V3HeaderReader {}, 
            &V3FunctionHeaderReader{}, 
            &FE15FunctionArgsReader{}, 
            &V3CodeDisassembler{},
        ),
    }
}
