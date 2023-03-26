use rand::Rng;
use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::settings::{BASE_OF_VALID_FILES, COPY_BROKEN_FILES, INPUT_DIR, OUTPUT_DIR};

const PYTHON_ARGS: &[&str] = &[
    "noqa", "#", "'", "\"", "False", "await", "else", "import", "pass", "None", "break", "except",
    "in", "raise", "True", "class", "finally", "is", "return", "and", "continue", "for", "lambda",
    "try", "as", "def", "from", "nonlocal", "while", "assert", "del", "global", "not", "with",
    "async", "elif", "if", "or", "yield", "__init__", ":", "?", "[", "\"", "\"\"\"", "\'", "]",
    "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".", ",", "==", "!=", "{", "=", "|", "\\", ";",
    "_", "-", "**", "*", "/", "!", "(", ")", "(True)", "{}", "()", "[]", "pylint", "\n", "\t",
];

const JAVASCRIPT_ARGS: &[&str] = &["true", "false"];

pub fn create_broken_python_files() -> Child {
    Command::new("create_broken_files")
        .args(
            r##"-i BASE_OF_VALID_FILES -o INPUT_DIR -n 1 -c true -s"##
                .replace("BASE_OF_VALID_FILES", BASE_OF_VALID_FILES)
                .replace("INPUT_DIR", INPUT_DIR)
                .split(' '),
        )
        .args(PYTHON_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_broken_javascript_files() -> Child {
    Command::new("create_broken_files")
        .args(
            r##"-i BASE_OF_VALID_FILES -o INPUT_DIR -n 1 -c true -s"##
                .replace("BASE_OF_VALID_FILES", BASE_OF_VALID_FILES)
                .replace("INPUT_DIR", INPUT_DIR)
                .split(' '),
        )
        .args(JAVASCRIPT_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn try_to_save_file(full_name: &str) {
    if COPY_BROKEN_FILES {
        let file_name = Path::new(&full_name)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if let Err(e) = fs::copy(
            full_name,
            format!(
                "{OUTPUT_DIR}/{}{file_name}",
                rand::thread_rng().gen_range(1..100000)
            ),
        ) {
            eprintln!("Failed to copy file {full_name}, reason {e}");
        }
    }
}
