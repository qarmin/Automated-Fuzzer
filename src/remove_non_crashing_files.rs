use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use jwalk::WalkDir;
use log::info;
use rand::random;
use rayon::prelude::*;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::common::{
    collect_command_to_string, execute_command_and_connect_output,
    remove_and_create_entire_folder,
};
use crate::error_signature::{parse_error_signature, get_legacy_error_type};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub const MAX_FILES: usize = 999_999_999_999;

pub(crate) fn remove_non_crashing_files(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    obj.remove_non_parsable_files(&settings.broken_files_dir);

    let broken_files: Vec<String> = collect_broken_files(settings).into_iter().take(MAX_FILES).collect();
    info!("Found {} broken files to check", broken_files.len());

    remove_non_crashing(broken_files, settings, obj, 1);

    let broken_files: Vec<String> = collect_broken_files(settings);
    info!("After checking {} broken files left", broken_files.len());
}

fn remove_non_crashing(broken_files: Vec<String>, settings: &Setting, obj: &Box<dyn ProgramConfig>, step: u32) {
    let still_broken_files = broken_files
        .into_iter()
        .filter(|e| {
            let res = fs::metadata(e).map(|e| e.len()).unwrap_or_default() < settings.max_file_size_limit;
            if !res {
                let _ = fs::remove_file(e);
            }
            res
        })
        .collect::<Vec<_>>();
    info!("After filtering by size, {} files left", still_broken_files.len());

    let atomic_counter = AtomicUsize::new(0);
    let all = still_broken_files.len();
    let results = still_broken_files
        .into_par_iter()
        .filter_map(|full_name| {
            let start_text = fs::read(&full_name).unwrap();
            let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
            if idx.is_multiple_of(100) {
                info!("_____ Processed already {idx} / {all} (step {step})");
            }
            let output_result = execute_command_and_connect_output(obj, &full_name);
            if settings.debug_print_results {
                info!("File {full_name}\n{}", output_result.get_output());
            }
            if output_result.is_broken() {
                fs::write(&full_name, start_text).unwrap();
                return Some((full_name, output_result.get_output().trim().to_string()));
            }
            info!("File {full_name} is not broken, and will be removed");

            fs::remove_file(&full_name).unwrap();
            None
        })
        .collect::<Vec<_>>();

    remove_and_create_entire_folder(&settings.temp_folder);

    save_results_to_file(obj, settings, results);
}

pub(crate) fn save_results_to_file(obj: &Box<dyn ProgramConfig>, settings: &Setting, content: Vec<(String, String)>) {
    info!("Saving results to file");
    let command = obj.get_full_command("TEST___FILE");
    let command_str = collect_command_to_string(&command);

    for (file_name, result) in content {
        let content = fs::read(&file_name).unwrap();
        let extension = Path::new(&file_name)
            .extension()
            .map(|e| e.to_str().unwrap().to_string())
            .unwrap_or_default();
        let command_str_with_extension = command_str.replace("TEST___FILE", &format!("TEST___FILE.{extension}"));

        // Parse the error signature for detailed grouping
        let signature = parse_error_signature(&result);
        let legacy_error_type = get_legacy_error_type(&result);

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
        }

        let folder = format!(
            "{}/{}_{}__({} bytes) - {}",
            settings.temp_folder,
            settings.name,
            if legacy_error_type.is_empty() { &signature.error_type } else { legacy_error_type },
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

        // Write metadata file with signature info
        let metadata = format!(
            r#"error_type = "{}"
error_signature = "{}"
short_description = "{}"
source_file = "{}"
issue_title = "{}"
project = "{}"
found_date = "{}"
file_size = {}
"#,
            signature.error_type,
            signature.signature(),
            signature.short_description,
            signature.source_file.as_deref().unwrap_or(""),
            signature.issue_title(),
            settings.name,
            chrono::Local::now().format("%Y-%m-%d"),
            content.len(),
        );
        fs::write(format!("{folder}/to_report_metadata.toml"), metadata).unwrap();

        fs::write(format!("{folder}/crash_output.txt"), &result).unwrap();

        if !extension.is_empty() {
            fs::write(format!("{folder}/problematic_file.{extension}"), &content).unwrap();
        } else {
            fs::write(format!("{folder}/problematic_file"), &content).unwrap();
        }

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

pub fn zip_file(zip_filename: &str, file_name: &str, file_code: &[u8]) {
    let zip_file = File::create(zip_filename).unwrap();
    let mut zip_writer = ZipWriter::new(zip_file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let _ = zip_writer.start_file(file_name, options);
    let _ = zip_writer.write_all(file_code);
}
