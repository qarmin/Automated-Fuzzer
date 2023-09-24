use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

pub type Hash = [u8; 16];

pub fn collect_files_to_check(dir: &str) -> Vec<String> {
    let mut files_to_check = vec![];
    for i in WalkDir::new(dir).into_iter().flatten() {
        let path = i.path();
        if !(path.is_file() && path.to_string_lossy().ends_with(".py")) {
            continue;
        }
        files_to_check.push(path.to_str().unwrap().to_string());
    }
    files_to_check
}
pub fn calculate_hashes_of_files(setting: &Setting) -> HashMap<String, (Hash, usize)> {
    info!("Starting to calculate hashes of files");
    let files_to_check = collect_files_to_check(&setting.test_dir);
    let mut hashmap = HashMap::new();
    files_to_check.into_iter().for_each(|file_name| {
        let file_content = fs::read(&file_name).unwrap();
        let size = file_content.len();
        let hash: Hash = md5::compute(file_content).0;
        hashmap.insert(file_name, (hash, size));
    });
    info!("Finished calculating hashes of files");
    hashmap
}

pub fn check_if_hashes_are_equal(
    hashmap: &mut HashMap<String, (Hash, usize)>,
    setting: &Setting,
) -> Vec<String> {
    info!("Starting to verifying hashes of files");
    let files_to_check = collect_files_to_check(&setting.test_dir);

    let items: Vec<_> = files_to_check.into_par_iter().filter_map(|file_name| {
        let file_content = fs::read(&file_name).unwrap();
        let size = file_content.len();
        let hash: Hash = md5::compute(file_content).0;
        let (original_hash, original_size) = *hashmap.get(&file_name).unwrap();

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

pub fn collect_only_direct_folders(dir: &str, depth: usize) -> Vec<String> {
    let dirs = WalkDir::new(dir)
        .skip_hidden(false)
        .min_depth(depth)
        .max_depth(depth)
        .into_iter()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    info!("Found {} folders", dirs.len());
    dirs
}

pub fn copy_files_from_start_dir_to_test_dir(setting: &Setting, move_in_ci: bool) {
    info!("Starting to copy files to check");
    let moving = setting.ci_run && move_in_ci;
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
        if moving {
            fs::rename(file_name, new_full_name).unwrap();
        } else {
            fs::copy(file_name, new_full_name).unwrap();
        }
    }
    if moving {
        info!("Moved files to {}", &setting.test_dir);
    } else {
        info!("Copied files to {}", &setting.test_dir);
    }
}

pub fn get_diff_between_files(original_file: &str, new_file: &str) -> String {
    let diff_output = std::process::Command::new("diff")
        .arg("-u")
        .arg(original_file)
        .arg(new_file)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    connect_output(&diff_output)
}

pub fn more_detailed_copy<P: AsRef<Path> + std::fmt::Debug, Q: AsRef<Path> + std::fmt::Debug>(
    from: P,
    to: Q,
    do_panic: bool,
) {
    if let Err(e) = fs::copy(&from, &to) {
        if do_panic {
            panic!("Failed to copy file {from:?} to {to:?} with error {e}");
        } else {
            error!("Failed to copy file {from:?} to {to:?} with error {e}");
        }
    }
}

pub fn more_detailed_move<P: AsRef<Path> + std::fmt::Debug, Q: AsRef<Path> + std::fmt::Debug>(
    from: P,
    to: Q,
    do_panic: bool,
) {
    if let Err(e) = fs::rename(&from, &to) {
        if do_panic {
            panic!("Failed to copy file {from:?} to {to:?} with error {e}");
        } else {
            error!("Failed to copy file {from:?} to {to:?} with error {e}");
        }
    }
}

pub fn run_ruff_format_check(item_to_check: &str, print_info: bool) -> Output {
    if print_info {
        info!("Ruff checking format on: {item_to_check}");
    }
    let output = std::process::Command::new("ruff")
        .arg("format")
        .arg(item_to_check)
        .arg("--check")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    if print_info {
        info!("Ruff checked format on: {item_to_check}");
    }
    output
}

pub fn run_ruff_format(item_to_check: &str, print_info: bool) -> Output {
    if print_info {
        info!("Ruff formatted on: {item_to_check}");
    }
    let output = std::process::Command::new("ruff")
        .arg("format")
        .arg(item_to_check)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    if print_info {
        info!("Ruff formatted on: {item_to_check}");
    }
    output
}

pub fn find_broken_ruff_files(all: &str) -> Vec<String> {
    let mut new_files = vec![];
    for i in all.lines() {
        if let Some(s) = i.strip_prefix("error: Failed to format ") {
            if let Some(idx) = s.find(".py") {
                let file_name = &s[..idx + 3];
                new_files.push(file_name.to_string());
            }
        }
    }
    new_files
}

pub fn connect_output(output: &Output) -> String {
    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    format!("{}\n{}", out, err)
}
pub fn check_if_file_is_parsable_by_cpython(
    source_code_file_name: &str,
) -> bool {
    let output = Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(source_code_file_name)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    output.status.success()
}

pub fn copy_into_smaller_folders(source_dir: &str, target_dir: &str, max_elements: usize) {
    let files_to_copy = collect_files_to_check(source_dir);
    files_to_copy.par_chunks(max_elements).enumerate().for_each(|(idx, names)|{
        let target_dir = format!("{}/{}", target_dir, idx);
        let _ = fs::remove_dir_all(&target_dir);
        fs::create_dir_all(&target_dir).unwrap();
        for full_path in names {
            let file_name = Path::new(full_path).file_name().unwrap().to_str().unwrap();
            let new_name = format!("{}/{}", target_dir, file_name);
            fs::copy(full_path, new_name).unwrap();
        }
    })
}