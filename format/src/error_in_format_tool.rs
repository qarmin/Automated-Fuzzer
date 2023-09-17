use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::common::collect_files_to_check;
use crate::settings::Setting;

// Used to test if ruff format crashes or if cause format error

pub fn error_in_format_ttol(setting: &Setting) {
    let files_to_check = collect_files_to_check(&setting.start_dir);

    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all = files_to_check.len();
    files_to_check.into_par_iter().for_each(|original_file_name| {
        let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if idx % 1000 == 0 {
            println!("_____ {idx} / {all}");
        }

        let output = command_check_output(&original_file_name);
        let all = connect_output(&output);
        if is_broken(&all) {
            return;
        }

        let new_file_name = original_file_name.replace(&setting.start_dir, &setting.test_dir);
        let new_file_folder_name = Path::new(&new_file_name).parent().unwrap();
        fs::create_dir_all(new_file_folder_name).unwrap();
        fs::copy(&original_file_name, &new_file_name).unwrap();

        let _output = command_full_output(&new_file_name);
        let output = command_full_output(&new_file_name); // Run again to check if not broke anything
        let all = connect_output(&output);

        if is_broken(&all) {
            let broken_file_name = original_file_name.replace(&setting.start_dir, &setting.broken_files_dir);
            let broken_file_folder_name = Path::new(&broken_file_name).parent().unwrap();
            fs::create_dir_all(broken_file_folder_name).unwrap();
            fs::copy(&original_file_name, &broken_file_name).unwrap();
            println!("_________________________________________\nFound error in file: {original_file_name}\n{all}\n_________________________________________");
        }
    });
}

#[allow(dead_code)]
fn remove_invalid_files(setting: &Setting) {
    let files_to_check = collect_files_to_check(&setting.start_dir);

    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all = files_to_check.len();
    files_to_check.into_par_iter().for_each(|file_name| {
        let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if idx % 1000 == 0 {
            println!("_____ {idx} / {all}");
        }

        let output = command_check_output(&file_name);
        let all = connect_output(&output).replace("warning: `ruff format` is a work-in-progress, subject to change at any time, and intended only for experimentation.", "");

        if is_broken(&all) {
            println!("_________________________________________\nFound error in file: {file_name}\n{all}\n_________________________________________");
            fs::remove_file(&file_name).unwrap();
        }
    });
}

fn is_broken(all: &str) -> bool {
    all.contains("error") || all.contains("Error")
}

fn command_check_output(file_name: &str) -> Output {
    let command = Command::new("ruff")
        .arg("format")
        .arg(file_name)
        .arg("--check")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    command.wait_with_output().unwrap()
}

fn command_full_output(file_name: &str) -> Output {
    let command = Command::new("ruff")
        .arg("format")
        .arg(file_name)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    command.wait_with_output().unwrap()
}

fn connect_output(output: &Output) -> String {
    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    format!("{}\n{}", out, err)
}
