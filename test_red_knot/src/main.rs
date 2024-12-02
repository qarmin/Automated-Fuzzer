use once_cell::sync::{Lazy, OnceCell};
use rand::random;
use rayon::prelude::*;
use std::env::args;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Instant;
use walkdir::WalkDir;

// const INPUT_FILES_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/AA_PYTHON_VALID_FILES";
// const INPUT_FILES_DIR: &str = "/home/rafal/Downloads/FILES_999";
// const INPUT_FILES_DIR: &str = "/home/rafal/Downloads/aa";
// const FILES_TO_TEST_DIR: &str = "/home/rafal/test/rrrr/A";
// const TEMP_TEST_DIR: &str = "/home/rafal/test/rrrr/B";
// const BROKEN_FILES_DIR: &str = "/home/rafal/test/rrrr/REALLY_BROKEN";
const INPUT_FILES_DIR: &str = "input";
const FILES_TO_TEST_DIR: &str = "temp1";
const TEMP_TEST_DIR: &str = "temp2";
const BROKEN_FILES_DIR: &str = "broken";

const RUN_MINIMIZER: bool = false;
const MAX_FILES: usize = 1000000000;
// const MAX_FILES: usize = 16;

const CREATE_FILES_PER_RUN: usize = 1;

pub static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
pub static MAX_TIME: OnceCell<u64> = OnceCell::new();

pub fn collect_output(output: &Output) -> String {
    let stdout = &output.stdout;
    let stderr = &output.stderr;
    let stdout_str = String::from_utf8_lossy(stdout);
    let stderr_str = String::from_utf8_lossy(stderr);
    format!("{stdout_str}\n{stderr_str}")
}

pub fn check_if_time_exceeded() -> bool {
    let elapsed = START_TIME.elapsed().as_secs();
    let max_time = *MAX_TIME.get().expect("Max time not set");
    elapsed > max_time
}

fn create_broken_files() {
    let input_c = fs::canonicalize(INPUT_FILES_DIR).unwrap().to_string_lossy().to_string();
    let test_c = fs::canonicalize(FILES_TO_TEST_DIR)
        .unwrap()
        .to_string_lossy()
        .to_string();

    assert!(Path::new(&input_c).exists());
    assert!(Path::new(&test_c).exists());

    println!("Creating broken files from {input_c} to {test_c}");

    let child = Command::new("create_broken_files")
        .arg("--input-path")
        .arg(input_c)
        .arg("--output-path")
        .arg(test_c)
        .arg("--number-of-broken-files")
        .arg(CREATE_FILES_PER_RUN.to_string())
        .arg("-m")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");
    let output = child.wait_with_output().expect("Failed to wait on child");
    let _all = collect_output(&output);
}

fn run_red_knot(folder: &str) -> String {
    let command = Command::new("red_knot")
        .arg("--current-directory")
        .arg(folder)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    collect_output(&command)
}

fn test_with_red_knot(files_to_test: &[String]) {
    let temp_folder = format!("{}/{}", TEMP_TEST_DIR, random::<u64>());
    fs::create_dir_all(&temp_folder).unwrap();

    let checked_items_in_group = 400;
    // group check
    let files = files_to_test
        .chunks(checked_items_in_group)
        .into_iter()
        .filter(|group| {
            if check_if_time_exceeded() {
                return false;
            }

            let time = Instant::now();
            for file in *group {
                let only_file_name = Path::new(file).file_name().unwrap().to_string_lossy();
                let new_file_name = format!("{}/{}", temp_folder, only_file_name);
                let _ = fs::copy(file, &new_file_name);
            }
            let all = run_red_knot(&temp_folder);
            let _elapsed = time.elapsed();
            // println!("========================================\n{elapsed:?}\n=================================");
            all.contains("RUST_BACKTRACE")
        })
        .flatten()
        .collect::<Vec<_>>();

    fs::remove_dir_all(&temp_folder).unwrap();
    fs::create_dir_all(&temp_folder).unwrap();

    println!("Before: {} After: {}", files_to_test.len(), files.len());

    for file in files {
        if check_if_time_exceeded() {
            break;
        }

        let only_file_name = Path::new(file).file_name().unwrap().to_string_lossy();
        let extension = Path::new(file).extension().unwrap().to_string_lossy();
        let new_file_name = format!("{}/{}", temp_folder, only_file_name);
        let _ = fs::copy(file, &new_file_name);
        let mut all = run_red_knot(&temp_folder);

        if all.contains("RUST_BACKTRACE") {
            if RUN_MINIMIZER {
                run_minimizer(&file, &new_file_name, &temp_folder);
                all = run_red_knot(&temp_folder);
                if !all.contains("RUST_BACKTRACE") {
                    panic!("Failed to minimize file");
                }
            }
            let start = if all.contains("is always 'load' but got: 'Invalid'") {
                "invalid"
            } else if all.contains("assertion `left == right` failed") {
                "assertion_left_right"
            } else if all.contains("no entry found for key") {
                "no_entry"
            } else if all.contains("Expected the symbol table to create a symbol for every Name node") {
                "expected_symbol"
            } else if all.contains("previous.is_none") {
                "previous"
            } else {
                "other"
                // panic!("Invalid {all}")
            };

            let random_number = random::<u64>();
            let f_n = format!("{start}_{random_number}.{extension}");
            fs::copy(&new_file_name, format!("{}/{f_n}", BROKEN_FILES_DIR)).unwrap();
            fs::write(format!("{}/{start}_{random_number}.log", BROKEN_FILES_DIR), all).unwrap();
        }

        fs::remove_file(&new_file_name).unwrap();
    }

    fs::remove_dir_all(&temp_folder).unwrap();
}

