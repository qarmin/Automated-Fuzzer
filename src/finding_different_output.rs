use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use humansize::format_size;
use log::info;
use rayon::prelude::*;

use crate::common::{
    check_if_app_ends, collect_files, execute_command_and_connect_output, generate_files,
    remove_and_create_entire_folder,
};
use crate::obj::ProgramConfig;
use crate::settings::{Setting, StabilityMode};

pub(crate) fn find_broken_files_by_different_output(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    info!("Starting finding broken files by different output");
    let atomic_all_broken = AtomicU32::new(0);

    let loop_number = settings.loop_number;

    for i in 1..=loop_number {
        info!("Starting loop {i} out of all {loop_number}");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        }

        info!("Removing old files");
        remove_and_create_entire_folder(&settings.temp_possible_broken_files_dir);

        info!("So - generating files from valid input files dir");
        generate_files(obj, settings);
        info!("generated files");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        }

        info!("Removing non parsable files");
        obj.remove_non_parsable_files(&settings.temp_possible_broken_files_dir);
        info!("Removed non parsable files");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        }
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
        }
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
    assert_ne!(obj.get_stability_mode(), StabilityMode::None);
    let atomic = AtomicU32::new(0);
    let all = files.len();

    files
        .into_par_iter()
        .map(|full_name| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number.is_multiple_of(100) {
                info!("_____ {number} / {all}");
            }
            if check_if_app_ends() {
                return None;
            }
            let file_content = fs::read(&full_name).unwrap();
            let mut outputs = Vec::new();
            let mut file_after_content = Vec::new();
            for _ in 0..settings.stability_runs {
                let output_result = execute_command_and_connect_output(obj, &full_name);
                if settings.debug_print_results {
                    info!("{}", output_result.get_output());
                }

                if [StabilityMode::OutputContent, StabilityMode::FileContent].contains(&obj.get_stability_mode()) {
                    file_after_content.push(fs::read(&full_name).unwrap());
                }
                if [StabilityMode::OutputContent, StabilityMode::ConsoleOutput].contains(&obj.get_stability_mode()) {
                    outputs.push(output_result.get_output().to_string());
                }
                fs::write(&full_name, &file_content).unwrap();
            }

            let is_output_different = !outputs.is_empty() && outputs.windows(2).any(|w| w[0] != w[1]);
            let is_file_different =
                !file_after_content.is_empty() && file_after_content.windows(2).any(|w| w[0] != w[1]);
            if is_file_different || is_output_different {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                atomic_all_broken.fetch_add(1, Ordering::Relaxed);
                // TODO - maybe later add minimization, but I doubt that this will easy and reproducible
                obj.validate_txt_and_save_file(full_name, &outputs);
                return Some(());
            }

            Some(())
        })
        .while_some()
        .collect::<()>();
}
