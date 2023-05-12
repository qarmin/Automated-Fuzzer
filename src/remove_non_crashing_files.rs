use crate::common::execute_command_and_connect_output;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use rayon::prelude::*;
use std::fs;
use walkdir::WalkDir;

pub fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let broken_files: Vec<String> = WalkDir::new(&settings.output_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            println!("{entry:?}");
            if entry.file_type().is_file() {
                return Some(entry.path().to_string_lossy().to_string());
            }
            None
        })
        .collect();

    println!("Found {} files to check", broken_files.len());
    broken_files.into_par_iter().for_each(|full_name| {
        let (is_really_broken, output) = execute_command_and_connect_output(obj, &full_name);
        if is_really_broken || obj.is_broken(&output) {
            return;
        };

        println!("File {full_name} is not broken, and will be removed");
        fs::remove_file(&full_name).unwrap();
    });
}
