use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use rand::Rng;
use rayon::prelude::*;

use crate::{BASE_OF_VALID_FILES, DESTRUCTIVE_RUN, INPUT_DIR, OUTPUT_DIR};


pub fn get_mypy_run_command(full_name: &str) -> Child {
    Command::new("mypy")
        .arg(full_name)
        .args("--no-incremental --ignore-missing-imports --disallow-any-unimported --disallow-any-expr --disallow-any-decorated --disallow-any-explicit --disallow-any-generics --disallow-subclassing-any --disallow-untyped-calls --disallow-untyped-defs --disallow-incomplete-defs --check-untyped-defs --disallow-untyped-decorators --warn-redundant-casts --warn-unused-ignores --no-warn-no-return --warn-return-any --warn-unreachable --strict".split(' '))
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn validate_mypy_output(full_name: String, s: String) -> bool {
    println!("NONE ____ {s}");
    if s.contains("INTERNAL ERROR") || s.contains("Traceback"){
            println!("\n_______________ File {full_name} _______________________");
            println!("{s}");
            if DESTRUCTIVE_RUN {
                let file_name = Path::new(&full_name)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                if let Err(e) = fs::copy(&full_name, format!("{OUTPUT_DIR}/{}{file_name}", rand::thread_rng().gen_range(1..100000))) {
                    eprintln!("Failed to copy file {full_name}, reason {e}");
                }
            }
        return false;
    }
     true
}