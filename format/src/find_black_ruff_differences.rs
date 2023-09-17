use crate::common::{calculate_hashes_of_files, collect_only_direct_folders, Hash};
use crate::settings::Setting;
use jwalk::WalkDir;
use log::info;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Output, Stdio};

pub fn check_differences(setting: &Setting) {
    info!(
        "Start dir {} have {} files",
        &setting.start_dir,
        collect_number_of_files(&setting.start_dir)
    );

    copy_files_from_start_dir_to_test_dir(setting);

    run_ruff(&setting.test_dir);

    let hashed_files = calculate_hashes_of_files(setting);

    run_black(&setting.test_dir, setting);

    let different_files = find_different_files(&hashed_files, &setting.test_dir);

    copy_broken_black_files(different_files, setting);

    copy_broken_files_to_test_dir(setting);

    run_ruff(&setting.test_dir);

    copy_broken_files_with_ruff(setting);

    run_diff_on_files(setting);
}

fn find_different_files(
    hashmap: &HashMap<String, (Hash, usize)>,
    test_dir: &String,
) -> Vec<String> {
    let different_files: Vec<_> = WalkDir::new(test_dir)
        .into_iter()
        .flatten()
        .filter_map(|i| {
            let path = i.path();
            if path.is_dir() {
                return None;
            }
            let file_name = path.to_str().unwrap();
            let file_content = fs::read(file_name).unwrap();
            let size = file_content.len();
            let hash: Hash = md5::compute(file_content).0;
            let (original_hash, original_size) = *hashmap.get(file_name).unwrap();

            if original_hash != hash || original_size != size {
                return Some(file_name.to_string());
            }
            None
        })
        .collect();
    info!("Found {} files with differences", different_files.len());
    different_files
}

fn collect_number_of_files(dir: &str) -> usize {
    WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|i| {
            let path = i.path();
            if path.is_dir() {
                return false;
            }
            let file_name = path.to_str().unwrap();
            file_name.ends_with(".py")
        })
        .count()
}

fn run_diff_on_files(setting: &Setting) {
    let black_files: Vec<_> = WalkDir::new(&setting.broken_files_dir)
        .into_iter()
        .flatten()
        .filter(|i| {
            let path = i.path();
            if path.is_dir() {
                return false;
            }
            let file_name = path.to_str().unwrap();
            file_name.ends_with("_black.py")
        })
        .collect();

    info!("Starting to run diff on files");
    black_files.into_iter().for_each(|black_dir_entry| {
        let black_file = black_dir_entry.path().to_str().unwrap().to_string();
        let ruff_file = black_file.replace("_black", "_ruff");
        let result_file = black_file.replace("_black", "_diff");

        if !(Path::new(&ruff_file).is_file()) {
            let _ = fs::remove_file(&black_file);
            let _ = fs::remove_file(&ruff_file);
            let _ = fs::remove_file(&result_file);
            return;
        }

        let diff_output = std::process::Command::new("diff")
            .arg("-u")
            .arg(&black_file)
            .arg(&ruff_file)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        let out = String::from_utf8_lossy(&diff_output.stdout);
        let err = String::from_utf8_lossy(&diff_output.stderr);
        let all = format!("{}\n{}", out, err);

        if all.trim().is_empty() {
            let _ = fs::remove_file(&black_file);
            let _ = fs::remove_file(&ruff_file);
            let _ = fs::remove_file(&result_file);
        } else {
            fs::write(&result_file, all.trim().as_bytes()).unwrap();
        }
    });

    info!("Finished running diff on files");
}

fn copy_broken_files_to_test_dir(setting: &Setting) {
    let _ = fs::remove_dir_all(&setting.test_dir);
    fs::create_dir_all(&setting.test_dir).unwrap();
    info!("Removed and created test_dir");

    info!("Starting to copy files with differences to test_dir, to check them again with ruff");
    for file in WalkDir::new(&setting.broken_files_dir)
        .into_iter()
        .flatten()
    {
        let path = file.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.to_str().unwrap();
        let new_full_name = file_name.replace(&setting.broken_files_dir, &setting.test_dir);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied files with differences to test_dir");
}

fn copy_broken_black_files(different_files: Vec<String>, setting: &Setting) {
    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();
    info!("Created broken_files_dir");

    info!("Starting to copy black files with differences to broken_files_dir");
    let mut rng = rand::thread_rng();
    for full_name in different_files.into_iter() {
        let new_full_name = format!("{}{}_black.py", &setting.broken_files_dir, rng.gen::<u64>());
        fs::copy(full_name, new_full_name).unwrap();
    }
    info!("Copied black files with differences to broken_files_dir");
}

fn copy_broken_files_with_ruff(setting: &Setting) {
    info!("Starting to copy ruff files with differences to broken_files_dir");
    for i in WalkDir::new(&setting.test_dir).into_iter().flatten() {
        let path = i.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.to_str().unwrap();
        let new_full_name = file_name
            .replace("_black", "_ruff")
            .replace(&setting.test_dir, &setting.broken_files_dir);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied ruff files with differences to broken_files_dir");
}

fn run_ruff(dir: &str) -> Output {
    info!("Running ruff on dir: {dir}",);
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

fn run_black(dir: &str, setting: &Setting) {
    info!("Running black");
    let direct_folders = collect_only_direct_folders(dir, setting.depth);
    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all = direct_folders.len();

    let start_time = std::time::Instant::now();
    let atomic_bool_stopped_search = std::sync::atomic::AtomicBool::new(false);

    direct_folders.into_par_iter().for_each(|folder| {
        if start_time.elapsed().as_secs() > setting.black_timeout {
            if !atomic_bool_stopped_search.load(std::sync::atomic::Ordering::Relaxed) {
                atomic_bool_stopped_search.store(true, std::sync::atomic::Ordering::Relaxed);
                info!("Max seconds to run black reached, stopping");
            }
            return;
        }

        let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if idx % 10 == 0 {
            info!("_____ {idx} / {all}");
        }
        std::process::Command::new("black")
            .arg(folder)
            .arg("--workers=1")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
    });

    info!("Black formatted files");
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
    info!("Copied files to {}", &setting.test_dir);
}
