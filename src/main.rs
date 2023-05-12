#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::borrowed_box)]

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, process};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::common::{
    execute_command_and_connect_output, minimize_binary_output, minimize_string_output,
};
use crate::settings::{get_object, load_settings};

pub mod apps;
mod broken_files;
mod common;
mod obj;
mod remove_non_crashing_files;
mod settings;

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build_global()
        .unwrap();

    let settings = load_settings();
    let obj = get_object(settings.clone());

    if settings.remove_non_crashing_items_from_broken_files {
        remove_non_crashing_files::remove_non_crashing_files(&settings, &obj);
        return;
    }

    assert!(Path::new(&settings.base_of_valid_files).exists());
    assert!(Path::new(&settings.output_dir).exists());

    let atomic_all_broken = AtomicU32::new(0);
    for i in 1..=settings.loop_number {
        println!("Starting loop {i} out of all {}", settings.loop_number);

        if !settings.safe_run && settings.generate_files {
            let _ = fs::remove_dir_all(&settings.input_dir);
            fs::create_dir_all(&settings.input_dir).unwrap();

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
            println!("Generated files to test.");
        }

        let mut files = Vec::new();
        assert!(Path::new(&settings.input_dir).is_dir());
        for i in WalkDir::new(&settings.input_dir).into_iter().flatten() {
            let Some(s) = i.path().to_str() else { continue; };
            if settings
                .extensions
                .iter()
                .any(|e| s.to_lowercase().ends_with(e))
            {
                files.push(s.to_string());
            }
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
