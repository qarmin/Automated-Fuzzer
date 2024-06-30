use crate::common::{execute_command_and_connect_output, remove_and_create_entire_folder};
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use jwalk::WalkDir;
use log::info;
use rayon::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    obj.remove_non_parsable_files(&settings.broken_files_dir);

    let broken_files: Vec<String> = collect_broken_files(settings);
    let before = broken_files.len();
    let after = AtomicUsize::new(before);
    info!("Found {before} files to check");

    let atomic_counter = AtomicUsize::new(0);
    let all = broken_files.len();
    broken_files.into_par_iter().for_each(|full_name| {
        let start_text = fs::read(&full_name).unwrap();
        let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
        if idx % 100 == 0 {
            info!("_____ Processsed already {idx} / {all}");
        }
        let (is_really_broken, output) = execute_command_and_connect_output(obj, &full_name);
        // if settings.debug_print_results {
        //     info!("File {full_name}\n{output}");
        // }
        if is_really_broken || obj.is_broken(&output) {
            fs::write(&full_name, &start_text).unwrap();
            return;
        };
        info!("File {full_name} is not broken, and will be removed");

        fs::remove_file(&full_name).unwrap();
        after.fetch_sub(1, Ordering::Relaxed);
    });

    // TODO - why is this here?
    if collect_broken_files(settings).is_empty() {
        remove_and_create_entire_folder(&settings.broken_files_dir);
    }

    let after = after.load(Ordering::Relaxed);
    info!("Removed {} files, left {after} files", before - after);
}

fn collect_broken_files(settings: &Setting) -> Vec<String> {
    WalkDir::new(&settings.broken_files_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().is_file() {
                return None;
            }

            let path = entry.path().to_string_lossy().to_string();
            let path_to_lowercase = path.to_lowercase();

            if settings.extensions.iter().any(|e| path_to_lowercase.ends_with(e)) {
                return Some(path);
            }

            None
        })
        .collect()
}
