use clap::{AppSettings, Clap};

use exalt::Game;

#[derive(Clap)]
#[clap(version = "1.0", author = "thane98")]
#[clap(setting = AppSettings::ColoredHelp)]
struct ExaltCliArgs {
    #[clap(short, long, default_value = "FE14")]
    game: Game,

    #[clap(subcommand)]
    command: Subcommand,
}

#[derive(Clap)]
enum Subcommand {
    Assemble(AssembleArgs),
    Disassemble(DisassembleArgs),
}

#[derive(Clap)]
struct AssembleArgs {
    #[clap(short, long, default_value = "a.cmb")]
    output: String,

    input: String,
}

#[derive(Clap)]
struct DisassembleArgs {
    #[clap(short, long, default_value = "a.yml")]
    output: String,

    input: String,
}

fn main() {
    let args = ExaltCliArgs::parse();
    match args.command {
        Subcommand::Assemble(sub_args) => {
            let raw_file = std::fs::read_to_string(&sub_args.input)
                .expect(&format!("Failed to read input file '{}'.", &sub_args.input));
            let output_path = std::path::Path::new(&sub_args.output);
            let filename = output_path
                .file_name()
                .expect("Failed to parse output path.")
                .to_os_string()
                .into_string()
                .expect("Failed to parse output path.");
            let cmb = exalt::pretty_assemble(&raw_file, &filename, args.game)
                .expect("Code generation failed.");
            std::fs::write(output_path, cmb).expect("Failed to save CMB file.");
        }
        Subcommand::Disassemble(sub_args) => {
            let raw_file = std::fs::read(&sub_args.input)
                .expect(&format!("Failed to read input file '{}'.", &sub_args.input));
            let script = exalt::pretty_disassemble(&raw_file, args.game)
                .expect("Failed to disassemble input file.");
            std::fs::write(sub_args.output, script).expect("Failed to save output.");
        }
    }
}
