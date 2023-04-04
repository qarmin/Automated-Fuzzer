use std::cmp::max;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::obj::ProgramConfig;

const PYTHON_ARGS: &[&str] = &[
    "noqa", "#", "'", "\"", "False", "await", "else", "import", "pass", "None", "break", "except",
    "in", "raise", "True", "class", "finally", "is", "return", "and", "continue", "for", "lambda",
    "float", "int", "bool", "try", "as", "def", "from", "nonlocal", "while", "assert", "del",
    "global", "not", "with", "async", "elif", "if", "or", "yield", "__init__", ":", "?", "[", "\"",
    "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".", ",", "==", "!=", "{",
    "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)", "{}", "()", "[]",
    "pylint", "\n", "\t",
];

const JAVASCRIPT_ARGS: &[&str] = &[
    ":", "?", "[", "\"", "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".",
    ",", "==", "!=", "{", "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)",
    "{}", "()", "[]", "pylint", "\n", "\t", "#", "'", "\"", "//", "abstract", "arguments", "await",
    "boolean", "break", "byte", "case", "catch", "char", "class", "const", "continue", "debugger",
    "default", "delete", "do", "double", "else", "enum", "eval", "export", "extends", "false",
    "final", "finally", "float", "for", "function", "goto", "if", "implements", "import", "in",
    "instanceof", "int", "interface", "let", "long", "native", "new", "null", "package", "private",
    "protected", "public", "return", "short", "static", "super", "switch", "synchronized", "this",
    "throw", "throws", "transient", "true", "try", "typeof", "var", "void", "volatile", "while",
    "with", "yield",
];

pub fn create_broken_python_files(obj: &dyn ProgramConfig) -> Child {
    let base_of_valid_files = &obj.get_settings().base_of_valid_files;
    let input_dir = &obj.get_settings().input_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    Command::new("create_broken_files")
        .args(format!("-i {base_of_valid_files} -o {input_dir} -n {broken_files_for_each_file} -c true -s").split(' '))
        .args(PYTHON_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_broken_javascript_files(obj: &dyn ProgramConfig) -> Child {
    let base_of_valid_files = &obj.get_settings().base_of_valid_files;
    let input_dir = &obj.get_settings().input_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    Command::new("create_broken_files")
        .args(format!("-i {base_of_valid_files} -o {input_dir} -n {broken_files_for_each_file} -c true -s").split(' '))
        .args(JAVASCRIPT_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_new_file_name(obj: &dyn ProgramConfig,old_name: &str) -> String {
    loop {
        let pat = Path::new(&old_name);
        let extension = pat.extension().unwrap().to_str().unwrap().to_string();
        let file_name = pat.file_stem().unwrap().to_str().unwrap().to_string();
        let new_name = format!(
            "{}/{file_name}{}.{extension}",
            obj.get_settings().output_dir,
            rand::thread_rng().gen_range(1..10000)
        );
        if !Path::new(&new_name).exists() {
            return new_name;
        }
    }
}

pub fn try_to_save_file(obj: &dyn ProgramConfig,full_name: &str, new_name: &str) -> bool {
    if obj.get_settings().copy_broken_files {
        if let Err(e) = fs::copy(full_name, new_name) {
            eprintln!("Failed to copy file {full_name}, reason {e}, (maybe broken files folder not exists?)");
            return true;
        };
        return true;
    }
    false
}

#[allow(clippy::borrowed_box)]
pub fn minimize_output(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let Ok(data) = fs::read_to_string(full_name) else {
        println!("INFO: Cannot read content of {full_name}, probably because is not valid UTF-8");
        return;
    };
    let output = execute_command_and_connect_output(obj, full_name);

    if !obj.is_broken(&output) {
        return;
    }

    let mut lines = data.lines().map(str::to_string).collect::<Vec<String>>();
    let mut rng = rand::thread_rng();

    let old_line_number = lines.len();

    let mut attempts = obj.get_settings().minimization_attempts;
    let mut minimized_output = false;
    while attempts > 0 {
        let Some(new_lines) = minimize_lines(full_name, &lines, &mut rng) else {
            break;
        };
        if new_lines.len() == lines.len() {
            break;
        }

        let output = execute_command_and_connect_output(obj, full_name);
        if obj.is_broken(&output) {
            attempts = obj.get_settings().minimization_attempts;
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

    println!(
        "File {full_name}, minimized from {old_line_number} to {} lines",
        lines.len()
    );
}

#[allow(clippy::comparison_chain)]
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
        // Removing random from middle
        let limit_upper;
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
        for (idx, line) in content.into_iter().enumerate() {
            if !indexes_to_remove.contains(&idx) {
                new_data.push(line);
            }
        }
        content = new_data
    }

    write!(output_file, "{}", content.join("\n")).unwrap();
    Some(content)
}

#[allow(clippy::borrowed_box)]
pub fn execute_command_and_connect_output(obj: &Box<dyn ProgramConfig>, full_name: &str) -> String {
    let command = obj.get_run_command(full_name);
    let output = command.wait_with_output().unwrap();

    let mut out = output.stderr.clone();
    out.push(b'\n');
    out.extend(output.stdout);
    String::from_utf8(out).unwrap()
}