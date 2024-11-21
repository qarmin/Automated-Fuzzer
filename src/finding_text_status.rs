use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use humansize::format_size;
use log::info;
use rand::random;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator};
use rayon::prelude::*;

use crate::common::{
    check_if_app_ends, close_app_if_timeouts, collect_files, execute_command_and_connect_output,
    execute_command_on_pack_of_files, generate_files, minimize_new, remove_and_create_entire_folder,
    CheckGroupFileMode,
};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub fn find_broken_files_by_text_status(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    info!("Starting finding broken files by text status");
    let atomic_all_broken = AtomicU32::new(0);

    let loop_number = settings.loop_number;

    for i in 1..=loop_number {
        info!("Starting loop {i} out of all {loop_number}");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        };

        info!("Removing old files");
        remove_and_create_entire_folder(&settings.temp_possible_broken_files_dir);

        info!("So - generating files from valid input files dir");
        generate_files(obj, settings);
        info!("generated files");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        };

        info!("Removing non parsable files");
        obj.remove_non_parsable_files(&settings.temp_possible_broken_files_dir);
        info!("Removed non parsable files");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        };
        info!("Collecting files");
        let (mut files, files_size) = collect_files(settings);
        let start_file_size = files.len();
        info!(
            "Collected {start_file_size} files with size {}",
            format_size(files_size, humansize::BINARY)
        );

        if settings.grouping > 1 && obj.get_files_group_mode() != CheckGroupFileMode::None {
            info!("Started to check files in groups of {} elements", settings.grouping);
            files = test_files_in_group(files, settings, obj);
            info!(
                "After grouping left {} files to check out of all {start_file_size}",
                files.len()
            );
        } else {
            info!("No grouping");
        }
        let atomic_broken = AtomicU32::new(0);

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        };
        test_files(files, settings, obj, &atomic_broken, &atomic_all_broken);

        info!("");
        info!(
            "Found {} broken files ({} in all iterations)",
            atomic_broken.load(Ordering::Relaxed),
            atomic_all_broken.load(Ordering::Relaxed)
        );
        info!("");
    }
    info!("");
    info!(
        "Found {} broken files in all iterations",
        atomic_all_broken.load(Ordering::Relaxed)
    );
    info!("");
}

fn test_files_in_group(files: Vec<String>, settings: &Setting, obj: &Box<dyn ProgramConfig>) -> Vec<String> {
    if obj.get_files_group_mode() == CheckGroupFileMode::None {
        return files;
    }

    let all_chunks_number = files.len() / settings.grouping as usize + 1;
    let atomic = AtomicU32::new(0);
    info!(
        "Started to check files in groups of {} elements, {} groups",
        settings.grouping, all_chunks_number
    );
    let res = files
        .into_par_iter()
        .chunks(settings.grouping as usize)
        .map(|group| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number % 10 == 0 {
                info!("+++++ {number} / {all_chunks_number}");
            }

            if check_if_app_ends() {
                return None;
            }

            let temp_folder = &settings.temp_folder;
            let mut map = HashMap::new();
            let random_folder = format!("{temp_folder}/{}", random::<u64>());
            fs::create_dir_all(&random_folder).expect("Failed to create random folder");

            let mut all_temp_files = vec![];
            for (idx, file_name) in group.iter().enumerate() {
                let extension = Path::new(file_name).extension().unwrap().to_str().unwrap();
                let temp_name = format!("{random_folder}/{idx}.{extension}");
                fs::copy(file_name, &temp_name).expect("Failed to copy file");
                map.insert(file_name, temp_name);
            }

            if obj.get_files_group_mode() == CheckGroupFileMode::ByFilesGroup {
                all_temp_files = map.values().map(ToString::to_string).collect();
            }
            // let count = WalkDir::new(&random_folder).max_depth(999).into_iter().flatten().count();
            // warn!("{:?} {:?} {}", all_temp_files.len(), random_folder, count);

            let output_result = execute_command_on_pack_of_files(obj, &random_folder, &all_temp_files);

            fs::remove_dir_all(&random_folder).expect("Failed to remove random folder");

            if settings.debug_print_results {
                info!("{}", output_result.get_output());
            }
            // info!("Group {}, elements {} - result {}", number , group.len(), is_really_broken || obj.is_broken(&output));

            if output_result.is_broken() {
                info!("Group {} is broken", number);
                output_result.debug_print();
                Some(Some(group))
            } else {
                Some(None)
            }
        })
        .while_some()
        .flatten()
        .flatten()
        .collect();

    close_app_if_timeouts();

    remove_and_create_entire_folder(&settings.temp_folder);
    res
}

fn test_files(
    files: Vec<String>,
    settings: &Setting,
    obj: &Box<dyn ProgramConfig>,
    atomic_broken: &AtomicU32,
    atomic_all_broken: &AtomicU32,
) {
    let atomic = AtomicU32::new(0);
    let all = files.len();

    files
        .into_par_iter()
        .map(|full_name| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number % 1000 == 0 {
                info!("_____ {number} / {all}");
            }
            if check_if_app_ends() {
                return None;
            }
            let output_result = execute_command_and_connect_output(obj, &full_name);
            if settings.debug_print_results {
                info!("{}", output_result.get_output());
            }
            if output_result.is_broken() {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                atomic_all_broken.fetch_add(1, Ordering::Relaxed);
                if let Some(new_file_name) = obj.validate_output_and_save_file(full_name, output_result.get_output()) {
                    if settings.minimize_output {
                        minimize_new(obj, &new_file_name);
                    }
                };
            };
            Some(())
        })
        .while_some()
        .collect::<()>();
}
