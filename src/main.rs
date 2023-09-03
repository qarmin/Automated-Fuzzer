#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::borrowed_box)]

use rand::prelude::*;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, process};

use rayon::prelude::*;

use crate::common::{
    execute_command_and_connect_output, minimize_binary_output, minimize_string_output,
};
use crate::settings::{get_object, load_settings};
use jwalk::WalkDir;

pub mod apps;
mod broken_files;
mod clean_base_files;
mod common;
mod minimal_rules;
mod obj;
mod remove_non_crashing_files;
mod settings;

fn main() {
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(8)
    //     .build_global()
    //     .unwrap();

    let settings = load_settings();
    let mut obj = get_object(settings.clone());

    obj.init();

    let _ = fs::create_dir_all(&settings.temp_folder);

    if settings.remove_non_crashing_items_from_broken_files {
        remove_non_crashing_files::remove_non_crashing_files(&settings, &obj);
        return;
    }
    if settings.clean_base_files {
        clean_base_files::clean_base_files(&settings);
        return;
    }
    if settings.find_minimal_rules {
        minimal_rules::find_minimal_rules(&settings, &obj);
        return;
    }

    println!(
        "Base of valid {} Files Number {}.",
        settings.base_of_valid_files,
        WalkDir::new(&settings.base_of_valid_files)
            .max_depth(999)
            .into_iter()
            .flatten()
            .count()
    );
    println!(
        "Input Dir {} Files Number {}.",
        settings.input_dir,
        WalkDir::new(&settings.input_dir)
            .max_depth(999)
            .into_iter()
            .flatten()
            .count()
    );
    println!(
        "Output Dir {} Files Number {}.",
        settings.output_dir,
        WalkDir::new(&settings.output_dir)
            .max_depth(999)
            .into_iter()
            .flatten()
            .count()
    );

    assert!(Path::new(&settings.base_of_valid_files).exists());
    assert!(Path::new(&settings.output_dir).exists());

    let atomic_all_broken = AtomicU32::new(0);

    // There is no big sense in running more than 1 times where initial files are not changed
    let loop_number = if settings.generate_files {
        settings.loop_number
    } else {
        1
    };
    for i in 1..=loop_number {
        println!("Starting loop {i} out of all {loop_number}");

        if !settings.ignore_generate_copy_files_step {
            println!("Removing old files");
            let _ = fs::remove_dir_all(&settings.input_dir);
            fs::create_dir_all(&settings.input_dir).unwrap();
            if settings.generate_files {
                let command = obj.broken_file_creator();
                let output = command.wait_with_output().unwrap();
                let out = String::from_utf8(output.stdout).unwrap();
                if !output.status.success() {
                    println!("{:?}", output.status);
                    println!("{out}");
                    println!("Failed to generate files");
                    process::exit(1);
                }
                if settings.debug_print_broken_files_creator {
                    println!("{out}");
                };
            } else {
                // instead creating files, copy them
                // let base_of_valid_files = &obj.get_settings().base_of_valid_files;
                // let input_dir = &obj.get_settings().input_dir;
                let mut collected_files = Vec::new();
                for i in WalkDir::new(&settings.base_of_valid_files)
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
                    if settings
                        .extensions
                        .iter()
                        .any(|e| s.to_lowercase().ends_with(e))
                    {
                        collected_files.push((
                            s.to_string(),
                            old_name.to_string(),
                            extension.to_string(),
                        ));
                    }
                }
                println!(
                    "Completed collecting files to check({} found files)",
                    collected_files.len()
                );
                collected_files
                    .into_par_iter()
                    .for_each(|(s, old_name, extension)| {
                        let mut rng = thread_rng();

                        let mut new_name =
                            format!("{}/{}.{}", settings.input_dir, old_name, extension);
                        while Path::new(&new_name).exists() {
                            let random_number: u64 = rng.gen();
                            new_name = format!(
                                "{}/{}-{}.{}",
                                settings.input_dir, old_name, random_number, extension
                            );
                        }
                        // println!("Copying file {s}  to {new_name:?}");
                        if let Err(e) = fs::copy(&s, &new_name) {
                            println!("Failed to copy file {s} to {new_name} with error {e}");
                        };
                    });
            }
        }

        let mut files = Vec::new();
        assert!(Path::new(&settings.input_dir).is_dir());
        for i in WalkDir::new(&settings.input_dir)
            .max_depth(999)
            .into_iter()
            .flatten()
        {
            let path = i.path();
            if !path.is_file() {
                continue;
            }
            let Ok(metadata) = i.metadata() else {
                return;
            };
            metadata.permissions().set_mode(0o777);
            let Some(s) = path.to_str() else {
                continue;
            };
            if settings
                .extensions
                .iter()
                .any(|e| s.to_lowercase().ends_with(e))
            {
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

        let atomic = AtomicU32::new(0);
        let atomic_broken = AtomicU32::new(0);
        let all = files.len();

        files.into_par_iter().for_each(|full_name| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number % 1000 == 0 {
                println!("_____ {number} / {all}");
            }
            let (is_really_broken, output) = execute_command_and_connect_output(&obj, &full_name);
            if settings.debug_print_results {
                println!("{output}");
            }
            if is_really_broken || obj.is_broken(&output) {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                atomic_all_broken.fetch_add(1, Ordering::Relaxed);
                if let Some(new_file_name) = obj.validate_output_and_save_file(full_name, output) {
                    if settings.minimize_output {
                        if settings.binary_mode {
                            minimize_binary_output(&obj, &new_file_name);
                        } else {
                            minimize_string_output(&obj, &new_file_name);
                        }
                    }
                };
            }
        });

        println!(
            "\n\nFound {} broken files",
            atomic_broken.load(Ordering::Relaxed)
        );
    }
    println!(
        "\n\nFound {} broken files in all iterations",
        atomic_all_broken.load(Ordering::Relaxed)
    );
}
