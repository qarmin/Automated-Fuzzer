use std::process::{Child, Command, Stdio};

use crate::common::try_to_save_file;

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
    if s.contains("INTERNAL ERROR") || s.contains("Traceback") {
        println!("\n_______________ File {full_name} _______________________");
        println!("{s}");
        try_to_save_file(&full_name);
        return false;
    }
    true
}