fn run_minimizer(input_file: &str, output_file: &str, output_folder: &str) {
    // minimizer --input-file /home/rafal/Desktop/RunEveryCommand/C/PY_FILE_TEST_25518.py --output-file a.py --command "red_knot" --attempts 1000 --broken-info "RUST_BACKTRACE" -z "not yet implemented" -z "failed to parse" -z "SyntaxError" -z "Sorry:" -z "IndentationError" -k "python3 -m compileall {}" -r -v
    let child = Command::new("minimizer")
        .arg("--input-file")
        .arg(input_file)
        .arg("--output-file")
        .arg(output_file)
        .arg("--command")
        .arg(format!("red_knot --current-directory {}", output_folder))
        .arg("--attempts")
        .arg("20")
        .arg("--broken-info")
        .arg("RUST_BACKTRACE")
        // .arg("-z")
        // .arg("not yet implemented")
        // .arg("-z")
        // .arg("failed to parse")
        // .arg("-z")
        // .arg("SyntaxError")
        // .arg("-z")
        // .arg("Sorry:")
        // .arg("-z")
        // .arg("IndentationError")
        // .arg("-k")
        // .arg("python3 -m compileall {}")
        .arg("-r")
        .arg("-v")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");
    let output = child.wait_with_output().expect("Failed to wait on child");
    let all = collect_output(&output);
    println!("{all}");
}

fn main() {
    // First argument is max time in seconds
    let max_time = args()
        .nth(1)
        .unwrap_or("600".to_string())
        .parse()
        .expect("Invalid max time");
    MAX_TIME.set(max_time).unwrap();
    println!("Max time set to {}", MAX_TIME.get().unwrap());

    // let threads = 8;
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(threads)
    //     .build_global()
    //     .unwrap();
    let threads = std::thread::available_parallelism().map(|e| e.get()).unwrap_or(8);

    loop {
        if check_if_time_exceeded() {
            println!("Exceeded time");
            return;
        }

        let _ = fs::remove_dir_all(FILES_TO_TEST_DIR);
        let _ = fs::remove_dir_all(TEMP_TEST_DIR);
        fs::create_dir_all(FILES_TO_TEST_DIR).unwrap();
        fs::create_dir_all(TEMP_TEST_DIR).unwrap();
        fs::create_dir_all(BROKEN_FILES_DIR).unwrap();

        assert!(Path::new(INPUT_FILES_DIR).exists());
        assert!(Path::new(FILES_TO_TEST_DIR).exists());
        assert!(Path::new(TEMP_TEST_DIR).exists());
        assert!(Path::new(BROKEN_FILES_DIR).exists());

        create_broken_files();

        let broken_files = WalkDir::new(FILES_TO_TEST_DIR)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_string_lossy().to_string())
            .take(MAX_FILES)
            .collect::<Vec<String>>();

        if broken_files.is_empty() {
            eprintln!("No files to test");
            return;
        }

        let chunks = broken_files
            .chunks(broken_files.len() / threads)
            .map(|x| x.to_vec())
            .collect::<Vec<Vec<String>>>();
        chunks.into_par_iter().enumerate().for_each(|(idx, chunk)| {
            println!("Starting chunk {idx} with {} files", chunk.len());
            test_with_red_knot(&chunk);
            println!("Ended chunk {idx}")
        });
    }
}
