#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::borrowed_box)]

use rand::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, process};

use crate::common::{
    check_if_app_ends, close_app_if_timeouts, execute_command_and_connect_output, execute_command_on_pack_of_files,
    minimize_binary_output, minimize_string_output, remove_and_create_entire_folder, CheckGroupFileMode, TIMEOUT_SECS,
};
use crate::obj::ProgramConfig;
use crate::settings::{get_object, load_settings, Setting};
use jwalk::WalkDir;
use log::{error, info};

pub mod apps;
mod broken_files;
mod clean_base_files;
mod common;
mod minimal_rules;
mod obj;
mod remove_non_crashing_files;
mod settings;
mod verify_reported_broken_files;

fn main() {
    handsome_logger::init().unwrap();

    let first_arg: u64 = std::env::args()
        .nth(1)
        .map_or(999_999_999_999_999, |x| x.parse().unwrap());
    info!("Timeout set to {first_arg} seconds");
    TIMEOUT_SECS.set(first_arg).unwrap();

    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(8)
    //     .build_global()
    //     .unwrap();

    let settings = load_settings();

    info!("{settings:#?}");

    let mut obj = get_object(settings.clone());

    obj.init();

    let _ = fs::create_dir_all(&settings.temp_folder);
    let _ = fs::create_dir_all(&settings.broken_files_dir);

    if settings.remove_non_crashing_items_from_broken_files {
        remove_non_crashing_files::remove_non_crashing_files(&settings, &obj);
        return;
    }
    if settings.clean_base_files {
        clean_base_files::clean_base_files(&settings, &obj);
        return;
    }
    if settings.find_minimal_rules {
        minimal_rules::check_code(&settings, &obj);
        return;
    }
    if settings.verify_if_files_are_still_broken {
        verify_reported_broken_files::verify_if_files_are_still_broken(&settings, &obj);
        return;
    }

    check_files_number("Valid input dir", &settings.valid_input_files_dir);
    check_files_number("Broken files dir", &settings.broken_files_dir);
    check_files_number(
        "Temp possible broken files dir", &settings.temp_possible_broken_files_dir,
    );

    assert!(Path::new(&settings.valid_input_files_dir).exists());
    assert!(Path::new(&settings.broken_files_dir).exists());

    let atomic_all_broken = AtomicU32::new(0);

    // There is no big sense in running more than 1 times where initial files are not changed
    let loop_number = if settings.generate_files {
        settings.loop_number
    } else {
        1
    };

    for i in 1..=loop_number {
        info!("Starting loop {i} out of all {loop_number}");

        if check_if_app_ends() {
            info!("Timeout reached, exiting");
            break;
        };

        if !settings.ignore_generate_copy_files_step {
            info!("Removing old files");
            remove_and_create_entire_folder(&settings.temp_possible_broken_files_dir);
            if settings.generate_files {
                info!("So - generating files from valid input files dir");
                generate_files(&obj, &settings);
                info!("generated files");
            } else {
                info!("So - copying files");
                // instead creating files, copy them
                // let valid_input_files_dir = &obj.get_settings().valid_input_files_dir;
                // let temp_possible_broken_files_dir = &obj.get_settings().temp_possible_broken_files_dir;
                copy_files(&settings);
            }
        } else {
            info!("So - no copying or generating files");
        }
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
        let mut files = collect_files(&settings);
        let start_file_size = files.len();
        info!("Collected {start_file_size} files");

        if settings.grouping > 1 && obj.get_files_group_mode() != CheckGroupFileMode::None {
            info!("Started to check files in groups of {} elements", settings.grouping);
            files = test_files_in_group(files, &settings, &obj);
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
        test_files(files, &settings, &obj, &atomic_broken, &atomic_all_broken);

        info!("");
        info!("Found {} broken files", atomic_broken.load(Ordering::Relaxed));
        info!("");
    }
    info!("");
    info!(
        "Found {} broken files in all iterations",
        atomic_all_broken.load(Ordering::Relaxed)
    );
    info!("");
}

fn generate_files(obj: &Box<dyn ProgramConfig>, settings: &Setting) {
    let command = obj.broken_file_creator();
    let output = command.wait_with_output().unwrap();
    let out = String::from_utf8(output.stdout).unwrap();
    if !output.status.success() {
        error!("{:?}", output.status);
        error!("{out}");
        error!("Failed to generate files");
        process::exit(1);
    }
    if settings.debug_print_broken_files_creator {
        info!("{out}");
    };
}

fn copy_files(settings: &Setting) {
    let mut collected_files = Vec::new();
    for i in WalkDir::new(&settings.valid_input_files_dir)
        .max_depth(999)
        .into_iter()
        .flatten()
    {
        let path = i.path();
        if !path.is_file() {
            continue;
        }
        let Some(s) = path.to_str() else {
            continue;
        };
        let Some(old_name) = path.file_stem() else {
            continue;
        };
        let Some(old_name) = old_name.to_str() else {
            continue;
        };
        let Some(extension) = path.extension() else {
            continue;
        };
        let Some(extension) = extension.to_str() else {
            continue;
        };
        if settings.extensions.iter().any(|e| s.to_lowercase().ends_with(e)) {
            collected_files.push((s.to_string(), old_name.to_string(), extension.to_string()));
        }
    }
    info!(
        "Completed collecting files to check({} found files)",
        collected_files.len()
    );
    collected_files.into_par_iter().for_each(|(s, old_name, extension)| {
        let mut rng = thread_rng();

        let mut new_name = format!("{}/{}.{}", settings.temp_possible_broken_files_dir, old_name, extension);
        while Path::new(&new_name).exists() {
            let random_number: u64 = rng.gen();
            new_name = format!(
                "{}/{}-{}.{}",
                settings.temp_possible_broken_files_dir, old_name, random_number, extension
            );
        }
        // info!("Copying file {s}  to {new_name:?}");
        if let Err(e) = fs::copy(&s, &new_name) {
            error!("Failed to copy file {s} to {new_name} with error {e}");
        };
    });
}

fn collect_files(settings: &Setting) -> Vec<String> {
    let mut files = Vec::new();
    assert!(Path::new(&settings.temp_possible_broken_files_dir).is_dir());
    for i in WalkDir::new(&settings.temp_possible_broken_files_dir)
        .max_depth(999)
        .into_iter()
        .flatten()
    {
        let path = i.path();
        if !path.is_file() {
            continue;
        }
        let Ok(metadata) = i.metadata() else {
            continue;
        };
        metadata.permissions().set_mode(0o777);
        let Some(s) = path.to_str() else {
            continue;
        };
        if settings.extensions.iter().any(|e| s.to_lowercase().ends_with(e)) {
            files.push(s.to_string());
        }
    }
    if files.len() > settings.max_collected_files {
        files.truncate(settings.max_collected_files);
    }

    if files.is_empty() {
        dbg!(&settings);
        assert!(!files.is_empty());
    }

    files
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

            let (is_really_broken, output) = execute_command_on_pack_of_files(obj, &random_folder, &all_temp_files);

            fs::remove_dir_all(&random_folder).expect("Failed to remove random folder");

            if settings.debug_print_results {
                info!("{output}");
            }
            // info!("Group {}, elements {} - result {}", number , group.len(), is_really_broken || obj.is_broken(&output));

            if is_really_broken || obj.is_broken(&output) {
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
                if check_if_app_ends() {
                    return None;
                }
            }
            let (is_really_broken, output) = execute_command_and_connect_output(obj, &full_name);
            if settings.debug_print_results {
                info!("{output}");
            }
            if is_really_broken || obj.is_broken(&output) {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                atomic_all_broken.fetch_add(1, Ordering::Relaxed);
                if let Some(new_file_name) = obj.validate_output_and_save_file(full_name, output) {
                    if settings.minimize_output {
                        if settings.binary_mode {
                            minimize_binary_output(obj, &new_file_name);
                        } else {
                            minimize_string_output(obj, &new_file_name);
                        }
                    }
                };
            };
            Some(())
        })
        .while_some()
        .collect::<()>();
}

fn check_files_number(name: &str, dir: &str) {
    info!(
        "{name} - {} - Files Number {}.",
        dir,
        WalkDir::new(dir)
            .max_depth(999)
            .into_iter()
            .flatten()
            .filter(|e| e.path().is_file())
            .count()
    );
}
