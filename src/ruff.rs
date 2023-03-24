use std::fs;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use rand::Rng;
use rayon::prelude::*;

use crate::{BASE_OF_VALID_FILES, DESTRUCTIVE_RUN, INPUT_DIR, OUTPUT_DIR};

const RUFF_SETTING_FILE: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/ruff.toml";
const RUFF_APP: &str = "ruff";// Rememver to install it with cargo install --path

pub fn create_broken_python_files() -> Child {
    Command::new("create_broken_files")
        // .args(r##"-i BASE_OF_VALID_FILES -o INPUT_DIR -n 1 -c true -s "noqa" "#" "'" "\"" "False" "await" "else" "import" "pass" "None" "break" "except" "in" "raise" "True" "class" "finally" "is" "return" "and" "continue" "for" "lambda" "try" "as" "def" "from" "nonlocal" "while" "assert" "del" "global" "not" "with" "async" "elif" "if" "or" "yield" "__init__" ":" "?" "[" "\"" "\'" "]" "}" "{" "|" "\\" ";" "_" "-" "**" "*" "/" "!" "(" ")" "(True)" "{}" "()" "[]" "noqa" "pylint" "\n" "\t""##.replace("BASE_OF_VALID_FILES", BASE_OF_VALID_FILES).replace("INPUT_DIR", INPUT_DIR).split(' '))
        .args(r##"-i BASE_OF_VALID_FILES -o INPUT_DIR -n 1 -c true -s "noqa" "#" "\t""##.replace("BASE_OF_VALID_FILES", BASE_OF_VALID_FILES).replace("INPUT_DIR", INPUT_DIR).split(' '))
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn get_ruff_run_command(full_name: &str) -> Child {
    Command::new(RUFF_APP)
        .arg(full_name)
        .arg("--config")
        .arg(RUFF_SETTING_FILE)
        .arg("--no-cache")
        .arg("--fix")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn validate_ruff_output(full_name: String, s: String) -> bool{
    if s.contains("RUST_BACKTRACE") || s.contains("This indicates a bug in") {
        let mut lines = s
            .lines()
            .filter(|e|
                {
                    !((e.contains(".py") && e.matches(':').count() >= 3) || e.starts_with("warning: `") || e.starts_with("Ignoring `"))
                })
            .map(String::from)
            .collect::<Vec<String>>();
        lines.dedup();
        let s = lines.into_iter().collect::<String>();


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