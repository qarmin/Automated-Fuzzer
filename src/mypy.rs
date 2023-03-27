use std::process::{Child, Command, Stdio};

use crate::common::{create_new_file_name, try_to_save_file};

pub fn get_mypy_run_command(full_name: &str) -> Child {
    Command::new("mypy")
        .arg(full_name)
        .args("--no-incremental --ignore-missing-imports --disallow-any-unimported --disallow-any-expr --disallow-any-decorated --disallow-any-explicit --disallow-any-generics --disallow-subclassing-any --disallow-untyped-calls --disallow-untyped-defs --disallow-incomplete-defs --check-untyped-defs --disallow-untyped-decorators --warn-redundant-casts --warn-unused-ignores --no-warn-no-return --warn-return-any --warn-unreachable --strict".split(' '))
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn is_broken_mypy(content: &str) -> bool {
    content.contains("INTERNAL ERROR") || content.contains("Traceback")
}

pub fn validate_mypy_output(full_name: String, output: String) -> Option<String> {
    let new_name = create_new_file_name(&full_name);
    println!("\n_______________ File {full_name} saved to {new_name} _______________________");
    println!("{output}");

    if try_to_save_file(&full_name, &new_name) {
        Some(new_name)
    } else {
        None
    }
}
