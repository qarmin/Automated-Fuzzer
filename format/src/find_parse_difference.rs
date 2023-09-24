use std::fs;
use log::info;
use rayon::prelude::*;
use crate::common::{check_if_file_is_parsable_by_cpython, collect_files_to_check, connect_output, find_broken_ruff_files, run_ruff_format_check};
use crate::settings::Setting;

#[derive(Debug)]
enum Broken {
    RuffBrokenCpythonNot(String),
    CpythonBrokenRuffNot(String),
}

pub fn find_parse_difference(settings: &Setting) {
    let files_to_check = collect_files_to_check(&settings.start_dir);
    let all_files = files_to_check.len();

    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);

    let files = files_to_check
        .into_par_iter()
        .filter_map(|file_name| {
            let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if idx % 100 == 0 {
                info!("_____ {idx} / {all_files}");
            }

            let output = run_ruff_format_check(&file_name, false);
            let all = connect_output(&output);
            let ruff_think_if_is_broken = !find_broken_ruff_files(&all).is_empty();

            let cpython_think_if_is_broken = !check_if_file_is_parsable_by_cpython(&file_name);

            if ruff_think_if_is_broken && !cpython_think_if_is_broken {
                info!("Ruff think that file is broken, but cpython think that file is parsable: {}", file_name);
                Some(Broken::RuffBrokenCpythonNot(file_name))
            } else if cpython_think_if_is_broken && !ruff_think_if_is_broken {
                info!("Cpython think that file is broken, but ruff think that file is parsable: {}", file_name);
                Some(Broken::CpythonBrokenRuffNot(file_name))
            } else {
                None
            }
        }).collect::<Vec<_>>();
    dbg!(files);

    let _ = fs::remove_dir_all(format!("{}/__pycache__",settings.start_dir));
}
