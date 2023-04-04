#![allow(clippy::upper_case_acronyms)]

use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use config::Config;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::common::{execute_command_and_connect_output, minimize_output};
use crate::dlint::DlintStruct;
use crate::mypy::MypyStruct;
use crate::obj::ProgramConfig;
use crate::oxc::OxcStruct;
use crate::rome::RomeStruct;
use crate::ruff::RuffStruct;
use crate::settings::MODES;

mod common;
mod dlint;
mod mypy;
mod oxc;
mod rome;
mod ruff;
mod settings;
mod obj;

#[derive(Clone)]
pub struct Setting {
    loop_number: u32,
    broken_files_for_each_file: u32,
    copy_broken_files: bool,
    generate_files: bool,
    minimize_output: bool,
    minimization_attempts: u32,
    current_mode: MODES,
    extensions: Vec<String>,
    output_dir: String,
    base_of_valid_files: String,
    input_dir: String,
    app_binary: String,
    app_config: String
}

fn load_settings() -> Setting {
    let settings = Config::builder()
        .add_source(config::File::with_name("fuzz_settings"))
        .build()
        .unwrap();
    let config = settings
        .try_deserialize::<HashMap<String, HashMap<String, String>>>()
        .unwrap();

    let general = config["general"].clone();
    let current_mode_string = general["current_mode"].clone();
    let current_mode = MODES::from_str(&current_mode_string).unwrap();
    let curr_setting = config[&current_mode_string].clone();

    let copy_broken_files = general["copy_broken_files"].parse().unwrap();
    let broken_files_dir: String = general["broken_files_dir"].parse().unwrap();
    let non_destructive_input_dir: String = curr_setting["non_destructive_input_dir"].parse().unwrap();
    let input_dir = if copy_broken_files {
        broken_files_dir.clone()
    } else {
        non_destructive_input_dir.clone()
    };
    let set = Setting {
        loop_number: general["loop_number"].parse().unwrap(),
        broken_files_for_each_file: general["broken_files_for_each_file"].parse().unwrap(),
        copy_broken_files,
        generate_files: general["generate_files"].parse().unwrap(),
        minimize_output: general["minimize_output"].parse().unwrap(),
        minimization_attempts: general["minimization_attempts"].parse().unwrap(),
        current_mode,
        extensions: curr_setting["extensions"].split('\n').map(str::trim).filter_map(|e| if e.is_empty() { None } else {
            Some(format!(".{e}"))
        }).collect(),
        output_dir: curr_setting["output_dir"].parse().unwrap(),
        base_of_valid_files: curr_setting["base_of_valid_files"].parse().unwrap(),
        input_dir,
        app_binary: curr_setting["app_binary"].parse().unwrap(),
        app_config: curr_setting["app_config"].parse().unwrap(),
    };
    panic!("POTATO");
    return set;
}

fn main() {
    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(1)
    //     .build_global()
    //     .unwrap();

    let settings = load_settings();
    let obj: Box<dyn ProgramConfig> = match settings.current_mode {
        MODES::OXC => Box::new(OxcStruct{settings: settings.clone()}),
        MODES::MYPY => Box::new(MypyStruct{settings: settings.clone()}),
        MODES::DLINT => Box::new(DlintStruct{settings: settings.clone()}),
        MODES::ROME => Box::new(RomeStruct{settings: settings.clone()}),
        MODES::RUFF => Box::new(RuffStruct{settings: settings.clone()})
    };

    for i in 1..=settings.loop_number {
        println!("Starting loop {i} out of all {}", settings.loop_number);

        if settings.generate_files {
            let _ = fs::remove_dir_all(&settings.input_dir);
            fs::create_dir_all(&settings.input_dir).unwrap();

            let command = obj.broken_file_creator();
            let _output = command.wait_with_output().unwrap();
            // println!("{}", String::from_utf8(output.stdout).unwrap());
            println!("Generated files to test.");
        }

        let mut files = Vec::new();
        for i in WalkDir::new(&settings.input_dir).into_iter().flatten() {
            let Some(s) = i.path().to_str() else { continue; };
            if settings.extensions.iter().any(|e| s.ends_with(e)) {
                files.push(s.to_string());
            }
        }
        assert!(!files.is_empty());

        let atomic = AtomicU32::new(0);
        let atomic_broken = AtomicU32::new(0);
        let all = files.len();

        files.into_par_iter().for_each(|full_name| {
            let number = atomic.fetch_add(1, Ordering::Release);
            if number % 1000 == 0 {
                println!("_____ {number} / {all}")
            }

            let s = execute_command_and_connect_output(&obj,&full_name);

            if obj.is_broken(&s) {
                atomic_broken.fetch_add(1, Ordering::Relaxed);
                if let Some(new_file_name) = obj.validate_output(full_name, s) {
                    if settings.minimize_output {
                        minimize_output(&obj, &new_file_name);
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
