use crate::common::{
    collect_only_direct_folders, connect_output, copy_into_smaller_folders, create_broken_files,
    find_broken_files_by_cpython, find_broken_ruff_files, run_ruff_format_check,
};
use crate::settings::Setting;
use log::{error, info};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
enum Broken {
    RuffBrokenCpythonNot(String),
    CpythonBrokenRuffNot(String),
}

pub fn find_parse_difference(settings: &Setting) {
    info!("Removing files from {} and {}", &settings.test_dir, &settings.test_dir2);
    let _ = fs::remove_dir_all(&settings.test_dir);
    fs::create_dir_all(&settings.test_dir).unwrap();
    let _ = fs::remove_dir_all(&settings.test_dir2);
    fs::create_dir_all(&settings.test_dir2).unwrap();
    info!("Removed files from {} and {}", &settings.test_dir, &settings.test_dir2);

    create_broken_files(&settings.start_dir, &settings.test_dir, 1);

    copy_into_smaller_folders(&settings.test_dir, &settings.test_dir2, 1000);

    let folders_to_check = collect_only_direct_folders(&settings.test_dir2, 1);
    let all_folders = folders_to_check.len();

    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);

    let mut files: Vec<_> = folders_to_check
        .into_par_iter()
        .flat_map(|folder_name| {
            let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if idx % 2 == 0 {
                info!("_____ {idx} / {all_folders}");
            }

            let output = run_ruff_format_check(&folder_name, false);
            let all = connect_output(&output);
            let broken_files_ruff: HashSet<String> = find_broken_ruff_files(&all).into_iter().collect();

            let broken_files_cpython: HashSet<String> =
                find_broken_files_by_cpython(&folder_name).into_iter().collect();

            let mut differences = Vec::new();
            for broken_ruff in &broken_files_ruff {
                if broken_files_cpython.contains(broken_ruff) {
                    continue;
                }
                differences.push(Broken::RuffBrokenCpythonNot(broken_ruff.to_string()));
            }

            for broken_cpython in &broken_files_cpython {
                if broken_files_ruff.contains(broken_cpython) {
                    continue;
                }
                differences.push(Broken::CpythonBrokenRuffNot(broken_cpython.to_string()));
            }

            differences
        })
        .collect::<Vec<_>>();

    files.sort();

    let ruff_broken_cpython_not = files
        .iter()
        .filter(|x| matches!(x, Broken::RuffBrokenCpythonNot(_)))
        .count();
    let cpython_broken_ruff_not = files
        .iter()
        .filter(|x| matches!(x, Broken::CpythonBrokenRuffNot(_)))
        .count();

    if ruff_broken_cpython_not > 0 {
        fs::create_dir_all(format!("{}/RuffBroken",settings.broken_files_dir)).unwrap( )
    }
    if cpython_broken_ruff_not > 0 {
        fs::create_dir_all(format!("{}/Cpython",settings.broken_files_dir)).unwrap( )
    }

    files.into_par_iter().for_each(|file| match file {
        Broken::RuffBrokenCpythonNot(full_name) => {
            let file_name = Path::new(&full_name).file_name().unwrap().to_str().unwrap();
            let new_full_name = format!("{}/RuffBroken/{}", &settings.broken_files_dir, file_name);
            fs::copy(&full_name, new_full_name).unwrap();
        }
        Broken::CpythonBrokenRuffNot(full_name) => {
            let file_name = Path::new(&full_name).file_name().unwrap().to_str().unwrap();
            let new_full_name = format!("{}/Cpython/{}", &settings.broken_files_dir, file_name);
            fs::copy(&full_name, new_full_name).unwrap();
        }
    });
    if ruff_broken_cpython_not > 0 {
        error!(
            "Found \t{}\t files that are broken by ruff but not by cpython",
            ruff_broken_cpython_not
        );
    }
    if cpython_broken_ruff_not > 0 {
        error!(
            "Found \t{}\t files that are broken by cpython but not by ruff",
            cpython_broken_ruff_not
        );
    }
}
