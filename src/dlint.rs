use std::process::{Child, Command, Stdio};

use crate::common::{create_new_file_name, try_to_save_file};

const DLINT_APP: &str = "/home/rafal/test/deno_lint/target/release/examples/dlint";

pub fn get_dlint_run_command(full_name: &str) -> Child {
    Command::new(DLINT_APP)
        .arg("run")
        .arg(full_name)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn is_broken_dlint(content: &str) -> bool {
    content.contains("RUST_BACKTRACE")
}

pub fn validate_dlint_output(full_name: String, output: String) -> Option<String> {
    let new_name = create_new_file_name(&full_name);
    println!("\n_______________ File {full_name} saved to {new_name} _______________________");
    println!("{output}");

    if try_to_save_file(&full_name, &new_name) {
        Some(new_name)
    } else {
        None
    }
}
