use std::process::{Child, Command, Stdio};

use crate::choose_run_command;
use crate::common::{create_new_file_name, try_to_save_file};

const RUFF_SETTING_FILE: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/ruff.toml";
const RUFF_APP: &str = "ruff";

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

pub fn execute_command_and_connect_output(full_name: &str) -> String {
    let command = choose_run_command(full_name);
    let output = command.wait_with_output().unwrap();

    let mut out = output.stderr.clone();
    out.push(b'\n');
    out.extend(output.stdout);
    String::from_utf8(out).unwrap()
}

pub fn is_broken_ruff(content: &str) -> bool {
    content.contains("RUST_BACKTRACE") || content.contains("This indicates a bug in")
}

pub fn validate_ruff_output(full_name: String, output: String) -> Option<String> {
    let mut lines = output
        .lines()
        .filter(|e| {
            !((e.contains(".py") && e.matches(':').count() >= 3)
                || e.starts_with("warning: `")
                || e.starts_with("Ignoring `"))
        })
        .map(String::from)
        .collect::<Vec<String>>();
    lines.dedup();
    let output = lines.into_iter().collect::<String>();

    let new_name = create_new_file_name(&full_name);
    println!("\n_______________ File {full_name} saved to {new_name} _______________________");
    println!("{output}");

    if try_to_save_file(&full_name, &new_name) {
        Some(new_name)
    } else {
        None
    }
}
