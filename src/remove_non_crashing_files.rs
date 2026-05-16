use std::collections::HashMap;
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

use crate::common::{collect_command_to_string, execute_command_and_connect_output, remove_and_create_entire_folder};
use crate::error_signature::{ErrorSignature, parse_error_signature};
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
    log_settings_summary("remove_non_crashing_files", settings, obj);

    obj.remove_non_parsable_files(&settings.broken_files_dir);

    let broken_files: Vec<String> = collect_broken_files(settings).into_iter().take(MAX_FILES).collect();
    info!("Found {} broken files to check", broken_files.len());

    remove_non_crashing(broken_files, settings, obj, 1);

    let broken_files: Vec<String> = collect_broken_files(settings);
    info!("After checking {} broken files left", broken_files.len());
}

/// Print a compact summary of the loaded configuration. Useful when something
/// goes wrong silently (e.g. wrong config picked up, mismatched extensions,
/// 0 reports generated): the very first lines of the log make the situation
/// obvious without needing to grep through the TOML.
pub(crate) fn log_settings_summary(context: &str, settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let command = obj.get_full_command("FILE");
    let command_str = collect_command_to_string(&command);
    info!("=== CONFIG SUMMARY ({context}) ===");
    info!("  project name           = {}", settings.name);
    info!("  command                = {command_str}");
    info!("  extensions             = {:?}", settings.extensions);
    info!("  broken_files_dir       = {}", settings.broken_files_dir);
    info!("  valid_input_files_dir  = {}", settings.valid_input_files_dir);
    info!("  temp_folder (reports)  = {}", settings.temp_folder);
    info!("  max_file_size_limit    = {}", settings.max_file_size_limit);
    info!(
        "  remove_non_crashing    = {}",
        settings.remove_non_crashing_items_from_broken_files
    );
    info!("=== END CONFIG SUMMARY ===");
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
            let Ok(start_text) = fs::read(&full_name) else {
                log::warn!("Cannot read {full_name}; skipping");
                return None;
            };
            let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
            if idx.is_multiple_of(100) {
                info!("_____ Processed already {idx} / {all} (step {step})");
            }
            let Some(output_result) = execute_command_and_connect_output(obj, &full_name) else {
                // Filesystem error during the run — don't classify, don't delete.
                return None;
            };
            if settings.debug_print_results {
                info!("File {full_name}\n{}", output_result.get_output());
            }
            if output_result.is_broken() {
                if let Err(e) = fs::write(&full_name, start_text) {
                    log::warn!("Cannot restore {full_name}: {e}");
                }
                return Some((full_name, output_result.get_output().trim().to_string()));
            }
            info!("File {full_name} is not broken, and will be removed");

            if let Err(e) = fs::remove_file(&full_name) {
                log::warn!("Cannot remove non-broken {full_name}: {e}");
            }
            None
        })
        .collect::<Vec<_>>();

    // Only reset temp_folder when we actually have reports to write —
    // otherwise we'd destroy reports from a previous run for no reason.
    if !results.is_empty() {
        remove_and_create_entire_folder(&settings.temp_folder);
    }

    save_results_to_file(obj, settings, results);
}

struct CrashCandidate {
    file_name: String,
    result: String,
    signature: ErrorSignature,
    group_key: String,
    file_size: usize,
}

