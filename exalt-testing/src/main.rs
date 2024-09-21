use byteorder::{LittleEndian, ReadBytesExt};
use clap::Parser;
use encoding_rs::SHIFT_JIS;
use exalt_assembler::CodeGenTextData;
use exalt_compiler::CompileRequest;
use exalt_lir::Game;
use rustc_hash::FxHashMap;
use std::error::Error;
use std::io::Cursor;
use std::iter::FromIterator;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser)]
struct ExaltTestingArgs {
    #[clap(short, long)]
    game: Game,

    #[clap(short, long)]
    compile: bool,

    input: String,
}

fn extract_v1_text_offsets(script: &[u8]) -> anyhow::Result<exalt_assembler::CodeGenTextData> {
    let mut cursor = Cursor::new(script);
    cursor.set_position(0x24);
    let text_data_address = cursor.read_u32::<LittleEndian>()? as usize;
    let function_table_address = cursor.read_u32::<LittleEndian>()? as usize;
    let mut offsets = FxHashMap::default();
    cursor.set_position(text_data_address as u64);
    while cursor.position() < function_table_address as u64 {
        let start = cursor.position() as usize - text_data_address;
        let mut buffer = Vec::new();
        let mut next = cursor.read_u8()?;
        while next != 0 {
            buffer.push(next);
            next = cursor.read_u8()?;
        }
        let (value, _, _) = SHIFT_JIS.decode(&buffer);
        let value = value.to_string();
        if !offsets.contains_key(&value) {
            offsets.insert(value.to_string(), start);
        }
    }
    let raw_text = Vec::from_iter(
        script[text_data_address..function_table_address]
            .iter()
            .cloned(),
    );
    Ok(CodeGenTextData::hard_coded(raw_text, offsets))
}

fn fail_test(err: impl Error) {
    println!("FAILED!");
    println!("{:?}", err);
}

fn get_script_filename(path: &Path) -> anyhow::Result<String> {
    Ok(path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("bad file name"))?
        .to_string_lossy()
        .to_string())
}

fn build_compile_request(
    name: String,
    contents: String,
    game: Game,
    text_data: Option<CodeGenTextData>,
) -> anyhow::Result<CompileRequest> {
    std::fs::write("tmp.exl", contents)?;
    Ok(CompileRequest {
        game,
        target: Path::new("tmp.exl").to_path_buf(),
        output: Some(Path::new(&name).to_path_buf()),
        text_data,
        additional_includes: vec![],
    })
}

fn test_v3_scripts(root: &Path, game: Game) -> anyhow::Result<()> {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "cmb" {
            continue;
        }
        let filename = get_script_filename(path)?;
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path)?;
        match exalt_disassembler::disassemble(&raw_file, game) {
            Ok(script) => match exalt_assembler::assemble(&script, &filename, game) {
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
                    fail_test(&*err);
                    failures += 1;
                }
            },
            Err(err) => {
                fail_test(&*err);
                failures += 1;
            }
        }
    }

    let success_rate = (successes as f64) / (successes + failures) as f64 * 100.0;
    println!(
        "Successes: {}, Failures: {}, Rate: {}%",
        successes as i64, failures as i64, success_rate
    );
    Ok(())
}

fn test_v3_scripts_full(root: &Path, game: Game) -> anyhow::Result<()> {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "cmb" {
            continue;
        }
        let filename = get_script_filename(path)?;
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path)?;
        match exalt_disassembler::disassemble(&raw_file, game) {
            Ok(script) => {
                match exalt_decompiler::decompile(&script, None, Vec::new(), game, true) {
                    Ok(contents) => {
                        match exalt_compiler::compile_to_vec(&build_compile_request(
                            filename, contents, game, None,
                        )?) {
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
                                fail_test(err);
                                failures += 1;
                            }
                        }
                    }
                    Err(err) => {
                        fail_test(&*err);
                        failures += 1;
                    }
                }
            }
            Err(err) => {
                fail_test(&*err);
                failures += 1;
            }
        }
    }

    let success_rate = (successes as f64) / (successes + failures) as f64 * 100.0;
    println!(
        "Successes: {}, Failures: {}, Rate: {}%",
        successes as i64, failures as i64, success_rate
    );
    Ok(())
}

fn test_v1_or_v2_scripts(root: &Path, game: Game) -> anyhow::Result<()> {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "cmb" {
            continue;
        }
        let filename = get_script_filename(path)?;
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path)?;
        let text_data = extract_v1_text_offsets(&raw_file)?;
        match exalt_disassembler::disassemble(&raw_file, game) {
            Ok(script) => match exalt_assembler::assemble_with_hard_coding(
                &script, &filename, game, text_data,
            ) {
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
                    fail_test(&*err);
                    failures += 1;
                }
            },
            Err(err) => {
                fail_test(&*err);
                failures += 1;
            }
        }
    }

    let success_rate = (successes as f64) / (successes + failures) as f64 * 100.0;
    println!(
        "Successes: {}, Failures: {}, Rate: {}%",
        successes as i64, failures as i64, success_rate
    );
    Ok(())
}

fn test_v1_or_v2_scripts_full(root: &Path, game: Game) -> anyhow::Result<()> {
    let mut successes = 0;
    let mut failures = 0;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "cmb" {
            continue;
        }
        let filename = get_script_filename(path)?;
        print!("Testing script '{}'... ", filename);
        let raw_file = std::fs::read(path)?;
        let text_data = extract_v1_text_offsets(&raw_file)?;
        match exalt_disassembler::disassemble(&raw_file, game) {
            Ok(script) => {
                match exalt_decompiler::decompile(&script, None, Vec::new(), game, true) {
                    Ok(contents) => {
                        match exalt_compiler::compile_to_vec(&build_compile_request(
                            filename,
                            contents,
                            game,
                            Some(text_data),
                        )?) {
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
                                fail_test(err);
                                failures += 1;
                            }
                        }
                    }
                    Err(err) => {
                        fail_test(&*err);
                        failures += 1;
                    }
                }
            }
            Err(err) => {
                fail_test(&*err);
                failures += 1;
            }
        }
    }

    let success_rate = (successes as f64) / (successes + failures) as f64 * 100.0;
    println!(
        "Successes: {}, Failures: {}, Rate: {}%",
        successes as i64, failures as i64, success_rate
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = ExaltTestingArgs::parse();
    let input_path = Path::new(&args.input);
    println!(
        "Testing scripts at path '{}' for game '{:?}'",
        input_path.display(),
        args.game
    );
    match args.game {
        Game::FE9 | Game::FE10 | Game::FE11 | Game::FE12 => {
            if args.compile {
                test_v1_or_v2_scripts_full(input_path, args.game)
            } else {
                test_v1_or_v2_scripts(input_path, args.game)
            }
        }
        Game::FE13 | Game::FE14 | Game::FE15 => {
            if args.compile {
                test_v3_scripts_full(input_path, args.game)
            } else {
                test_v3_scripts(input_path, args.game)
            }
        }
    }
}
