use std::fs;
use std::path::Path;

use log::info;

use crate::common::{execute_command_and_connect_output, set_timeout};
use crate::error_signature::parse_error_signature;
use crate::fuzz_cargo::run_cargo_fuzz;
use crate::settings::{StabilityMode, get_object, load_settings};

/// Run fuzzer in CI mode with state persistence
pub fn run_ci(config: &str, timeout: u64, state_dir: &str, output_dir: &str, mode: &str, target: Option<&str>) {
    set_timeout(timeout);

    // Ensure directories exist
    fs::create_dir_all(state_dir).unwrap();
    fs::create_dir_all(output_dir).unwrap();

    let corpus_dir = format!("{state_dir}/corpus");
    let known_crashes_dir = format!("{state_dir}/known_crashes");
    let history_path = format!("{state_dir}/history.toml");
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::create_dir_all(&known_crashes_dir).unwrap();

    match mode {
        "custom" => {
            run_ci_custom(config, timeout, state_dir, output_dir);
        }
        "cargo-fuzz" => {
            let target = target.expect("--target is required for CI cargo-fuzz mode");
            run_cargo_fuzz(target, &corpus_dir, timeout, None, 4);
        }
        other => {
            eprintln!("Unknown CI mode: {other}");
        }
    }

    // Record run in history
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let entry = format!("[[runs]]\ndate = \"{timestamp}\"\nmode = \"{mode}\"\ntimeout = {timeout}\n\n");
    let mut history = fs::read_to_string(&history_path).unwrap_or_default();
    history.push_str(&entry);
    fs::write(&history_path, history).unwrap();

    info!("CI run completed. State saved to {state_dir}");
}

fn run_ci_custom(_config: &str, _timeout: u64, state_dir: &str, output_dir: &str) {
    let settings = load_settings();
    let mut obj = get_object(settings.clone());
    obj.init();

    let _ = fs::create_dir_all(&settings.temp_folder);
    let _ = fs::create_dir_all(&settings.broken_files_dir);
    let _ = fs::create_dir_all(&settings.custom_folder_path);

    // Run the fuzzer
    if settings.check_for_stability && obj.get_stability_mode() != StabilityMode::None {
        crate::finding_different_output::find_broken_files_by_different_output(&settings, &obj);
    } else {
        crate::finding_text_status::find_broken_files_by_text_status(&settings, &obj);
    }

    // After fuzzing, run remove_non_crashing to verify and generate reports
    info!("CI: Running remove_non_crashing to verify results");
    crate::remove_non_crashing_files::remove_non_crashing_files(&settings, &obj);

    // Copy results to output_dir
    if Path::new(&settings.temp_folder).exists() {
        copy_dir_contents(&settings.temp_folder, output_dir);
    }

    // Save known crashes to state
    let known_crashes_dir = format!("{state_dir}/known_crashes");
    if Path::new(&settings.broken_files_dir).exists() {
        copy_dir_contents(&settings.broken_files_dir, &known_crashes_dir);
    }
}

/// Verify if previously found crashes are still reproducible
pub fn verify_regressions(_config: &str, state_dir: &str) {
    set_timeout(999_999_999_999);

    let settings = load_settings();
    let obj = get_object(settings.clone());

    let known_crashes_dir = format!("{state_dir}/known_crashes");
    if !Path::new(&known_crashes_dir).exists() {
        println!("No known crashes directory at {known_crashes_dir}");
        return;
    }

    let files: Vec<String> = jwalk::WalkDir::new(&known_crashes_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().is_file() {
                return None;
            }
            let path = entry.path().to_string_lossy().to_string();
            let lower = path.to_lowercase();
            if settings.extensions.iter().any(|e| lower.ends_with(e)) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if files.is_empty() {
        println!("No known crash files to verify.");
        return;
    }

    info!("Verifying {} known crash files...", files.len());

    let mut still_broken = 0;
    let mut fixed = 0;

    for file in &files {
        let output_result = execute_command_and_connect_output(&obj, file);
        if output_result.is_broken() {
            let sig = parse_error_signature(output_result.get_output());
            println!("[STILL BROKEN] {} - {}", file, sig.short_description);
            still_broken += 1;
        } else {
            println!("[FIXED] {}", file);
            // Move to archive with unique name to avoid collisions
            let archive_dir = format!("{state_dir}/archived_fixed");
            let _ = fs::create_dir_all(&archive_dir);
            let file_name = Path::new(file).file_name().unwrap().to_string_lossy().to_string();
            let mut dest = format!("{archive_dir}/{file_name}");
            if Path::new(&dest).exists() {
                dest = format!("{archive_dir}/{}_{}", rand::random::<u32>(), file_name);
            }
            let _ = fs::rename(file, dest);
            fixed += 1;
        }
    }

    println!("\nRegression check: {still_broken} still broken, {fixed} fixed");
}

fn copy_dir_contents(src: &str, dst: &str) {
    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dst_path = Path::new(dst).join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_contents(&src_path.to_string_lossy(), &dst_path.to_string_lossy());
            } else {
                let _ = fs::copy(&src_path, &dst_path);
            }
        }
    }
}
