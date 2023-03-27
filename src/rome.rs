use std::process::{Child, Command, Stdio};

use crate::common::{create_new_file_name, try_to_save_file};

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

pub fn is_broken_rome(content: &str) -> bool {
    content.contains("RUST_BACKTRACE") || content.contains("Rome encountered an unexpected error")
}

pub fn validate_rome_output(full_name: String, output: String) -> Option<String> {
    let new_name = create_new_file_name(&full_name);
    println!("\n_______________ File {full_name} saved to {new_name} _______________________");
    println!("{output}");

    if try_to_save_file(&full_name, &new_name) {
        Some(new_name)
    } else {
        None
    }
}
