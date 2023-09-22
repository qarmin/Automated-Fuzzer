use crate::common::{
    calculate_hashes_of_files, check_if_hashes_are_equal, collect_files_to_check,
    copy_files_from_start_dir_to_test_dir, get_diff_between_files,
};
use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rand::{random, Rng};
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

#[derive(PartialEq)]
enum CopyMove {
    Copy,
    Move,
}

pub fn test_ruff_format_stability(setting: &Setting) {
    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();

    copy_files_from_start_dir_to_test_dir(setting, true);
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

    let files_to_check = collect_files_to_check(&setting.broken_files_dir);

    let new_folder = format!("{}/BD/", setting.broken_files_dir);
    let _ = fs::remove_dir_all(&new_folder);
    fs::create_dir_all(&new_folder).unwrap();
    copy_move_files_from_folder(&setting.broken_files_dir, &new_folder, CopyMove::Move);

    let new_folder2 = format!("{}/BD2/", setting.broken_files_dir);
    let _ = fs::remove_dir_all(&new_folder2);
    fs::create_dir_all(&new_folder2).unwrap();
    copy_move_files_from_folder(&new_folder, &new_folder2, CopyMove::Copy);

    let new_folder3 = format!("{}/BD3/", setting.broken_files_dir);
    let _ = fs::remove_dir_all(&new_folder3);
    fs::create_dir_all(&new_folder3).unwrap();
    copy_move_files_from_folder(&new_folder, &new_folder3, CopyMove::Copy);

    run_ruff(&new_folder2);

    run_ruff(&new_folder3);
    run_ruff(&new_folder3);

    // Must be non unique, to be able to use easily "cat *.txt > diff.txt" when collecting output from multiple directories
    let mut diff_file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(format!(
            "{}/diff{}.txt",
            setting.broken_files_dir,
            random::<u64>()
        ))
        .unwrap();

    for non_existent in files_to_check {
        let first_file = format!(
            "{}/BD/{}",
            setting.broken_files_dir,
            Path::new(&non_existent)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        );
        let second_file = first_file.replace("/BD", "/BD2");
        let third_file = first_file.replace("/BD", "/BD3");

        // let diff1 = get_diff_between_files(&first_file, &second_file);
        let diff2 = get_diff_between_files(&second_file, &third_file);
        // writeln!(diff_file, "{}", diff1).unwrap();
        // writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        // writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        writeln!(diff_file, "{}", diff2).unwrap();
        writeln!(
            diff_file,
            "//////////////////////////////////////////////////////"
        )
        .unwrap();
        writeln!(
            diff_file,
            "//////////////////////////////////////////////////////"
        )
        .unwrap();
        writeln!(
            diff_file,
            "//////////////////////////////////////////////////////"
        )
        .unwrap();
        writeln!(
            diff_file,
            "//////////////////////////////////////////////////////"
        )
        .unwrap();
        writeln!(
            diff_file,
            "//////////////////////////////////////////////////////"
        )
        .unwrap();
    }
}

fn copy_move_files_from_folder(original: &str, new: &str, copy: CopyMove) {
    let _ = fs::remove_dir_all(new);
    fs::create_dir_all(new).unwrap();

    for i in WalkDir::new(original).into_iter().flatten() {
        let path = i.path();
        if path.is_dir() {
            continue;
        }
        let new_file_name = path.to_str().unwrap().replace(original, new);
        if copy == CopyMove::Copy {
            if let Err(e) = fs::copy(&path, &new_file_name) {
                panic!("Failed to copy file {path:?} to {new_file_name} with error {e}")
            }
        } else {
            if let Err(e) = fs::rename(&path, &new_file_name) {
                panic!("Failed to copy file {path:?} to {new_file_name} with error {e}")
            }
        }
    }
}

fn copy_files_to_broken_files(hashset_with_differences: &HashSet<String>, setting: &Setting) {
    let mut rng = rand::thread_rng();
    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();
    for file_name in hashset_with_differences {
        let start_file = file_name.replace(&setting.test_dir, &setting.start_dir);
        let broken_file = format!("{}/A_{}.py", &setting.broken_files_dir, rng.gen::<u64>());
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
