use crate::common::{execute_command_and_connect_output, remove_and_create_entire_folder};
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rayon::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    obj.remove_non_parsable_files(&settings.broken_files_dir);

    let broken_files: Vec<String> = collect_broken_files(settings);
    let broken_files_before = broken_files.len();

    remove_non_crashing(broken_files, settings, obj, 1);

    let broken_files: Vec<String> = collect_broken_files(settings);
    let broken_files_after = broken_files.len();

    remove_non_crashing(broken_files, settings, obj, 2);

    let broken_files: Vec<String> = collect_broken_files(settings);
    let broken_files_after2 = broken_files.len();

    info!("At start there was {broken_files_before} files, after first pass {broken_files_after}, after second pass {broken_files_after2}");
    if broken_files_after != broken_files_after2 {
        error!("There is unstable checking for broken files");
    }
}

fn remove_non_crashing(broken_files: Vec<String>, settings: &Setting, obj: &Box<dyn ProgramConfig>, step: u32) {
    let atomic_counter = AtomicUsize::new(0);
    let all = broken_files.len();
    broken_files.into_par_iter().for_each(|full_name| {
        let start_text = fs::read(&full_name).unwrap();
        let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
        if idx % 100 == 0 {
            info!("_____ Processsed already {idx} / {all} (step {step})");
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
    });

    // TODO - why is this here?
    if collect_broken_files(settings).is_empty() {
        remove_and_create_entire_folder(&settings.broken_files_dir);
    }
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
