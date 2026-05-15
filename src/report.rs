use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::info;

use crate::error_signature::parse_error_signature;

/// List all unreported crash results
pub fn list_reports(results_dir: &str, project_filter: Option<&str>) {
    let path = Path::new(results_dir);
    if !path.exists() {
        println!("Results directory '{results_dir}' does not exist.");
        return;
    }

    let mut reports: HashMap<String, Vec<String>> = HashMap::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            let dir_path = entry.path();
            let metadata_path = dir_path.join("to_report_metadata.toml");
            let report_path = dir_path.join("to_report.txt");

            if !report_path.exists() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();

            // Try to read metadata for better grouping
            if metadata_path.exists() {
                if let Ok(meta_content) = fs::read_to_string(&metadata_path) {
                    let project = extract_toml_value(&meta_content, "project").unwrap_or_default();
                    if let Some(filter) = project_filter {
                        if project != filter {
                            continue;
                        }
                    }
                    let sig = extract_toml_value(&meta_content, "error_signature").unwrap_or_default();
                    let title = extract_toml_value(&meta_content, "issue_title").unwrap_or_default();
                    let size = extract_toml_value(&meta_content, "file_size").unwrap_or_default();

                    reports.entry(sig.clone()).or_default().push(dir_name.clone());
                    if reports[&sig].len() == 1 {
                        println!("[NEW] {sig}  ({size} bytes) - {title}");
                    }
                    continue;
                }
            }

            // Fallback: try to parse from crash_output.txt
            let crash_output = dir_path.join("crash_output.txt");
            if crash_output.exists() {
                if let Ok(output) = fs::read_to_string(&crash_output) {
                    let sig = parse_error_signature(&output);
                    let key = sig.signature();
                    reports.entry(key.clone()).or_default().push(dir_name);
                    if reports[&key].len() == 1 {
                        println!("[NEW] {} - {}", key, sig.issue_title());
                    }
                    continue;
                }
            }

            println!("[???] {dir_name}");
        }
    }

    let total_unique = reports.len();
    let total_files: usize = reports.values().map(|v| v.len()).sum();
    println!("\nTotal: {total_unique} unique signatures, {total_files} crash files");
}

/// Create a report for a specific crash directory
pub fn create_report(crash_dir: &str, repo: Option<&str>, version: Option<&str>, variant: &str) {
    let dir = Path::new(crash_dir);
    if !dir.exists() {
        eprintln!("Directory '{crash_dir}' does not exist.");
        return;
    }

    let to_report = dir.join("to_report.txt");
    let metadata_path = dir.join("to_report_metadata.toml");
    let crash_output = dir.join("crash_output.txt");

    // Determine the issue title
    let title = if metadata_path.exists() {
        let meta = fs::read_to_string(&metadata_path).unwrap_or_default();
        extract_toml_value(&meta, "issue_title").unwrap_or_else(|| "Crash found by fuzzer".to_string())
    } else if crash_output.exists() {
        let output = fs::read_to_string(&crash_output).unwrap_or_default();
        let sig = parse_error_signature(&output);
        sig.issue_title()
    } else {
        "Crash found by fuzzer".to_string()
    };

    // Read existing report text
    let report_body = if to_report.exists() {
        fs::read_to_string(&to_report).unwrap_or_default()
    } else {
        eprintln!("No to_report.txt found in {crash_dir}");
        return;
    };

    // Build the issue body
    let version_line = if let Some(v) = version {
        format!("Self compiled {v}\n\n")
    } else {
        String::new()
    };

    let body = match variant {
        "library" => {
            format!(
                "{version_line}### What happened?\n\n{report_body}\n\n### Expected behavior\n\n_No response_\n\n### Assets\n\n[compressed.zip](attachment)\n"
            )
        }
        _ => {
            // CLI variant
            format!("{version_line}### What happened?\n\n{report_body}\n\n[compressed.zip](attachment)\n")
        }
    };

    // Write outputs
    let title_path = dir.join("issue_title.txt");
    let body_path = dir.join("issue_body.md");
    let script_path = dir.join("create_issue.sh");

    fs::write(&title_path, &title).unwrap();
    fs::write(&body_path, &body).unwrap();

    // Generate the create_issue.sh script
    let repo_arg = if let Some(r) = repo {
        format!(" \\\n     --repo \"{r}\"")
    } else {
        String::new()
    };

    let script = format!(
        r#"#!/bin/bash
# Review the issue before creating!
# Title: {title}
#
# IMPORTANT: After creating the issue, manually attach compressed.zip
# through the GitHub web interface (gh CLI does not support attachments).

gh issue create{repo_arg} \
     --title "$(cat "$(dirname "$0")/issue_title.txt")" \
     --body "$(cat "$(dirname "$0")/issue_body.md")"

echo ""
echo "Remember to manually attach compressed.zip as a comment/attachment!"
echo "GitHub CLI does not support file attachments - use the web interface."
"#
    );

    fs::write(&script_path, &script).unwrap();

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
    }

    println!("Report generated in {crash_dir}/:");
    println!("  issue_title.txt: {title}");
    println!("  issue_body.md:   ({} bytes)", body.len());
    println!("  create_issue.sh: ready to run");
    println!("\nTo create the issue:");
    println!("  bash {}/create_issue.sh", crash_dir);
}

/// Create reports for all unreported crashes
pub fn create_all_reports(results_dir: &str, project_filter: Option<&str>) {
    let path = Path::new(results_dir);
    if !path.exists() {
        println!("Results directory '{results_dir}' does not exist.");
        return;
    }

    let mut count = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            let dir_path = entry.path();
            let report_path = dir_path.join("to_report.txt");
            let script_path = dir_path.join("create_issue.sh");

            if !report_path.exists() || script_path.exists() {
                continue; // Skip if already has a report or no crash data
            }

            if let Some(filter) = project_filter {
                let metadata_path = dir_path.join("to_report_metadata.toml");
                if metadata_path.exists() {
                    let meta = fs::read_to_string(&metadata_path).unwrap_or_default();
                    let project = extract_toml_value(&meta, "project").unwrap_or_default();
                    if project != filter {
                        continue;
                    }
                }
            }

            let dir_str = dir_path.to_string_lossy().to_string();
            create_report(&dir_str, None, None, "cli");
            count += 1;
        }
    }

    info!("Generated {count} reports.");
}

fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with(key) {
            if let Some(val) = line.split('=').nth(1) {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}