pub(crate) fn save_results_to_file(obj: &Box<dyn ProgramConfig>, settings: &Setting, content: Vec<(String, String)>) {
    if content.is_empty() {
        log::warn!(
            "save_results_to_file: 0 reports will be written. \
             Reasons this can happen: (1) no input files matched extensions {:?} in {}, \
             (2) all crashes were filtered out by max_file_size_limit={}, \
             (3) none of the candidates reproduced under the configured command (binary mismatch?). \
             Check the CONFIG SUMMARY at the top of the log.",
            settings.extensions, settings.broken_files_dir, settings.max_file_size_limit
        );
        return;
    }
    info!(
        "save_results_to_file: starting with {} reproducible crashes; reports go to {}",
        content.len(),
        settings.temp_folder
    );
    let command = obj.get_full_command("TEST___FILE");
    let command_str = collect_command_to_string(&command);
    let crate_code = try_read_crate_code(&settings.name);

    //  Group all crashes by error signature (project__type__src_line)
    let mut groups: HashMap<String, Vec<CrashCandidate>> = HashMap::new();
    for (file_name, result) in content {
        let file_size = fs::metadata(&file_name).map(|m| m.len() as usize).unwrap_or(usize::MAX);
        let signature = parse_error_signature(&result);

        let src_part = signature
            .source_file
            .as_deref()
            .unwrap_or("unknown")
            .replace(['/', '\\'], "_");
        let line_suffix = signature.source_line.map(|l| format!("_{l}")).unwrap_or_default();
        let group_key = format!(
            "{}__{}__{}{}",
            settings.name, signature.error_type, src_part, line_suffix
        );

        groups.entry(group_key.clone()).or_default().push(CrashCandidate {
            file_name,
            result,
            signature,
            group_key,
            file_size,
        });
    }

    let total_crashes: usize = groups.values().map(|v| v.len()).sum();
    let total_groups = groups.len();
    info!("Grouped {total_crashes} crashes into {total_groups} unique signatures; keeping smallest per group");

    //  For each group, pick the smallest crash as representative
    for (group_key, mut candidates) in groups {
        let group_size = candidates.len();
        candidates.sort_by_key(|c| c.file_size);
        let rep = candidates.into_iter().next().unwrap();
        info!(
            "Group [{group_key}]: {group_size} crashes, picking smallest ({} bytes, {})",
            rep.file_size, rep.file_name
        );
        write_report_for_representative(settings, &rep, &command_str, crate_code.as_deref());
    }
}

fn write_report_for_representative(
    settings: &Setting,
    rep: &CrashCandidate,
    command_str: &str,
    crate_code: Option<&str>,
) {
    let content = match fs::read(&rep.file_name) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to read representative {}: {e}", rep.file_name);
            return;
        }
    };
    let extension = Path::new(&rep.file_name)
        .extension()
        .map(|e| e.to_str().unwrap().to_string())
        .unwrap_or_default();
    let command_str_with_extension = command_str.replace("TEST___FILE", &format!("TEST___FILE.{extension}"));

    let group_folder = format!("{}/{}", settings.temp_folder, rep.group_key);
    let _ = fs::create_dir_all(&group_folder);

    let file_size = content.len();
    let file_idx = random::<u64>();
    let folder = format!("{group_folder}/{file_size}_bytes_{file_idx}");
    let _ = fs::create_dir_all(&folder);

    //  Build to_report.txt
    let mut report = String::new();

    let content_to_string = String::from_utf8(content.clone());
    if let Ok(content_string) = &content_to_string {
        report += "File content(at the bottom should be attached raw, not formatted file - github removes some non-printable characters, so copying from here may not work):\n";
        report += "```\n";
        report += content_string;
        report += "\n```\n\n";
    } else {
        report += &format!(
            "File content is binary ({} bytes), so is available only in zip file at the bottom of page\n\n",
            humansize::format_size(file_size, humansize::BINARY)
        );
    }

    let reproducer_path = format!("{}.reproducer.rs", rep.file_name);
    let reproducer_code = fs::read_to_string(&reproducer_path).ok();
    let code_to_include = reproducer_code.as_deref().or(crate_code);

    if let Some(code) = code_to_include {
        report += "### Reproducer\n\n";
        report += "I tried this code:\n\n";
        report += "```rust\n";
        report += code;
        report += "\n```\n\n";
    }

    report += "command\n```\n";
    report += &command_str_with_extension;
    report += "\n```\n\n";

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
    report += &rep.result;
    report += "\n```\n";

    fs::write(format!("{folder}/to_report.txt"), &report).unwrap();

    let metadata = ReportMetadata {
        error_type: rep.signature.error_type.clone(),
        error_signature: rep.signature.signature(),
        short_description: rep.signature.short_description.clone(),
        source_file: rep.signature.source_file.as_deref().unwrap_or("").to_string(),
        source_line: rep.signature.source_line.unwrap_or(0),
        issue_title: rep.signature.issue_title(),
        project: settings.name.clone(),
        found_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        file_size,
    };
    let metadata_str = toml::to_string_pretty(&metadata).unwrap();
    fs::write(format!("{folder}/to_report_metadata.toml"), metadata_str).unwrap();

    fs::write(format!("{folder}/crash_output.txt"), &rep.result).unwrap();

    if !extension.is_empty() {
        fs::write(format!("{folder}/problematic_file.{extension}"), &content).unwrap();
    } else {
        fs::write(format!("{folder}/problematic_file"), &content).unwrap();
    }
    let zip_filename = format!("{folder}/compressed.zip");
    let only_file_name = Path::new(&rep.file_name)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    zip_file(&zip_filename, &only_file_name, &content);

    let issue_title = rep.signature.issue_title();
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

    let mut body = String::new();
    body += &report;
    body += "\n[Compressed archive placeholder - looks that I forgot to attach the file.]\n";
    fs::write(format!("{folder}/issue_body.md"), &body).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(format!("{folder}/create_issue.sh"), fs::Permissions::from_mode(0o755));
    }
}

