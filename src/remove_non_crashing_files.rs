use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use jwalk::WalkDir;
use log::info;
use rand::random;
use rayon::prelude::*;

use crate::common::{
    collect_command_to_string, execute_command_and_connect_output, execute_command_on_pack_of_files,
    remove_and_create_entire_folder, CheckGroupFileMode,
};
use crate::minimal_rules::zip_file;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub const MAX_FILES: usize = 999_999_999_999;

pub fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    obj.remove_non_parsable_files(&settings.broken_files_dir);

    let broken_files: Vec<String> = collect_broken_files(settings).into_iter().take(MAX_FILES).collect();
    info!("Found {} broken files to check", broken_files.len());
    // let broken_files_before = broken_files.len();

    remove_non_crashing(broken_files, settings, obj, 1);

    // let broken_files: Vec<String> = collect_broken_files(settings);
    // let broken_files_after = broken_files.len();
    //
    // remove_non_crashing(broken_files, settings, obj, 2);
    //
    // let broken_files: Vec<String> = collect_broken_files(settings);
    // let broken_files_after2 = broken_files.len();
    //
    // info!("At start there was {broken_files_before} files, after first pass {broken_files_after}, after second pass {broken_files_after2}");
    // if broken_files_after != broken_files_after2 {
    //     error!("There is unstable checking for broken files");
    // }
    let broken_files: Vec<String> = collect_broken_files(settings);
    info!("After checking {} broken files left", broken_files.len());
}

#[allow(dead_code)]
#[allow(unused)]
fn remove_non_crashing_in_group(
    broken_files: Vec<String>,
    settings: &Setting,
    obj: &Box<dyn ProgramConfig>,
) -> Vec<String> {
    // TODO this check may be broken - test it
    return broken_files;

    if settings.grouping == 1 || obj.get_files_group_mode() == CheckGroupFileMode::None {
        return broken_files;
    }
    info!("Removing non-crashing files in group");
    let group_size = 20;
    let atomic_counter = AtomicUsize::new(0);
    let all_chunks = broken_files.chunks(group_size).count();

    let still_broken_files = broken_files
        .into_par_iter()
        .chunks(group_size)
        .enumerate()
        .filter_map(|(chunk_idx, chunk)| {
            let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
            info!("_____ Processed already {idx} / {all_chunks} chunk (step {group_size})");
            let temp_folder = format!("{}/{}", settings.temp_folder, random::<u64>());
            fs::create_dir_all(&temp_folder).unwrap();

            for (idx, full_name) in chunk.iter().enumerate() {
                let extension = Path::new(full_name).extension().unwrap().to_str().unwrap();
                let new_name = format!("{temp_folder}/{idx}.{extension}");
                fs::copy(full_name, &new_name).unwrap();
            }

            let output_result = execute_command_on_pack_of_files(obj, &temp_folder, &[]);
            if settings.debug_print_results {
                info!("File pack {temp_folder}\n{}", output_result.get_output());
            }

            fs::remove_dir_all(&temp_folder).unwrap();

            if output_result.is_broken() {
                info!("Chunk {chunk_idx} is broken");
                Some(chunk.clone())
            } else {
                info!("Chunk {chunk_idx} is not broken");
                for full_name in chunk {
                    fs::remove_file(&full_name).unwrap();
                }
                None
            }
        })
        .flatten()
        .collect();
    info!("Removing non-crashing files in group done");
    still_broken_files
}
fn remove_non_crashing(broken_files: Vec<String>, settings: &Setting, obj: &Box<dyn ProgramConfig>, step: u32) {
    // Processing in groups
    // let still_broken_files = broken_files;
    let still_broken_files = if obj.get_files_group_mode() != CheckGroupFileMode::None {
        remove_non_crashing_in_group(broken_files, settings, obj)
    } else {
        broken_files
    };

    let atomic_counter = AtomicUsize::new(0);
    let all = still_broken_files.len();
    let results = still_broken_files
        .into_par_iter()
        .filter_map(|full_name| {
            let start_text = fs::read(&full_name).unwrap();
            let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
            if idx % 100 == 0 {
                info!("_____ Processed already {idx} / {all} (step {step})");
            }
            let output_result = execute_command_and_connect_output(obj, &full_name);
            if settings.debug_print_results {
                info!("File {full_name}\n{}", output_result.get_output());
            }
            if output_result.is_broken() {
                fs::write(&full_name, start_text).unwrap();
                return Some((full_name, output_result.get_output().trim().to_string()));
            };
            info!("File {full_name} is not broken, and will be removed");

            fs::remove_file(&full_name).unwrap();
            None
        })
        .collect::<Vec<_>>();

    remove_and_create_entire_folder(&settings.temp_folder);

    save_results_to_file(obj, settings, results);
}

