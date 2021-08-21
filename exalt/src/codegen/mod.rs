mod args;
mod code;
mod function;
mod generator;
mod header;
mod state;
mod version_info;

use args::{ArgSerializer, V1ArgSerializer, V3ArgSerializer};
use code::{Assembler, V1Assembler, V2Assembler, V3Assembler};
use function::{
    FunctionSerializer, V1FunctionSerializer, V2FunctionSerializer, V3FunctionSerializer,
};
use header::{RawHeaderBuilder, V1RawHeaderBuilder, V2RawHeaderBuilder, V3RawHeaderBuilder};
use state::{CodeGenState, CodeGenTextData};
use version_info::VersionInfo;

use crate::{Script, Game};

pub fn generate_script(
    script: &Script,
    game: Game,
    script_name: &str,
) -> anyhow::Result<Vec<u8>> {
    match game {
        Game::FE9 => generator::generate_script(
            script,
            script_name,
            &V1RawHeaderBuilder {},
            &V1FunctionSerializer {},
            &VersionInfo::v1_or_v2(),
        ),
        Game::FE10 | Game::FE11 | Game::FE12 => generator::generate_script(
            script,
            script_name,
            &V2RawHeaderBuilder {},
            &V2FunctionSerializer {},
            &VersionInfo::v1_or_v2(),
        ),
        Game::FE13 => generator::generate_script(
            script,
            script_name,
            &V3RawHeaderBuilder {},
            &V3FunctionSerializer {},
            &VersionInfo::v3(),
        ),
        Game::FE14 => generator::generate_script(
            script,
            script_name,
            &V3RawHeaderBuilder {},
            &V3FunctionSerializer {},
            &VersionInfo::v3(),
        ),
        Game::FE15 => generator::generate_script(
            script,
            script_name,
            &V3RawHeaderBuilder {},
            &V3FunctionSerializer {},
            &VersionInfo::v3(),
        ),
    }
}