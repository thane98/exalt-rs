use exalt_compiler::{CompileRequest, CompilerError, SourceFile};
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
        output: PathBuf,

        #[clap(short, long)]
        transform: Option<PathBuf>,

        #[clap(short, long)]
        debug: bool,
    },
    Compile {
        input: PathBuf,

        #[clap(short, long)]
        output: PathBuf,

        #[clap(short, long)]
        include: Vec<PathBuf>,
    },
}

fn disassemble(game: Game, input: PathBuf, output: PathBuf, format: Format) {
    let input = std::fs::read(input).expect("failed to read input file");
    let script =
        exalt_disassembler::disassemble(&input, game).expect("failed to disassemble script");
    let raw = match format {
        Format::Json => serde_json::to_string_pretty(&script).expect("error serializing script"),
        Format::Yml => serde_yaml::to_string(&script).expect("error serializing script"),
        Format::Ron => ron::ser::to_string_pretty(&script, ron::ser::PrettyConfig::new())
            .expect("error serializing script"),
    };
    std::fs::write(output, raw).expect("error writing script to disk");
}

fn assemble(game: Game, input: PathBuf, output: PathBuf, format: Format) {
    let input = std::fs::read(input).expect("failed to read input file");
    let script_name = output
        .file_name()
        .expect("failed to parse file name")
        .to_string_lossy();
    let script: RawScript = match format {
        Format::Json => serde_json::from_slice(&input).expect("failed to parse script"),
        Format::Yml => serde_yaml::from_slice(&input).expect("failed to parse script"),
        Format::Ron => {
            let text = String::from_utf8(input).expect("failed to read input as utf8");
            ron::from_str(&text).expect("failed to parse script")
        }
    };
    let raw =
        exalt_assembler::assemble(&script, &script_name, game).expect("failed to assemble script");
    std::fs::write(output, raw).expect("error writing cmb to disk");
}

fn decompile(game: Game, input: PathBuf, output: PathBuf, transform: Option<PathBuf>, debug: bool) {
    let input = std::fs::read(input).expect("failed to read input file");
    let transform: Option<IrTransform> = if let Some(path) = transform {
        let raw = std::fs::read(path).expect("failed to read transform file");
        serde_yaml::from_slice(&raw).expect("failed to parse transform file")
    } else {
        None
    };
    let script =
        exalt_disassembler::disassemble(&input, game).expect("failed to disassemble script");
    let script = exalt_decompiler::decompile(&script, transform, game, debug)
        .expect("failed to decompile script");
    std::fs::write(output, script).expect("failed writing output to disk");
}

fn compile(game: Game, input: PathBuf, output: PathBuf, includes: Vec<PathBuf>) {
    let input = std::fs::read_to_string(input).expect("failed to read input file");
    let script_name = output
        .file_name()
        .expect("failed to parse file name")
        .to_string_lossy();
    let request = CompileRequest {
        game,
        includes: includes
            .into_iter()
            .map(|p| {
                let contents = std::fs::read_to_string(&p)
                    .unwrap_or_else(|_| panic!("failed to read include file '{}'", p.display()));
                let name = p
                    .file_name()
                    .expect("failed to parse include file name")
                    .to_string_lossy()
                    .to_string();
                SourceFile { name, contents }
            })
            .collect(),
        target: SourceFile {
            name: script_name.to_string(),
            contents: input,
        },
        text_data: None,
    };
    match exalt_compiler::compile(&request) {
        Ok((raw, log)) => {
            std::fs::write(output, raw).expect("failed to write cmb to disk");
            if log.has_warnings() {
                log.print();
            }
        }
        Err(CompilerError::ParseError(log)) => log.print(),
        Err(err) => println!("{}", err),
    }
}

fn main() {
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
            transform,
            debug,
        } => decompile(game, input, output, transform, debug),
        Commands::Compile {
            input,
            output,
            include,
        } => compile(game, input, output, include),
    }
}
