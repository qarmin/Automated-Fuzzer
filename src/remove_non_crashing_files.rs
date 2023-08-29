use crate::common::execute_command_and_connect_output;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let broken_files: Vec<String> = collect_broken_files(settings);
    let before = broken_files.len();
    let after = AtomicUsize::new(before);
    println!("Found {before} files to check");
    broken_files.into_par_iter().for_each(|full_name| {
        let (is_really_broken, output) = execute_command_and_connect_output(obj, &full_name);
        if is_really_broken || obj.is_broken(&output) {
            return;
        };

        println!("File {full_name} is not broken, and will be removed");
        fs::remove_file(&full_name).unwrap();
        after.fetch_sub(1, Ordering::Relaxed);
    });

    let after = after.load(Ordering::Relaxed);
    println!("Removed {} files, left {after} files", before - after);
}

fn collect_broken_files(settings: &Setting) -> Vec<String> {
    WalkDir::new(&settings.output_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if entry.file_type().is_file() {
                return Some(entry.path().to_string_lossy().to_string());
            }
            None
        })
        .collect()
}
