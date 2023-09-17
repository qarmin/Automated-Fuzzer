use crate::common::{calculate_hashes_of_files, check_if_hashes_are_equal};
use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::{Output, Stdio};

pub fn test_ruff_format_stability(setting: &Setting) {
    copy_files_from_start_dir_to_test_dir(setting);
    run_ruff(&setting.test_dir);

    let mut hashset_with_differences = HashSet::new();
    let mut hashmap_with_results = calculate_hashes_of_files(setting);
    for i in 0..3 {
        info!("Iteration: {}", i);
        run_ruff(&setting.test_dir);
        let different_files = check_if_hashes_are_equal(&mut hashmap_with_results, setting);
        hashset_with_differences.extend(different_files);
    }
    info!(
        "Found {} files with differences",
        hashset_with_differences.len()
    );

    // let idx = 0;
    copy_files_to_broken_files(&hashset_with_differences, setting);
}

fn copy_files_to_broken_files(hashset_with_differences: &HashSet<String>, setting: &Setting) {
    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();
    for (idx, file_name) in hashset_with_differences.iter().enumerate() {
        let start_file = file_name.replace(&setting.test_dir, &setting.start_dir);
        let broken_file = format!("{}/A{}.py", &setting.broken_files_dir, idx);
        fs::copy(&start_file, &broken_file).unwrap();
        error!("File with difference: {}", start_file);
    }
}

fn run_ruff(dir: &str) -> Output {
    info!("Running ruff");
    let output = std::process::Command::new("ruff")
        .arg("format")
        .arg(dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    info!("Ruff formatted files");
    output
}

fn copy_files_from_start_dir_to_test_dir(setting: &Setting) {
    info!("Starting to copy files to check");
    let _ = fs::remove_dir_all(&setting.test_dir);
    fs::create_dir_all(&setting.test_dir).unwrap();

    for file in WalkDir::new(&setting.start_dir).into_iter().flatten() {
        let path = file.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.to_str().unwrap();
        let new_full_name = file_name.replace(&setting.start_dir, &setting.test_dir);
        let parent = Path::new(&new_full_name).parent().unwrap();
        let _ = fs::create_dir_all(parent);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied files to check");
}
