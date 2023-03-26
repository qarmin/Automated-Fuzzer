#![allow(clippy::upper_case_acronyms)]

use std::fs;
use std::process::Child;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::common::{create_broken_javascript_files, create_broken_python_files};
use crate::mypy::{get_mypy_run_command, validate_mypy_output};
use crate::rome::{get_rome_run_command, validate_rome_output};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::ruff::{get_ruff_run_command, validate_ruff_output};
use crate::settings::{CURRENT_MODE, EXTENSIONS, GENERATE_FILES, INPUT_DIR, LOOP_NUMBER, MODES};

mod common;
mod mypy;
mod rome;
mod ruff;
mod settings;

fn main() {
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(16)
    //     .build_global()
    //     .unwrap();

    for i in 1..=LOOP_NUMBER {
        println!("Starting loop {i} out of all {LOOP_NUMBER}");

        if GENERATE_FILES {
            let _ = fs::remove_dir_all(INPUT_DIR);
            fs::create_dir_all(INPUT_DIR).unwrap();

            let command = choose_broken_files_creator();
            let _output = command.wait_with_output().unwrap();
            // println!("{}", String::from_utf8(output.stdout).unwrap());
            println!("Generated files to test.");
        }

        let mut files = Vec::new();
        for i in WalkDir::new(INPUT_DIR).into_iter().flatten() {
            let Some(s) = i.path().to_str() else { continue; };
            if EXTENSIONS.iter().any(|e| s.ends_with(e)) {
                files.push(s.to_string());
            }
        }
        assert!(!files.is_empty());

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
            if !choose_validate_output_function(full_name, s) {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
            }
        });

        println!(
            "\n\nFound {} broken files",
            atomic_broken.load(Ordering::Relaxed)
        );
    }
}

fn choose_validate_output_function(full_name: String, s: String) -> bool {
    match CURRENT_MODE {
        MODES::RUFF => validate_ruff_output(full_name, s),
        MODES::MYPY => validate_mypy_output(full_name, s),
        MODES::ROME => validate_rome_output(full_name, s),
    }
}

fn choose_run_command(full_name: &str) -> Child {
    match CURRENT_MODE {
        MODES::RUFF => get_ruff_run_command(full_name),
        MODES::MYPY => get_mypy_run_command(full_name),
        MODES::ROME => get_rome_run_command(full_name),
    }
}

fn choose_broken_files_creator() -> Child {
    match CURRENT_MODE {
        MODES::RUFF | MODES::MYPY => create_broken_python_files(),
        MODES::ROME => create_broken_javascript_files(),
    }
}
