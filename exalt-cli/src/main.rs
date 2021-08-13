use clap::{AppSettings, Clap};

#[derive(Clap)]
#[clap(version = "1.0", author = "thane98")]
#[clap(setting = AppSettings::ColoredHelp)]
struct ExaltCliArgs {
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
    #[clap(short, long)]
    input: String,

    #[clap(short, long, default_value = "a.cmb")]
    output: String,

    #[clap(short, long, default_value = "FE14")]
    game: exalt::Game,
}

#[derive(Clap)]
struct DisassembleArgs {
    #[clap(short, long)]
    input: String,

    #[clap(short, long, default_value = "a.yml")]
    output: String,

    #[clap(short, long, default_value = "FE14")]
    game: exalt::Game,
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
            let cmb = match &sub_args.game {
                exalt::Game::FE10 => exalt::pretty_assemble_vgcn(&filename, &raw_file),
                exalt::Game::FE14 => exalt::pretty_assemble_v3ds(&filename, &raw_file),
                _ => panic!("Unsupported game."),
            }
            .expect("Code generation failed.");
            std::fs::write(output_path, cmb).expect("Failed to save CMB file.");
        }
        Subcommand::Disassemble(sub_args) => {
            let raw_file = std::fs::read(&sub_args.input)
                .expect(&format!("Failed to read input file '{}'.", &sub_args.input));
            let script = match &sub_args.game {
                exalt::Game::FE10 => exalt::pretty_disassemble_vgcn(&raw_file),
                exalt::Game::FE14 => exalt::pretty_disassemble_v3ds(&raw_file),
                _ => panic!("Unsupported game."),
            }
            .expect("Failed to disassemble input file.");
            std::fs::write(sub_args.output, script).expect("Failed to save output.");
        }
    }
}
