#![allow(clippy::upper_case_acronyms)]

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Child;
use std::ptr::write;
use std::sync::atomic::{AtomicU32, Ordering};

use rand::Rng;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::common::{create_broken_javascript_files, create_broken_python_files, minimize_output};
use crate::dlint::{get_dlint_run_command, is_broken_dlint, validate_dlint_output};
use crate::mypy::{get_mypy_run_command, is_broken_mypy, validate_mypy_output};
use crate::oxc::{get_oxc_run_command, is_broken_oxc, validate_oxc_output};
use crate::rome::{get_rome_run_command, is_broken_rome, validate_rome_output};
use crate::ruff::{
    execute_command_and_connect_output, get_ruff_run_command, is_broken_ruff, validate_ruff_output,
};
use crate::settings::{
    CURRENT_MODE, EXTENSIONS, GENERATE_FILES, INPUT_DIR, LOOP_NUMBER, MINIMIZE_OUTPUT, MODES,
};

mod common;
mod mypy;
mod rome;
mod ruff;
mod settings;
mod dlint;
mod oxc;

fn main() {
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(1)
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

            let s = execute_command_and_connect_output(&full_name);

            if is_broken(&s) {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                if let Some(new_file_name) = choose_validate_output_function(full_name, s) {
                    if MINIMIZE_OUTPUT {
                        minimize_output(&new_file_name);
                    }
                };
            }
        });

        println!(
            "\n\nFound {} broken files",
            atomic_broken.load(Ordering::Relaxed)
        );
    }
}

fn choose_validate_output_function(full_name: String, s: String) -> Option<String> {
    match CURRENT_MODE {
        MODES::RUFF => validate_ruff_output(full_name, s),
        MODES::MYPY => validate_mypy_output(full_name, s),
        MODES::ROME => validate_rome_output(full_name, s),
        MODES::DLINT => validate_dlint_output(full_name, s),
        MODES::OXC => validate_oxc_output(full_name, s),
    }
}

fn choose_run_command(full_name: &str) -> Child {
    match CURRENT_MODE {
        MODES::RUFF => get_ruff_run_command(full_name),
        MODES::MYPY => get_mypy_run_command(full_name),
        MODES::ROME => get_rome_run_command(full_name),
        MODES::DLINT  => get_dlint_run_command(full_name),
        MODES::OXC  => get_oxc_run_command(full_name),
    }
}

fn choose_broken_files_creator() -> Child {
    match CURRENT_MODE {
        MODES::RUFF | MODES::MYPY => create_broken_python_files(),
        MODES::ROME | MODES::DLINT| MODES::OXC => create_broken_javascript_files(),
    }
}

fn is_broken(content: &str) -> bool {
    match CURRENT_MODE {
        MODES::RUFF => is_broken_ruff(content),
        MODES::MYPY => is_broken_mypy(content),
        MODES::ROME => is_broken_rome(content),
        MODES::DLINT => is_broken_dlint(content),
        MODES::OXC => is_broken_oxc(content),
    }
}
