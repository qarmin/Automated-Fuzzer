use std::fs;
use std::process::Child;
use std::sync::atomic::{AtomicU32, Ordering};

use rayon::prelude::*;
use walkdir::WalkDir;
use crate::mypy::{get_mypy_run_command, validate_mypy_output};

use crate::ruff::{create_broken_python_files, get_ruff_run_command, validate_ruff_output};

mod ruff;
mod mypy;

// RUFF
const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/Broken";
const DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/InvalidFiles";
const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/ValidFiles";
const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/Broken";

// Mypy
// const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/mypy/Broken";
// const DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/InvalidFiles";
// const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/ValidFiles";
// const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/mypy/Broken";


const INPUT_DIR: &str = if DESTRUCTIVE_RUN { DESTRUCTIVE_INPUT_DIR } else { NON_DESTRUCTIVE_INPUT_DIR };
const LOOP_NUMBER: u32 = 5;
const DESTRUCTIVE_RUN: bool = true ; // Use true, to remove files and copy them, with false, just scanning will happen
const GENERATE_FILES: bool = true;

const CURRENT_MODE: MODES = MODES::RUFF;

enum MODES {
    RUFF,
    MYPY
}

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build_global()
        .unwrap();

    for i in 1..=LOOP_NUMBER {
        println!("Starting loop {i} out of all {LOOP_NUMBER}");

        if DESTRUCTIVE_RUN && GENERATE_FILES {
            let _ = fs::remove_dir_all(INPUT_DIR);
            fs::create_dir_all(INPUT_DIR).unwrap();

            let command = choose_broken_files_creator();
            let _output = command.wait_with_output().unwrap();
            // println!("{}", String::from_utf8(output.stdout).unwrap());
            println!("Generated files to test.");
        }

        let extensions = [".py"];
        let mut files = Vec::new();
        for i in WalkDir::new(INPUT_DIR).into_iter().flatten() {
            let Some(s) = i.path().to_str() else { continue; };
            if extensions.iter().any(|e| s.ends_with(e)) {
                files.push(s.to_string());
            }
        }
        assert!(files.len() > 0);

        let atomic = AtomicU32::new(0);
        let atomic_broken = AtomicU32::new(0);
        let all = files.len();

        files.into_par_iter().for_each(|full_name| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number % 1000 == 0 {
                println!("_____ {number} / {all}")
            }
            let command = choose_run_command(&full_name);
            let output = command.wait_with_output().unwrap();

            let mut out = output.stderr.clone();
            out.push(b'\n');
            out.extend(output.stdout);
            let s = String::from_utf8(out).unwrap();
            if !choose_validate_output_function(full_name, s){
                atomic_broken.fetch_add(1, Ordering::Relaxed);
            }
        });

        println!("\n\nFound {} broken files", atomic_broken.load(Ordering::Relaxed));
    }
}

fn choose_validate_output_function(full_name: String, s: String) -> bool {
    match CURRENT_MODE {
        MODES::RUFF => validate_ruff_output(full_name, s),
        MODES::MYPY => validate_mypy_output(full_name, s)
    }
}

fn choose_run_command(full_name: &str) -> Child {
    match CURRENT_MODE {
        MODES::RUFF => get_ruff_run_command(full_name),
        MODES::MYPY => get_mypy_run_command(full_name),
    }
}

fn choose_broken_files_creator() -> Child {
    match CURRENT_MODE {
        MODES::RUFF | MODES::MYPY => create_broken_python_files()
    }
}