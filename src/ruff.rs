use std::process::{Child, Command, Stdio};

use crate::common::try_to_save_file;

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

pub fn validate_ruff_output(full_name: String, s: String) -> bool {
    if s.contains("RUST_BACKTRACE") || s.contains("This indicates a bug in") {
        let mut lines = s
            .lines()
            .filter(|e| {
                !((e.contains(".py") && e.matches(':').count() >= 3)
                    || e.starts_with("warning: `")
                    || e.starts_with("Ignoring `"))
            })
            .map(String::from)
            .collect::<Vec<String>>();
        lines.dedup();
        let s = lines.into_iter().collect::<String>();

        println!("\n_______________ File {full_name} _______________________");
        println!("{s}");
        try_to_save_file(&full_name);
        return false;
    }
    true
}
