use crate::common::{
    execute_command_and_connect_output, execute_command_on_pack_of_files, remove_and_create_entire_folder,
};
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use jwalk::WalkDir;
use log::{error, info};
use rand::random;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    obj.remove_non_parsable_files(&settings.broken_files_dir);

    let broken_files: Vec<String> = collect_broken_files(settings);
    info!("Found {} broken files to check", broken_files.len());
    // let broken_files_before = broken_files.len();

    remove_non_crashing(broken_files, settings, obj, 1);

    // let broken_files: Vec<String> = collect_broken_files(settings);
    // let broken_files_after = broken_files.len();
    //
    // remove_non_crashing(broken_files, settings, obj, 2);
    //
    // let broken_files: Vec<String> = collect_broken_files(settings);
    // let broken_files_after2 = broken_files.len();
    //
    // info!("At start there was {broken_files_before} files, after first pass {broken_files_after}, after second pass {broken_files_after2}");
    // if broken_files_after != broken_files_after2 {
    //     error!("There is unstable checking for broken files");
    // }
}

fn remove_non_crashing(broken_files: Vec<String>, settings: &Setting, obj: &Box<dyn ProgramConfig>, step: u32) {
    // Processing in groups
    let group_size = 20;
    let atomic_counter = AtomicUsize::new(0);
    let all_chunks = broken_files.chunks(group_size).count();
    let still_broken_files: Vec<_> = broken_files
        .into_par_iter()
        .chunks(group_size)
        .enumerate()
        .filter_map(|(chunk_idx, chunk)| {
            let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
            info!("_____ Processsed already {idx} / {all_chunks} chunk (step {group_size})");
            let temp_folder = format!("{}/{}", settings.temp_folder, random::<u64>());
            fs::create_dir_all(&temp_folder).unwrap();

            for (idx, full_name) in chunk.iter().enumerate() {
                let extension = Path::new(full_name).extension().unwrap().to_str().unwrap();
                let new_name = format!("{temp_folder}/{idx}.{extension}");
                fs::copy(&full_name, &new_name).unwrap();
            }

            let (is_really_broken, output) = execute_command_on_pack_of_files(obj, &temp_folder, &[]);
            if settings.debug_print_results {
                info!("File pack {temp_folder}\n{output}");
            }

            fs::remove_dir_all(&temp_folder).unwrap();

            if is_really_broken || obj.is_broken(&output) {
                info!("Chunk {chunk_idx} is broken");
                Some(chunk.to_vec())
            } else {
                info!("Chunk {chunk_idx} is not broken");
                for full_name in chunk {
                    fs::remove_file(&full_name).unwrap();
                }
                None
            }
        })
        .flatten()
        .collect();

    let atomic_counter = AtomicUsize::new(0);
    let all = still_broken_files.len();
    still_broken_files.into_par_iter().for_each(|full_name| {
        let start_text = fs::read(&full_name).unwrap();
        let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
        if idx % 100 == 0 {
            info!("_____ Processsed already {idx} / {all} (step {step})");
        }
        let (is_really_broken, output) = execute_command_and_connect_output(obj, &full_name);
        if settings.debug_print_results {
            info!("File {full_name}\n{output}");
        }
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
