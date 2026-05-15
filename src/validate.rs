use std::fs;
use std::path::Path;
use std::process::Command;

use log::info;

use crate::ignore_list::IgnoreList;

/// Validate ignore_list.toml entries
pub fn validate_links(auto_remove: bool) {
    let mut ignore = IgnoreList::load_or_default();

    if ignore.ignore.is_empty() {
        println!("ignore_list.toml: no entries.");
        return;
    }

    println!("── ignore_list.toml ──");
    let mut to_remove: Vec<(String, String)> = Vec::new();

    for entry in &ignore.ignore {
        let url = &entry.issue_url;
        let state = get_issue_state(url);

        match state.as_deref() {
            Some("closed") => {
                println!("[CLOSED] {} \"{}\" — {}", entry.project, entry.pattern, url);
                if auto_remove {
                    to_remove.push((entry.project.clone(), entry.pattern.clone()));
                }
            }
            Some("open") => {
                println!("[OPEN]   {} \"{}\" — {}", entry.project, entry.pattern, url);
            }
            Some(other) => {
                println!("[{}] {} \"{}\" — {}", other.to_uppercase(), entry.project, entry.pattern, url);
            }
            None => {
                println!("[WARN]   {} \"{}\" — {} (could not check)", entry.project, entry.pattern, url);
            }
        }
    }

    if !to_remove.is_empty() {
        info!("Removing {} closed entries from ignore_list.toml", to_remove.len());
        for (project, pattern) in &to_remove {
            ignore.remove(project, pattern);
        }
        ignore.save();
        println!("Removed {} entries for closed issues.", to_remove.len());
    }
}

/// Validate ignored_item_N entries from crates/*/fuzz_settings_ci.toml
pub fn validate_config_ignored_items(auto_remove: bool) {
    let crates_dir = Path::new("crates");
    if !crates_dir.is_dir() {
        return;
    }

    let mut dirs: Vec<_> = fs::read_dir(crates_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    dirs.sort_by_key(|e| e.file_name());

    let mut any_printed = false;

    for entry in dirs {
        let config_path = entry.path().join("fuzz_settings_ci.toml");
        if !config_path.exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&config_path) else {
            continue;
        };

        let project_name = content
            .lines()
            .find(|l| l.trim().starts_with("name"))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());

        // Parse ignored_item lines that have a GitHub URL in the comment
        let mut items: Vec<(String, String, String)> = Vec::new(); // (key, pattern, url)
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || !trimmed.starts_with("ignored_item_") {
                continue;
            }
            // Extract URL from comment
            let Some(hash_pos) = trimmed.find(" # ").or_else(|| trimmed.find(" #")) else {
                continue;
            };
            let comment = trimmed[hash_pos + 2..].trim();
            if !comment.contains("github.com") {
                continue; // No GitHub URL, skip
            }
            // Extract URL (first http... token)
            let url = comment
                .split_whitespace()
                .find(|w| w.starts_with("http"))
                .unwrap_or("")
                .trim_end_matches(',');
            if url.is_empty() {
                continue;
            }

            let Some((key_part, rest)) = trimmed.split_once('=') else {
                continue;
            };
            let key = key_part.trim().to_string();
            let pattern = rest.split('#').next().unwrap_or("").trim().trim_matches('"').to_string();
            items.push((key, pattern, url.to_string()));
        }

        if items.is_empty() {
            continue;
        }

        if !any_printed {
            println!("\n── Fuzz config ignored items ──");
            any_printed = true;
        }
        println!("\n  {} ({}):", project_name, config_path.display());

        let mut lines_to_remove: Vec<String> = Vec::new();

        for (key, pattern, url) in &items {
            let state = get_issue_state(url);

            match state.as_deref() {
                Some("closed") => {
                    println!("    [CLOSED] {key} = \"{pattern}\" — {url}");
                    if auto_remove {
                        lines_to_remove.push(key.clone());
                    }
                }
                Some("open") => {
                    println!("    [OPEN]   {key} = \"{pattern}\" — {url}");
                }
                Some(other) => {
                    println!("    [{}] {key} = \"{pattern}\" — {url}", other.to_uppercase());
                }
                None => {
                    println!("    [WARN]   {key} = \"{pattern}\" — {url} (could not check)");
                }
            }
        }

        // Remove closed entries from config file
        if !lines_to_remove.is_empty() {
            let mut new_content = String::new();
            for line in content.lines() {
                let trimmed = line.trim();
                let should_remove = lines_to_remove.iter().any(|key| {
                    trimmed.starts_with(key) && trimmed[key.len()..].trim_start().starts_with('=')
                });
                if should_remove {
                    info!("Removing line from {}: {}", config_path.display(), trimmed);
                    continue;
                }
                new_content.push_str(line);
                new_content.push('\n');
            }
            fs::write(&config_path, new_content).unwrap();
            println!("    → Removed {} closed entries from {}", lines_to_remove.len(), config_path.display());
        }
    }
}

/// Get the state of a GitHub issue from its URL using `gh` CLI
fn get_issue_state(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();

    let issue_idx = parts.iter().position(|&p| p == "issues" || p == "pull")?;
    let number = parts.get(issue_idx + 1)?;
    let repo_idx = issue_idx.checked_sub(1)?;
    let owner_idx = repo_idx.checked_sub(1)?;
    let owner = parts.get(owner_idx)?;
    let repo = parts.get(repo_idx)?;

    let kind = parts[issue_idx];
    let api_path = if kind == "pull" {
        format!("repos/{owner}/{repo}/pulls/{number}")
    } else {
        format!("repos/{owner}/{repo}/issues/{number}")
    };

    let output = Command::new("gh")
        .args(["api", &api_path, "--jq", ".state"])
        .output()
        .ok()?;

    if output.status.success() {
        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if state.is_empty() { None } else { Some(state) }
    } else {
        None
    }
}
