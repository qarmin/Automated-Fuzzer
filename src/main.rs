use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use rand::Rng;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::ruff::{create_broken_python_files, get_ruff_run_command, validate_ruff_output};

mod ruff;

const INPUT_DIR: &str = "/home/rafal/Desktop/Ruff/Broken";
// const INPUT_DIR: &str = "/home/rafal/Desktop/33";
const OUTPUT_DIR: &str = "/home/rafal/Desktop/Ruff/Broken";
const RUFF_SETTING_FILE: &str = "/home/rafal/Desktop/Ruff/ruff.toml";

const LOOP_NUMBER: u32 = 2;

const DESTRUCTIVE_RUN: bool = false; // Use true, to remove files and copy them, with false, just scanning will happen

const CURRENT_MODE: MODES = MODES::RUFF;

enum MODES {
    RUFF
}

fn main() {
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(16)
    //     .build_global()
    //     .unwrap();

    for i in 1..(LOOP_NUMBER + 1) {
        println!("Starting loop {i} out of all {}", LOOP_NUMBER + 1);

        if DESTRUCTIVE_RUN {
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

        let atomic = AtomicU32::new(0);
        let all = files.len();

        files.into_par_iter().for_each(|mut full_name| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number % 1000 == 0 {
                println!("_____ {number} / {all}")
            }
            let command = choose_run_command(&full_name);
            let output = command.wait_with_output().unwrap();

            let mut out = output.stderr.clone();
            out.push('\n' as u8);
            out.extend(output.stdout);
            let s = String::from_utf8(out).unwrap();
            choose_validate_output_function(full_name,s);
        });
    }
}

fn choose_validate_output_function(full_name: String,s: String) {
    match CURRENT_MODE {
        MODES::RUFF => validate_ruff_output(full_name,s)
    }
}

fn choose_run_command(full_name: &str) -> Child {
    match CURRENT_MODE {
        MODES::RUFF => get_ruff_run_command(full_name)
    }
}

fn choose_broken_files_creator() -> Child {
    match CURRENT_MODE {
        MODES::RUFF => create_broken_python_files()
    }
}