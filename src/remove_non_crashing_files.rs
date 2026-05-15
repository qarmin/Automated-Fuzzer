use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use jwalk::WalkDir;
use log::info;
use rand::random;
use rayon::prelude::*;
use serde::Serialize;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::common::{
    collect_command_to_string, execute_command_and_connect_output,
    remove_and_create_entire_folder,
};
use crate::error_signature::{parse_error_signature, get_legacy_error_type};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

#[derive(Serialize)]
struct ReportMetadata {
    error_type: String,
    error_signature: String,
    short_description: String,
    source_file: String,
    source_line: u32,
    issue_title: String,
    project: String,
    found_date: String,
    file_size: usize,
}

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

    // Try to read crate source code for library-style reports
    let crate_code = try_read_crate_code(&settings.name);

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

        // ── Build group folder name: project__type__file_line ──
        let src_part = signature
            .source_file
            .as_deref()
            .unwrap_or("unknown")
            .replace('/', "_")
            .replace('\\', "_");
        let line_suffix = signature
            .source_line
            .map(|l| format!("_{l}"))
            .unwrap_or_default();
        let error_type_str = if legacy_error_type.is_empty() {
            &signature.error_type
        } else {
            legacy_error_type
        };
        // e.g. "zune__assertion_eq__src_image.rs_482"
        let group_folder = format!(
            "{}/{}__{}__{}{}",
            settings.temp_folder, settings.name, error_type_str, src_part, line_suffix
        );
        let _ = fs::create_dir_all(&group_folder);

        let file_idx = random::<u64>();
        let folder = format!("{group_folder}/{file_idx}");
        let _ = fs::create_dir_all(&folder);

        // ── Build to_report.txt ──
        let mut report = String::new();

        // File content section
        let content_to_string = String::from_utf8(content.clone());
        if let Ok(content_string) = &content_to_string {
            report += "File content(at the bottom should be attached raw, not formatted file - github removes some non-printable characters, so copying from here may not work):\n";
            report += "```\n";
            report += content_string;
            report += "\n```\n\n";
        } else {
            report += "File content is binary, so is available only in zip file\n\n";
        }

        // If we have crate code, include it as reproducer (library style)
        if let Some(ref code) = crate_code {
            report += "### Reproducer\n\n";
            report += "I tried this code:\n\n";
            report += "```rust\n";
            report += code;
            report += "\n```\n\n";
        }

        // Command
        report += "command\n```\n";
        report += &command_str_with_extension;
        report += "\n```\n\n";

        // ASAN instructions
        report += "App was compiled with nightly rust compiler to be able to use address sanitizer\n";
        report += "(You can ignore this part if there is no address sanitizer error)\n";
        report += "On Ubuntu 24.04, the commands to compile were:\n```\n";
        report += "rustup default nightly\n";
        report += "rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu\n";
        report += "rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu\n\n";
        report += "export RUST_BACKTRACE=1 # or full depending on project\n";
        report += "export ASAN_SYMBOLIZER_PATH=$(which llvm-symbolizer-18)\n";
        report += "export ASAN_OPTIONS=symbolize=1\n";
        report += "RUSTFLAGS=\"-Zsanitizer=address\" cargo +nightly build --target x86_64-unknown-linux-gnu\n";
        report += "```\n\ncause this\n```\n";
        report += &result;
        report += "\n```\n";

        fs::write(format!("{folder}/to_report.txt"), &report).unwrap();

        // ── Metadata ──
        let metadata = ReportMetadata {
            error_type: signature.error_type.clone(),
            error_signature: signature.signature(),
            short_description: signature.short_description.clone(),
            source_file: signature.source_file.as_deref().unwrap_or("").to_string(),
            source_line: signature.source_line.unwrap_or(0),
            issue_title: signature.issue_title(),
            project: settings.name.clone(),
            found_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            file_size: content.len(),
        };
        let metadata_str = toml::to_string_pretty(&metadata).unwrap();
        fs::write(format!("{folder}/to_report_metadata.toml"), metadata_str).unwrap();

        // ── Crash output ──
        fs::write(format!("{folder}/crash_output.txt"), &result).unwrap();

        // ── Problematic file + zip ──
        if !extension.is_empty() {
            fs::write(format!("{folder}/problematic_file.{extension}"), &content).unwrap();
        } else {
            fs::write(format!("{folder}/problematic_file"), &content).unwrap();
        }
        let zip_filename = format!("{folder}/compressed.zip");
        let only_file_name = Path::new(&file_name).file_name().unwrap().to_string_lossy().to_string();
        zip_file(&zip_filename, &only_file_name, &content);

        // ── create_issue.sh ──
        let issue_title = signature.issue_title();
        let repo = detect_github_repo(&settings.name);
        let repo_flag = if let Some(ref r) = repo {
            format!("--repo \"{r}\"")
        } else {
            "# WARNING: Could not detect repo. Add --repo \"owner/repo\" manually.".to_string()
        };

        let script = format!(
            r#"#!/bin/bash
# Issue: {issue_title}
# Repo:  {repo_display}
# Review the issue_body.md before running!

DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Creating issue: {issue_title}"
echo ""

ISSUE_URL=$(gh issue create \
    {repo_flag} \
    --title "$(cat "$DIR/issue_title.txt")" \
    --body-file "$DIR/issue_body.md" 2>&1)

echo ""
echo "Issue created:"
echo "$ISSUE_URL"
echo ""
echo "Opening in browser to attach compressed.zip..."
xdg-open "$ISSUE_URL" 2>/dev/null || open "$ISSUE_URL" 2>/dev/null || echo "Open manually: $ISSUE_URL"
echo ""
echo "Attach this file: $DIR/compressed.zip"
"#,
            repo_display = repo.as_deref().unwrap_or("UNKNOWN - set manually"),
        );
        fs::write(format!("{folder}/create_issue.sh"), &script).unwrap();
        fs::write(format!("{folder}/issue_title.txt"), &issue_title).unwrap();

        // Build issue body markdown
        let mut body = String::new();
        body += &report;
        body += "\n[Compressed archive placeholder (attachment not included) - looks that I forgot to attach the file.]\n";
        fs::write(format!("{folder}/issue_body.md"), &body).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                format!("{folder}/create_issue.sh"),
                fs::Permissions::from_mode(0o755),
            );
        }
    }
}

