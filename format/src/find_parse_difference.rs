use std::collections::HashSet;
use std::fs;
use std::path::Path;
use log::{error, info};
use rayon::prelude::*;
use crate::common::{check_if_file_is_parsable_by_cpython, collect_files_to_check, collect_only_direct_folders, connect_output, copy_into_smaller_folders, find_broken_files_by_cpython, find_broken_ruff_files, run_ruff_format_check};
use crate::settings::Setting;

#[derive(Debug)]
enum Broken {
    RuffBrokenCpythonNot(String),
    CpythonBrokenRuffNot(String),
}

pub fn find_parse_difference(settings: &Setting) {
    let _ = fs::remove_dir_all(&settings.test_dir);
    fs::create_dir_all(&settings.test_dir).unwrap();

    copy_into_smaller_folders(&settings.start_dir, &settings.test_dir, 1000);

    let folders_to_check = collect_only_direct_folders(&settings.test_dir, 1);
    let all_folders = folders_to_check.len();

    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);

    let files: Vec<_> = folders_to_check
        .into_par_iter()
        .flat_map(|folder_name| {
            let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if idx % 2 == 0 {
                info!("_____ {idx} / {all_folders}");
            }

            let output = run_ruff_format_check(&folder_name, false);
            let all = connect_output(&output);
            let broken_files_ruff: HashSet<String> = find_broken_ruff_files(&all).into_iter().collect();

            let broken_files_cpython: HashSet<String> = find_broken_files_by_cpython(&folder_name).into_iter().collect();


            let mut differences = Vec::new();
            dbg!(broken_files_ruff.len(), broken_files_cpython.len());
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

            dbg!(&differences);
            differences
        }).collect::<Vec<_>>();

    for file in files {
        match file {
            Broken::RuffBrokenCpythonNot(full_name) => {
                error!("File {} is not recognized as broken by cpython, but is by ruff", full_name);
                let file_name = Path::new(&full_name).file_name().unwrap().to_str().unwrap();
                let new_full_name = format!("{}/{}", &settings.broken_files_dir, file_name);
                fs::copy(&full_name, &new_full_name).unwrap();
            }
            Broken::CpythonBrokenRuffNot(full_name) => {
                error!("File {} is not recognized as broken by ruff, but is by cpython", full_name);
                let file_name = Path::new(&full_name).file_name().unwrap().to_str().unwrap();
                let new_full_name = format!("{}/{}", &settings.broken_files_dir, file_name);
                fs::copy(&full_name, &new_full_name).unwrap();
            }
        }
    }
}