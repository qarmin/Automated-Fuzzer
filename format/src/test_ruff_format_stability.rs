use crate::common::{
    calculate_hashes_of_files, check_if_hashes_are_equal, collect_files_to_check,
    copy_files_from_start_dir_to_test_dir, get_diff_between_files, more_detailed_copy, more_detailed_move,
    remove_empty_folders_from_broken_files_dir, run_ruff_format,
};
use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rand::{random, Rng};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(PartialEq, Copy, Clone)]
enum CopyMove {
    Copy,
    Move,
}

pub fn test_ruff_format_stability(setting: &Setting) {
    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();

    copy_files_from_start_dir_to_test_dir(setting, false);
    run_ruff_format(&setting.test_dir, true);

    let mut hashset_with_differences = HashSet::new();
    let mut hashmap_with_results = calculate_hashes_of_files(setting);
    for i in 0..3 {
        info!("Iteration: {}", i);
        run_ruff_format(&setting.test_dir, true);
        let different_files = check_if_hashes_are_equal(&mut hashmap_with_results, setting);
        hashset_with_differences.extend(different_files);
    }
    info!("Found {} files with differences", hashset_with_differences.len());

    copy_files_to_broken_files(&hashset_with_differences, setting);

    let files_to_check = collect_files_to_check(&setting.broken_files_dir);

    let new_folder = format!("{}BD/", setting.broken_files_dir);
    let _ = fs::remove_dir_all(&new_folder);
    fs::create_dir_all(&new_folder).unwrap();
    copy_move_files_from_folder_with_deleting(&setting.broken_files_dir, &new_folder, CopyMove::Move);

    let new_folder2 = format!("{}BD2/", setting.broken_files_dir);
    let _ = fs::remove_dir_all(&new_folder2);
    fs::create_dir_all(&new_folder2).unwrap();
    copy_move_files_from_folder_with_deleting(&new_folder, &new_folder2, CopyMove::Copy);

    let new_folder3 = format!("{}BD3/", setting.broken_files_dir);
    let _ = fs::remove_dir_all(&new_folder3);
    fs::create_dir_all(&new_folder3).unwrap();
    copy_move_files_from_folder_with_deleting(&new_folder, &new_folder3, CopyMove::Copy);

    run_ruff_format(&new_folder2, true);

    run_ruff_format(&new_folder3, true);
    run_ruff_format(&new_folder3, true);

    // Must be non unique, to be able to use easily "cat *.txt > diff.txt" when collecting output from multiple directories
    let mut diff_file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(format!("{}/diff{}.txt", setting.broken_files_dir, random::<u64>()))
        .unwrap();

    for non_existent in files_to_check {
        let first_file = format!(
            "{}BD/{}",
            setting.broken_files_dir,
            Path::new(&non_existent).file_name().unwrap().to_str().unwrap()
        );
        let second_file = first_file.replace("/BD", "/BD2");
        let third_file = first_file.replace("/BD", "/BD3");

        // let diff1 = get_diff_between_files(&first_file, &second_file);
        let diff2 = get_diff_between_files(&second_file, &third_file);
        // writeln!(diff_file, "{}", diff1).unwrap();
        // writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        // writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        writeln!(diff_file, "{diff2}").unwrap();
        writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
        writeln!(diff_file, "//////////////////////////////////////////////////////").unwrap();
    }

    remove_empty_folders_from_broken_files_dir(setting);
}

fn copy_move_files_from_folder_with_deleting(original: &str, new: &str, copy: CopyMove) {
    info!("Starting to copy/move files to check, from {original} to {new}");
    let _ = fs::remove_dir_all(new);
    fs::create_dir_all(new).unwrap();

    let files_to_move = WalkDir::new(original)
        .into_iter()
        .flatten()
        .filter(|e| e.path().is_file())
        .collect::<Vec<_>>();

    let moved_copied_files = files_to_move.len();

    files_to_move.into_par_iter().for_each(|i| {
        let path = i.path();
        let new_file_name = path.to_str().unwrap().replace(original, new);
        match copy {
            CopyMove::Copy => {
                more_detailed_copy(&path, &new_file_name, true);
            }
            CopyMove::Move => {
                more_detailed_move(&path, &new_file_name, true);
            }
        }
    });

    info!("Copied/moved {moved_copied_files} files, from {original} to {new}");
}

fn copy_files_to_broken_files(hashset_with_differences: &HashSet<String>, setting: &Setting) {
    info!(
        "Starting to copy files with differences to {}",
        setting.broken_files_dir
    );
    let mut rng = rand::thread_rng();
    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();

    for file_name in hashset_with_differences {
        let start_file = file_name.replace(&setting.test_dir, &setting.start_dir);
        let broken_file = format!("{}/A_{}.py", &setting.broken_files_dir, rng.gen::<u64>());
        more_detailed_copy(&start_file, &broken_file, true);
        error!("File with difference: {}", start_file);
    }
    info!("Copied files with differences to {}", setting.broken_files_dir);
}