/// Try to read crate source code from crates/<name>/src/main.rs
fn try_read_crate_code(project_name: &str) -> Option<String> {
    let path = format!("crates/{}/src/main.rs", project_name);
    fs::read_to_string(&path).ok()
}

/// Detect GitHub repo (owner/name) from crates/<name>/Cargo.toml git dependencies
fn detect_github_repo(project_name: &str) -> Option<String> {
    let cargo_path = format!("crates/{}/Cargo.toml", project_name);
    let content = fs::read_to_string(&cargo_path).ok()?;

    // Look for git = "https://github.com/OWNER/REPO..." (first non-commented one)
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(idx) = trimmed.find("github.com/") {
            let after = &trimmed[idx + "github.com/".len()..];
            // Extract "OWNER/REPO" — strip .git and anything after
            let repo_part = after
                .split('"')
                .next()?
                .trim_end_matches('/')
                .trim_end_matches(".git");
            // Should have exactly one slash: owner/repo
            if repo_part.contains('/') && repo_part.matches('/').count() == 1 {
                return Some(repo_part.to_string());
            }
        }
    }
    None
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
    let zip_file = match File::create(zip_filename) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to create zip file {zip_filename}: {e}");
            return;
        }
    };
    let mut zip_writer = ZipWriter::new(zip_file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    if let Err(e) = zip_writer.start_file(file_name, options) {
        log::error!("Failed to start zip entry {file_name}: {e}");
        return;
    }
    if let Err(e) = zip_writer.write_all(file_code) {
        log::error!("Failed to write zip content for {file_name}: {e}");
        return;
    }
    if let Err(e) = zip_writer.finish() {
        log::error!("Failed to finalize zip {zip_filename}: {e}");
    }
}
