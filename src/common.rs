use crate::is_broken;
use crate::ruff::execute_command_and_connect_output;
use rand::prelude::ThreadRng;
use rand::Rng;
use std::cmp::max;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::settings::{BASE_OF_VALID_FILES, BROKEN_FILES_FOR_EACH_FILE, COPY_BROKEN_FILES, INPUT_DIR, MINIMIZATION_ATTEMPTS, OUTPUT_DIR};

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
        .args(format!("-i {BASE_OF_VALID_FILES} -o {INPUT_DIR} -n {BROKEN_FILES_FOR_EACH_FILE} -c true -s").split(' '))
        .args(PYTHON_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_broken_javascript_files() -> Child {
    Command::new("create_broken_files")
        .args(format!("-i {BASE_OF_VALID_FILES} -o {INPUT_DIR} -n {BROKEN_FILES_FOR_EACH_FILE} -c true -s").split(' '))
        .args(JAVASCRIPT_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_new_file_name(old_name: &str) -> String {
    loop {
        let file_name = Path::new(&old_name)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let new_name = format!(
            "{OUTPUT_DIR}/{}{file_name}",
            rand::thread_rng().gen_range(1..10000)
        );
        if !Path::new(&new_name).exists() {
            return new_name;
        }
    }
}

pub fn try_to_save_file(full_name: &str, new_name: &str) -> bool {
    if COPY_BROKEN_FILES {
        if let Err(e) = fs::copy(full_name, new_name) {
            eprintln!("Failed to copy file {full_name}, reason {e}");
            return true;
        };
        return true;
    }
    false
}

pub fn minimize_output(full_name: &str) {
    let Ok(data) = fs::read_to_string(full_name) else {
        println!("INFO: Cannot read content of {full_name}, probably because is not valid UTF-8");
        return;
    };
    let output = execute_command_and_connect_output(full_name);

    if !is_broken(&output) {
        dbg!("");
        return;
    }

    let mut lines = data.lines().map(str::to_string).collect::<Vec<String>>();
    let mut rng = rand::thread_rng();

    let old_line_number = lines.len();

    let mut attempts = MINIMIZATION_ATTEMPTS;
    let mut minimized_output = false;
    while attempts > 0 {
        let Some(new_lines) = minimize_lines(full_name, &lines, &mut rng) else {
            break;
        };
        if new_lines.len() == lines.len() {
            break;
        }

        let output = execute_command_and_connect_output(full_name);
        if is_broken(&output) {
            attempts = MINIMIZATION_ATTEMPTS;
            lines = new_lines;
            minimized_output = true;
        } else {
            attempts -= 1;
        }
    }

    // Restore content of file
    if !minimized_output {
        let mut output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(false)
            .open(full_name)
            .unwrap();
        write!(output_file, "{}", lines.join("\n")).unwrap();
    }

    println!("File {full_name}, minimized from {old_line_number} to {} lines", lines.len());
}

pub fn minimize_lines(
    full_name: &str,
    lines: &Vec<String>,
    rng: &mut ThreadRng,
) -> Option<Vec<String>> {
    if lines.len() <= 3 {
        return None;
    }

    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let number = rng.gen_range(0..=3);
    let mut content;

    let limit = max(1, rng.gen_range(0..(max(1, lines.len() / 5))));

    if number == 0 {
        // Removing from start
        content = lines[limit..].to_vec();
    } else if number == 1 {
        // Removing from end
        let limit = lines.len() - limit;
        content = lines[..limit].to_vec();
    } else if number == 2 {
        // Removing from middle

        let limit_upper ;
        let limit_lower;
        loop {
            let limit1 = rng.gen_range(0..lines.len());
            let limit2 = rng.gen_range(0..lines.len());
            if limit1 > limit2 {
                limit_lower = limit2;
                limit_upper = limit1;
                break;
            } else if limit2 > limit1 {
                limit_lower = limit1;
                limit_upper = limit2;
                break;
            }
        }
        content = lines[limit_lower..limit_upper].to_vec();
    } else {
        // Removing randoms
        content = lines.to_vec();
        let mut indexes_to_remove = HashSet::new();
        for _ in 0..limit {
            indexes_to_remove.insert(rng.gen_range(0..content.len()));
        }

        let mut new_data = Vec::new();
        for (idx, line) in content.into_iter().enumerate(){
            if !indexes_to_remove.contains(&idx) {
                new_data.push(line);
            }
        }
        content = new_data
    }

    write!(output_file, "{}", content.join("\n")).unwrap();
    Some(content)
}
