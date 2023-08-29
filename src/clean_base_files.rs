use crate::settings::Setting;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn clean_base_files(settings: &Setting) {
    if settings.extensions.contains(&".py".to_string()) {
        remove_non_parsing_python_files(settings);
    }
}

fn remove_non_parsing_python_files(settings: &Setting) {
    create_new_python_ast_file(&settings.temp_file);

    let broken_files: Vec<String> = collect_base_files(settings);
    let before = broken_files.len();
    let after = AtomicUsize::new(before);
    println!("Found {before} python files to check");
    broken_files.into_par_iter().for_each(|full_name| {
        let output = Command::new("python3")
            .arg(&settings.temp_file)
            .arg(&full_name)
            .output();
        let output = output.unwrap();
        if output.status.success() {
            return;
        };

        println!("File {full_name} is not valid python file, and will be removed");
        fs::remove_file(&full_name).unwrap();
        after.fetch_sub(1, Ordering::Relaxed);
    });

    let after = after.load(Ordering::Relaxed);
    println!(
        "Removed {} python files, left {after} files",
        before - after
    );
}

fn create_new_python_ast_file(temp_file: &str) {
    let code = r#"
import ast
import sys

def parse_python_file(file_path):
    with open(file_path, 'r') as file:
        source_code = file.read()

    try:
        ast.parse(source_code)
        print(f"Syntax is correct in {file_path}")
    except SyntaxError as e:
        print(f"Syntax error in {file_path}: {e}")
        sys.exit(12)
        # raise Exception()

if len(sys.argv) != 2:
    print("Usage: python script.py <file_path>")
else:
    file_path = sys.argv[1]
    parse_python_file(file_path)
"#;
    fs::write(temp_file, code).unwrap();
}

fn collect_base_files(settings: &Setting) -> Vec<String> {
    WalkDir::new(&settings.base_of_valid_files)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if entry.file_type().is_file() {
                return Some(entry.path().to_string_lossy().to_string());
            }
            None
        })
        .collect()
}
