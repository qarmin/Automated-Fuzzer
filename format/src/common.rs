use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Stdio;

pub type Hash = [u8; 16];

pub fn collect_files_to_check(dir: &str) -> Vec<String> {
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

pub fn get_diff_between_files(original_file:&str, new_file:&str) -> String {
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
    let out = String::from_utf8_lossy(&diff_output.stdout);
    let err = String::from_utf8_lossy(&diff_output.stderr);
    format!("{}\n{}", out, err).trim().to_string()
}