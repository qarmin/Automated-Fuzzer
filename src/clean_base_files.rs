use crate::obj::ProgramConfig;
use crate::settings::Setting;
use log::info;
use rayon::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn clean_base_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    if settings.extensions.contains(&".py".to_string()) {
        remove_non_parsing_python_files(settings, obj);
    }
}

fn remove_non_parsing_python_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let broken_files = obj.collect_files_in_dir_with_extension(&settings.valid_input_files_dir);
    let before = broken_files.len();
    let after = AtomicUsize::new(before);
    info!("Found {before} python files to check");
    broken_files.into_par_iter().for_each(|full_name| {
        if !obj.is_parsable(&full_name) {
            return;
        }
        info!("File {full_name} is not valid python file, and will be removed");
        fs::remove_file(&full_name).unwrap();
        after.fetch_sub(1, Ordering::Relaxed);
    });

    let after = after.load(Ordering::Relaxed);
    info!("Removed {} python files, left {after} files", before - after);
}
