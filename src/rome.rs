use std::process::{Child, Command, Stdio};

use crate::common::try_to_save_file;

// const ROME_SETTING_FILE: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/rome.toml"; // TODO not sure how to use it, or what exactly will this help
const ROME_APP: &str = "rome";

pub fn get_rome_run_command(full_name: &str) -> Child {
    Command::new(ROME_APP)
        .arg("check")
        .arg(full_name)
        // .arg("--max-diagnostics") // This probably disable diagnostics instead hiding them from output
        // .arg("0")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn validate_rome_output(full_name: String, s: String) -> bool {
    if s.contains("RUST_BACKTRACE") || s.contains("Rome encountered an unexpected error") {
        let mut lines = s
            .lines()
            // .filter(|e|
            //     {
            //         !((e.contains(".py") && e.matches(':').count() >= 3) || e.starts_with("warning: `") || e.starts_with("Ignoring `"))
            //     })
            .map(String::from)
            .collect::<Vec<String>>();
        lines.dedup(); // Mostly used for removing duplicated empty space
        let s = lines.into_iter().collect::<String>();

        println!("\n_______________ File {full_name} _______________________");
        println!("{s}");
        try_to_save_file(&full_name);
        return false;
    }
    true
}
