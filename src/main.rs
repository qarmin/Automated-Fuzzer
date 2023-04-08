#![allow(clippy::upper_case_acronyms)]

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::apps::dlint::DlintStruct;
use crate::apps::image::ImageStruct;
use crate::apps::lofty::LoftyStruct;
use crate::apps::mypy::MypyStruct;
use crate::apps::oxc::OxcStruct;
use crate::apps::rome::RomeStruct;
use crate::apps::ruff::RuffStruct;
use crate::apps::selene::SeleneStruct;
use crate::apps::symphonia::SymphoniaStruct;
use crate::common::{
    execute_command_and_connect_output, minimize_binary_output, minimize_string_output,
};
use crate::obj::ProgramConfig;
use crate::settings::{load_settings, MODES};

pub mod apps;
mod broken_files;
mod common;
mod obj;
mod settings;

fn main() {
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(1)
    //     .build_global()
    //     .unwrap();

    let settings = load_settings();
    let obj: Box<dyn ProgramConfig> = match settings.current_mode {
        MODES::OXC => Box::new(OxcStruct {
            settings: settings.clone(),
        }),
        MODES::MYPY => Box::new(MypyStruct {
            settings: settings.clone(),
        }),
        MODES::DLINT => Box::new(DlintStruct {
            settings: settings.clone(),
        }),
        MODES::ROME => Box::new(RomeStruct {
            settings: settings.clone(),
        }),
        MODES::RUFF => Box::new(RuffStruct {
            settings: settings.clone(),
        }),
        MODES::LOFTY => Box::new(LoftyStruct {
            settings: settings.clone(),
        }),
        MODES::IMAGE => Box::new(ImageStruct {
            settings: settings.clone(),
        }),
        MODES::SYMPHONIA => Box::new(SymphoniaStruct {
            settings: settings.clone(),
        }),
        MODES::SELENE => Box::new(SeleneStruct {
            settings: settings.clone(),
        }),
    };

    for i in 1..=settings.loop_number {
        println!("Starting loop {i} out of all {}", settings.loop_number);

        if !settings.safe_run && settings.generate_files {
            let _ = fs::remove_dir_all(&settings.input_dir);
            fs::create_dir_all(&settings.input_dir).unwrap();

            let command = obj.broken_file_creator();
            let output = command.wait_with_output().unwrap();
            let out = String::from_utf8(output.stdout).unwrap();
            if settings.debug_print_broken_files_creator {
                println!("{out}");
            };
            println!("Generated files to test.");
        }

        let mut files = Vec::new();
        assert!(Path::new(&settings.input_dir).is_dir());
        for i in WalkDir::new(&settings.input_dir).into_iter().flatten() {
            let Some(s) = i.path().to_str() else { continue; };
            if settings.extensions.iter().any(|e| s.ends_with(e)) {
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
                println!("_____ {number} / {all}")
            }

            let s = execute_command_and_connect_output(&obj, &full_name);
            if settings.debug_print_results {
                println!("{s}");
            }
            if obj.is_broken(&s) {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                if let Some(new_file_name) = obj.validate_output_and_save_file(full_name, s) {
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
}
