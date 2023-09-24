use log::info;
use rayon::prelude::*;
use std::fs;
use std::path::Path;

use crate::common::{
    collect_files_to_check, connect_output, run_ruff_format, run_ruff_format_check,
};
use crate::settings::Setting;

// Used to test if ruff format crashes or if cause format error

pub fn error_in_format_ttol(setting: &Setting) {
    let files_to_check = collect_files_to_check(&setting.start_dir);

    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all = files_to_check.len();
    files_to_check.into_par_iter().for_each(|original_file_name| {
        let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if idx % 1000 == 0 {
            info!("_____ {idx} / {all}");
        }

        let output = run_ruff_format_check(&original_file_name, false);
        let all = connect_output(&output);
        if is_broken(&all) {
            return;
        }

        let new_file_name = original_file_name.replace(&setting.start_dir, &setting.test_dir);
        let new_file_folder_name = Path::new(&new_file_name).parent().unwrap();
        fs::create_dir_all(new_file_folder_name).unwrap();
        fs::copy(&original_file_name, &new_file_name).unwrap();

        let _output = run_ruff_format(&new_file_name, false);
        let output = run_ruff_format(&new_file_name, false); // Run again to check if not broke anything
        let all = connect_output(&output);

        if is_broken(&all) {
            let broken_file_name = original_file_name.replace(&setting.start_dir, &setting.broken_files_dir);
            let broken_file_folder_name = Path::new(&broken_file_name).parent().unwrap();
            fs::create_dir_all(broken_file_folder_name).unwrap();
            fs::copy(&original_file_name, &broken_file_name).unwrap();
            info!("_________________________________________\nFound error in file: {original_file_name}\n{all}\n_________________________________________");
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
            info!("_____ {idx} / {all}");
        }

        let output = run_ruff_format_check(&file_name, false);
        let all = connect_output(&output).replace("warning: `ruff format` is a work-in-progress, subject to change at any time, and intended only for experimentation.", "");

        if is_broken(&all) {
            info!("_________________________________________\nFound error in file: {file_name}\n{all}\n_________________________________________");
            fs::remove_file(&file_name).unwrap();
        }
    });
}

fn is_broken(all: &str) -> bool {
    all.contains("error") || all.contains("Error")
}