/// Try to read crate source code from crates/<name>/src/main.rs
/// Simplifies by replacing the directory-walking main() with a minimal version.
fn try_read_crate_code(project_name: &str) -> Option<String> {
    let path = format!("crates/{}/src/main.rs", project_name);
    let code = fs::read_to_string(&path).ok()?;

    // Build a simplified version:
    // - Replace the boilerplate main() with a simple one-file version
    // - Remove walkdir import and usage
    let mut simplified = String::new();

    // Collect use lines (skip walkdir)
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use walkdir") {
            continue;
        }
        if trimmed.starts_with("use ") || trimmed.starts_with("//") || trimmed.is_empty() {
            simplified.push_str(line);
            simplified.push('\n');
            continue;
        }
        break; // stop at first non-use/non-comment/non-empty line
    }

    // Add simplified main
    simplified.push_str("fn main() {\n");
    simplified.push_str("    let path = std::env::args().nth(1).unwrap();\n");
    simplified.push_str("    check_file(&path);\n");
    simplified.push_str("}\n\n");

    // Add check_file and everything after it. If the crate doesn't follow the
    // `fn check_file(path: &str)` convention, the simplified main() above has no
    // function to call — skip the reproducer entirely rather than dumping the
    // raw, un-simplified source.
    let idx = code.find("fn check_file(")?;
    simplified.push_str(&code[idx..]);

    Some(simplified)
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
            let repo_part = after.split('"').next()?.trim_end_matches('/').trim_end_matches(".git");
            // Should have exactly one slash: owner/repo
            if repo_part.contains('/') && repo_part.matches('/').count() == 1 {
                return Some(repo_part.to_string());
            }
        }
    }
    None
}

fn collect_broken_files(settings: &Setting) -> Vec<String> {
    // Empty extensions list = accept everything. Used by the CI fallback
    // ("Generate reports from crashes"), where /opt/ALL_CRASHES contains a mix
    // of original-extension files (e.g. .png) and cargo-fuzz artifacts with no
    // extension at all (e.g. `crash-<hash>`, `crash-<hash>_minimized`). The
    // files are already confirmed crashes upstream, so filtering by extension
    // there would silently drop most of them.
    let accept_all = settings.extensions.is_empty();

    let mut total = 0usize;
    let mut skipped_examples: Vec<String> = Vec::new();
    let matched: Vec<String> = WalkDir::new(&settings.broken_files_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().is_file() {
                return None;
            }
            total += 1;
            let path = entry.path().to_string_lossy().to_string();

            if accept_all {
                return Some(path);
            }

            let path_to_lowercase = path.to_lowercase();
            // Match if extension is in the allow-list OR the file has no
            // extension at all (cargo-fuzz outputs like `crash-<hash>`).
            let has_no_extension = Path::new(&path).extension().is_none();
            if has_no_extension || settings.extensions.iter().any(|e| path_to_lowercase.ends_with(e)) {
                Some(path)
            } else {
                if skipped_examples.len() < 5 {
                    skipped_examples.push(path);
                }
                None
            }
        })
        .collect();

    info!(
        "collect_broken_files: dir={} total_files={} matched={} extensions={:?} accept_all={accept_all}",
        settings.broken_files_dir,
        total,
        matched.len(),
        settings.extensions,
    );
    if matched.is_empty() && total > 0 {
        log::warn!(
            "collect_broken_files: {total} files present but none match extensions {:?}. Examples skipped: {:?}",
            settings.extensions, skipped_examples
        );
    }
    matched
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
