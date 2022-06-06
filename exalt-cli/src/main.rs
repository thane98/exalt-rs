use anyhow::Context;
use exalt_ast::Literal;
use exalt_compiler::{CompileRequest, ParseRequest};
use exalt_decompiler::IrTransform;
use std::path::PathBuf;
use strum_macros::EnumString;

use clap::{Parser, Subcommand};
use exalt_lir::{Game, RawScript};

#[derive(EnumString)]
#[strum(serialize_all = "snake_case")]
enum Format {
    Json,
    Yml,
    Ron,
}

#[derive(Parser)]
struct Args {
    #[clap(short, long, value_name = "GAME")]
    game: Game,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Disassemble {
        input: PathBuf,

        #[clap(short, long)]
        output: PathBuf,

        #[clap(short, long)]
        format: Format,
    },
    Assemble {
        input: PathBuf,

        #[clap(short, long)]
        output: PathBuf,

        #[clap(short, long)]
        format: Format,
    },
    Decompile {
        input: PathBuf,

        #[clap(short, long)]
        output: Option<PathBuf>,

        #[clap(short, long)]
        debug: bool,
    },
    Compile {
        input: PathBuf,

        #[clap(short, long)]
        output: Option<PathBuf>,
    },
}

fn disassemble(game: Game, input: PathBuf, output: PathBuf, format: Format) -> anyhow::Result<()> {
    let input = std::fs::read(input).context("failed to read input file")?;
    let script =
        exalt_disassembler::disassemble(&input, game).context("failed to disassemble script")?;
    let raw = match format {
        Format::Json => {
            serde_json::to_string_pretty(&script).context("error serializing script")?
        }
        Format::Yml => serde_yaml::to_string(&script).context("error serializing script")?,
        Format::Ron => ron::ser::to_string_pretty(&script, ron::ser::PrettyConfig::new())
            .context("error serializing script")?,
    };
    std::fs::write(output, raw).context("error writing script to disk")?;
    Ok(())
}

fn assemble(game: Game, input: PathBuf, output: PathBuf, format: Format) -> anyhow::Result<()> {
    let input = std::fs::read(input).context("failed to read input file")?;
    let script_name = output
        .file_name()
        .context("failed to parse file name")?
        .to_string_lossy();
    let script: RawScript = match format {
        Format::Json => serde_json::from_slice(&input).context("failed to parse script")?,
        Format::Yml => serde_yaml::from_slice(&input).context("failed to parse script")?,
        Format::Ron => {
            let text = String::from_utf8(input).context("failed to read input as utf8")?;
            ron::from_str(&text).context("failed to parse script")?
        }
    };
    let raw = exalt_assembler::assemble(&script, &script_name, game)
        .context("failed to assemble script")?;
    std::fs::write(output, raw).context("error writing cmb to disk")?;
    Ok(())
}

fn load_decompiler_transform(game: Game) -> anyhow::Result<Option<IrTransform>> {
    let exe_dir = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("current exe has no parent dir"))?
        .to_path_buf();
    let target = match game {
        Game::FE10 => Some("std/fe10/prelude.exl"),
        Game::FE14 => Some("std/fe14/prelude.exl"),
        _ => None,
    };
    if let Some(target) = target {
        // Load std lib files
        let path = exe_dir.join(target);
        if !path.is_file() {
            println!(
                "WARNING: could not find std library at path '{}'",
                path.display()
            );
            return Ok(None);
        }
        let (_, symbol_table) = exalt_compiler::parse(&ParseRequest { game, target: path })?;

        // Populate the transform from the symbol table
        let mut transform = IrTransform::default();
        for constant in symbol_table.constants() {
            let c = constant.borrow();
            if let Literal::Str(s) = &c.value {
                transform.strings.insert(s.clone(), c.name.clone());
            }
        }
        for (k, v) in symbol_table.aliases() {
            // Flip because aliases are key=friendly name, value=internal name
            transform.functions.insert(v, k);
        }
        if let Some(symbol) = symbol_table.lookup_enum("Event") {
            let e = symbol.borrow();
            for (name, variant) in &e.variants {
                if let Literal::Int(i) = &variant.value {
                    transform
                        .events
                        .insert(*i as usize, format!("Event.{}", name));
                }
            }
        }
        return Ok(Some(transform));
    }
    Ok(None)
}

fn decompile(
    game: Game,
    input: PathBuf,
    output: Option<PathBuf>,
    debug: bool,
) -> anyhow::Result<()> {
    let raw = std::fs::read(&input).context("failed to read input file")?;
    let transform = load_decompiler_transform(game)?;
    let includes = match (game, &transform) {
        (Game::FE10, Some(_)) => vec!["std:fe10:prelude".to_owned()],
        (Game::FE14, Some(_)) => vec!["std:fe14:prelude".to_owned()],
        _ => Vec::new(),
    };
    let script =
        exalt_disassembler::disassemble(&raw, game).context("failed to disassemble script")?;
    let script = exalt_decompiler::decompile(&script, transform, includes, game, debug)
        .context("failed to decompile script")?;
    let output_path = if let Some(path) = output {
        path
    } else {
        let mut path: PathBuf = input
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("input file has no file name"))?
            .into();
        path.set_extension("exl");
        path
    };
    std::fs::write(output_path, script).context("failed to write output file")?;
    Ok(())
}

fn compile(
    game: Game,
    target: PathBuf,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    let request = CompileRequest {
        game,
        target,
        output,
        text_data: None,
    };
    exalt_compiler::compile(&request)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let game = args.game;
    match args.command {
        Commands::Disassemble {
            input,
            output,
            format,
        } => disassemble(game, input, output, format),
        Commands::Assemble {
            input,
            output,
            format,
        } => assemble(game, input, output, format),
        Commands::Decompile {
            input,
            output,
            debug,
        } => decompile(game, input, output, debug),
        Commands::Compile { input, output } => compile(game, input, output),
    }
}
