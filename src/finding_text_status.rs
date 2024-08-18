use crate::common::{
    check_if_app_ends, collect_files, execute_command_and_connect_output, generate_files, minimize_new,
    remove_and_create_entire_folder,
};
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use humansize::format_size;
use log::info;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

pub fn find_broken_files_by_text_status(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
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
        let (files, files_size) = collect_files(settings);
        let start_file_size = files.len();
        info!(
            "Collected {start_file_size} files with size {}",
            format_size(files_size, humansize::BINARY)
        );

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