pub fn save_results_to_file(obj: &Box<dyn ProgramConfig>, settings: &Setting, content: Vec<(String, String)>) {
    info!("Saving results to file");
    let command = obj.get_full_command("TEST___FILE");
    let command_str = collect_command_to_string(&command);

    for (file_name, result) in content {
        let content = fs::read(&file_name).unwrap();
        let extension = Path::new(&file_name).extension().unwrap().to_str().unwrap();
        let command_str_with_extension = command_str.replace("TEST___FILE", &format!("TEST___FILE.{extension}"));
        let mut file_content = String::new();

        let mut cnt_text = String::new();
        let content_to_string = String::from_utf8(content.clone());
        if let Ok(content_string) = content_to_string {
            cnt_text += "File content(at the bottom should be attached raw, not formatted file - github removes some non-printable characters, so copying from here may not work):\n";
            cnt_text += "```\n";
            cnt_text += &content_string;
            cnt_text += "\n```";
        } else {
            cnt_text += "File content is binary, so is available only in zip file";
        };
        let error_type = match result {
            _ if result.contains("memory allocation of") => "memory_failure",
            _ if result.contains("stack overflow") => "stack_overflow",
            _ if result.contains("stack-overflow") => "asan_stack_overflow",
            _ if result.contains("heap-use-after-free") => "asan_heap_use_after_free",
            _ if result.contains("segmentation fault") => "segmentation_fault",
            _ if result.contains("Killed") => "killed",
            _ if result.contains("is not a char boundary") => "char_boundary",
            _ if result.contains("divide by zero") => "divide_by_zero",
            _ if result.contains("attempt to subtract with overflow") => "overflow_s",
            _ if result.contains("attempt to multiply with overflow") => "overflow_m",
            _ if result.contains("attempt to add with overflow") => "overflow_a",
            _ if result.contains("attempt to shift right with overflow") => "overflow_sr",
            _ if result.contains("attempt to shift left with overflow") => "overflow_sl",
            _ if result.contains("index out of bounds:") => "index_out_of_bounds",
            _ if result.contains("is out of bounds:") => "out_of_bounds",
            _ if result.contains("is out of bounds of") => "out_of_bounds_of",
            _ if result.contains("Option::unwrap()") => "option_unwrap",
            _ if result.contains("Result::unwrap()") => "result_unwrap",
            _ if result.contains("when slicing `") => "slicing",
            _ if result.contains("internal error: entered unreachable code") => "unreachable_code",
            _ if result.contains("not implemented: ") => "not_implemented",
            _ if result.contains("Aborted") => "aborted",
            _ if result.contains("output signal \"Some(15)\"") => "out_of_memory",
            _ if result.contains("AddressSanitizer: out of memory") => "asan_out_of_memory",
            _ if result.contains("output signal \"Some(11)\"") => "segmentation_fault2",
            _ if result.contains("AddressSanitizer") => "address_sanitizer",
            _ if result.contains("ThreadSanitizer") => "thread_sanitizer",
            _ if result.contains("LeakSanitizer") => "leak_sanitizer",
            _ if result.contains("assertion `") => "assertion",
            _ if result.contains("assertion failed:") => "assertion_failed",
            _ if result.contains("out of range for") => "out_of_range",
            _ if result.contains("panicked at ") => "panicked",
            _ if result.contains("RUST_BACKTRACE") => "panic",
            _ if result.contains("output status \"Some(124)\"") => "timeout",
            _ if result.contains("Fix introduced a syntax error") => "syntax_error",
            _ => "",
        };

        let folder = format!(
            "{}/{}_{}__({} bytes) - {}",
            settings.temp_folder,
            settings.name,
            error_type,
            content.len(),
            random::<u64>()
        );
        let _ = fs::create_dir_all(&folder);

        file_content += &r#"$CNT_TEXT

command
```
$COMMAND
```

App was compiled with nightly rust compiler to be able to use address sanitizer
(You can ignore this part if there is no address sanitizer error)
On Ubuntu 24.04, the commands to compile were:
```
rustup default nightly
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu

export RUST_BACKTRACE=1 # or full depending on project
export ASAN_SYMBOLIZER_PATH=$(which llvm-symbolizer-18)
export ASAN_OPTIONS=symbolize=1
RUSTFLAGS="-Zsanitizer=address" cargo +nightly build --target x86_64-unknown-linux-gnu
```

cause this
```
$ERROR
```
"#
        .replace("$CNT_TEXT", &cnt_text)
        .replace("$COMMAND", &command_str_with_extension)
        .replace("$ERROR", &result)
        .replace("\n\n```", "\n```");

        fs::write(format!("{folder}/to_report.txt"), &file_content).unwrap();

        fs::write(format!("{folder}/problematic_file.{extension}"), &content).unwrap();

        let zip_filename = format!("{folder}/compressed.zip");
        let only_file_name = Path::new(&file_name).file_name().unwrap().to_string_lossy().to_string();
        zip_file(&zip_filename, &only_file_name, &content);
    }
}

fn collect_broken_files(settings: &Setting) -> Vec<String> {
    WalkDir::new(&settings.broken_files_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().is_file() {
                return None;
            }

            let path = entry.path().to_string_lossy().to_string();
            let path_to_lowercase = path.to_lowercase();

            if settings.extensions.iter().any(|e| path_to_lowercase.ends_with(e)) {
                return Some(path);
            }

            None
        })
        .collect()
}
