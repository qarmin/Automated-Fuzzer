use std::path::Path;

use jwalk::WalkDir;
use log::info;

use crate::common::minimize_new;
use crate::settings::{get_object, load_settings};

pub fn run_minimize(_config: &str, dir_override: Option<&str>, _command_override: Option<&str>) {
    info!("Starting minimization");

    let settings = load_settings();
    let obj = get_object(settings.clone());

    let broken_dir = dir_override.unwrap_or(&settings.broken_files_dir);

    if !Path::new(broken_dir).exists() {
        eprintln!("Broken files directory '{broken_dir}' does not exist.");
        return;
    }

    let files: Vec<String> = WalkDir::new(broken_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().is_file() {
                return None;
            }
            let path = entry.path().to_string_lossy().to_string();
            let lower = path.to_lowercase();

            // Skip already minimized files
            if lower.contains("_minimized_") {
                return None;
            }

            if settings.extensions.iter().any(|e| lower.ends_with(e)) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if files.is_empty() {
        println!("No files to minimize in '{broken_dir}'.");
        return;
    }

    info!("Found {} files to minimize", files.len());

    for (idx, file) in files.iter().enumerate() {
        info!("Minimizing [{}/{}]: {}", idx + 1, files.len(), file);
        minimize_new(&obj, file);

        if crate::SHOULD_STOP.load(std::sync::atomic::Ordering::Relaxed) {
            info!("Stop requested, finishing minimization.");
            break;
        }
    }

    info!("Minimization complete.");
}
