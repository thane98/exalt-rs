use clap::{AppSettings, Clap};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct ExaltTestingArgs {
    #[clap(short, long, default_value = "FE14")]
    game: String,

    #[clap(short, long)]
    input: String,
}

fn fail_test(err: &anyhow::Error) {
    println!("FAILED!");
    println!("{:?}", err);
}

fn test_v3ds_scripts(root: &Path) {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap() != "cmb" {
            continue;
        }
        let filename = path
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap();
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path).unwrap();
        match exalt::disassemble_v3ds(&raw_file) {
            Ok(functions) => match exalt::gen_v3ds_code(&filename, &functions) {
                Ok(bytes) => {
                    if bytes != raw_file {
                        println!("FAILED! (output mismatch)");
                        failures += 1;
                    } else {
                        println!("Success");
                        successes += 1;
                    }
                }
                Err(err) => {
                    fail_test(&err);
                    failures += 1;
                }
            },
            Err(err) => {
                fail_test(&err);
                failures += 1;
            }
        }
    }

    let success_rate = (successes as f64) / (successes + failures) as f64 * 100.0;
    println!(
        "Successes: {}, Failures: {}, Rate: {}%",
        successes as i64, failures as i64, success_rate
    );
}

fn test_vgcn_scripts(root: &Path) {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap() != "cmb" {
            continue;
        }
        let filename = path
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap();
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path).unwrap();
        match exalt::pretty_disassemble_vgcn(&raw_file) {
            Ok(script) => match exalt::pretty_assemble_vgcn(&filename, &script) {
                Ok(bytes) => {
                    let expected_table_start = &raw_file[0x24..0x28];
                    let actual_table_start = &bytes[0x24..0x28];

                    if expected_table_start != actual_table_start {
                        println!("FAILED! (output mismatch)");
                        failures += 1;
                        continue;
                    }
                    match exalt::pretty_disassemble_vgcn(&bytes) {
                        Ok(s) => {
                            if s == script {
                                println!("Success");
                                successes += 1;
                            } else {
                                println!("FAILED! (output mismatch)");
                                failures += 1;
                            }
                        }
                        Err(err) => {
                            fail_test(&err);
                            failures += 1;
                        }
                    }
                }
                Err(err) => {
                    fail_test(&err);
                    failures += 1;
                }
            },
            Err(err) => {
                fail_test(&err);
                failures += 1;
            }
        }
    }

    let success_rate = (successes as f64) / (successes + failures) as f64 * 100.0;
    println!(
        "Successes: {}, Failures: {}, Rate: {}%",
        successes as i64, failures as i64, success_rate
    );
}

fn main() {
    let args = ExaltTestingArgs::parse();
    let input_path = Path::new(&args.input);
    println!(
        "Testing scripts at path '{}' for game '{}'.",
        input_path.display(),
        &args.game
    );
    let game = args.game.to_uppercase();
    match game.as_str() {
        "FE10" => test_vgcn_scripts(&input_path),
        "FE14" => test_v3ds_scripts(&input_path),
        _ => println!("No tests available for game '{}'.", args.game),
    }
}
