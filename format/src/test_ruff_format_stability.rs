use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::{Output, Stdio};

type Hash = [u8; 16];

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
    for (idx, file_name) in hashset_with_differences.into_iter().enumerate() {
        let start_file = file_name.replace(&setting.test_dir, &setting.start_dir);
        let broken_file = format!("{}/A{}.py", &setting.broken_files_dir, idx);
        fs::copy(&start_file, &broken_file).unwrap();
        error!("File with difference: {}", start_file);
    }
}

fn check_if_hashes_are_equal(
    hashmap: &mut HashMap<String, (Hash, usize)>,
    setting: &Setting,
) -> Vec<String> {
    info!("Starting to verifying hashes of files");
    let files_to_check = collect_files_to_check(&setting.test_dir);

    let items: Vec<_> = files_to_check.into_par_iter().filter_map(|file_name| {
        let file_content = fs::read(&file_name).unwrap();
        let size = file_content.len();
        let hash: Hash = md5::compute(file_content).0;
        let (original_hash, original_size) = hashmap.get(&file_name).unwrap().clone();

        if original_hash != hash {
            error!("Hashes are not equal for file: {} - before len {original_size}, curr len {size}", file_name);
            return Some((file_name, hash, size));
        }
        None
    }).collect();

    for i in &items {
        let (file_name, hash, size) = i;
        hashmap.insert(file_name.clone(), (*hash, *size));
    }
    items.into_iter().map(|i| i.0).collect()
}

fn calculate_hashes_of_files(setting: &Setting) -> HashMap<String, (Hash, usize)> {
    info!("Starting to calculate hashes of files");
    let files_to_check = collect_files_to_check(&setting.test_dir);

    files_to_check
        .into_iter()
        .map(|file_name| {
            let file_content = fs::read(&file_name).unwrap();
            let size = file_content.len();
            let hash = md5::compute(file_content).0;
            (file_name, (hash, size))
        })
        .collect()
}

fn collect_files_to_check(dir: &str) -> Vec<String> {
    let mut files_to_check = vec![];
    for i in WalkDir::new(dir).into_iter().flatten() {
        let path = i.path();
        if path.is_dir() {
            continue;
        }
        files_to_check.push(path.to_str().unwrap().to_string());
    }
    files_to_check
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
