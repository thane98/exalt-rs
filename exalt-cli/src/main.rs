use clap::{AppSettings, Clap};

#[derive(Clap)]
#[clap(version = "1.0", author = "thane98")]
#[clap(setting = AppSettings::ColoredHelp)]
struct ExaltCliArgs {
    #[clap(subcommand)]
    command: Subcommand
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
}

#[derive(Clap)]
struct DisassembleArgs {
    #[clap(short, long)]
    input: String,

    #[clap(short, long, default_value = "a.yml")]
    output: String,
}

fn main() {
    let args = ExaltCliArgs::parse();
    match args.command {
        Subcommand::Assemble(sub_args) => {
            let raw_file = std::fs::read_to_string(&sub_args.input)
                .expect(&format!("Failed to read input file '{}'.", &sub_args.input));
            let functions: Vec<exalt::V3dsFunctionData> = serde_yaml::from_str(&raw_file)
                .expect("Failed to parse input file as YAML.");
            let output_path = std::path::Path::new(&sub_args.output);
            let filename = output_path.file_name()
                .expect("Failed to parse output path.")
                .to_os_string()
                .into_string()
                .expect("Failed to parse output path.");
            let cmb = exalt::gen_v3ds_code(&filename, &functions)
                .expect("Code generation failed.");
            std::fs::write(output_path, cmb).expect("Failed to save CMB file.");
        }
        Subcommand::Disassemble(sub_args) => {
            let raw_file = std::fs::read(&sub_args.input)
                .expect(&format!("Failed to read input file '{}'.", &sub_args.input));
            let functions: Vec<exalt::V3dsFunctionData> = exalt::disassemble_v3ds(&raw_file)
                .expect("Failed to disassemble input file.");
            let yaml = serde_yaml::to_string(&functions)
                .expect("Failed to serialize result to YAML.");
            std::fs::write(sub_args.output, yaml)
                .expect("Failed to save output.");
        }
    }
}
