use clap::{AppSettings, Clap};
use exalt::Game;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct ExaltTestingArgs {
    #[clap(short, long, default_value = "FE14")]
    game: Game,

    input: String,
}

fn fail_test(err: &anyhow::Error) {
    println!("FAILED!");
    println!("{:?}", err);
}

fn get_script_filename(path: &Path) -> String {
    path.file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap()
}

fn test_v3_scripts(root: &Path, game: Game) {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap() != "cmb" {
            continue;
        }
        let filename = get_script_filename(path);
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path).unwrap();
        match exalt::pretty_disassemble(&raw_file, game) {
            Ok(script) => match exalt::pretty_assemble(&script, &filename, game) {
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

fn test_v1_or_v2_scripts(root: &Path, game: Game) {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap() != "cmb" {
            continue;
        }
        let filename = get_script_filename(path);
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path).unwrap();
        match exalt::pretty_disassemble(&raw_file, game) {
            Ok(script) => match exalt::pretty_assemble(&script, &filename, game) {
                Ok(bytes) => {
                    let expected_table_start = &raw_file[0x24..0x28];
                    let actual_table_start = &bytes[0x24..0x28];

                    if expected_table_start != actual_table_start {
                        println!("FAILED! (output mismatch)");
                        failures += 1;
                        continue;
                    }
                    match exalt::pretty_disassemble(&bytes, game) {
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
        "Testing scripts at path '{}' for game '{:?}'.",
        input_path.display(),
        args.game
    );
    match args.game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => {
            test_v1_or_v2_scripts(&input_path, args.game)
        }
        Game::FE13 | Game::FE14 | Game::FE15 => test_v3_scripts(&input_path, args.game),
    }
}
