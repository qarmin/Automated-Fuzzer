use std::process::Command;

use log::info;

use crate::ignore_list::IgnoreList;

pub fn validate_links(auto_remove: bool) {
    let mut ignore = IgnoreList::load_or_default();

    if ignore.ignore.is_empty() {
        println!("Ignore list is empty.");
        return;
    }

    let mut to_remove: Vec<(String, String)> = Vec::new();

    for entry in &ignore.ignore {
        let url = &entry.issue_url;
        let state = get_issue_state(url);

        match state.as_deref() {
            Some("CLOSED" | "closed" | "COMPLETED" | "completed") => {
                println!(
                    "[FIXED] {} \"{}\" - {} is CLOSED",
                    entry.project, entry.pattern, url
                );
                if auto_remove {
                    to_remove.push((entry.project.clone(), entry.pattern.clone()));
                }
            }
            Some("OPEN" | "open") => {
                println!(
                    "[OPEN]  {} \"{}\" - {} still open",
                    entry.project, entry.pattern, url
                );
            }
            Some(other) => {
                println!(
                    "[{}]  {} \"{}\" - {}",
                    other.to_uppercase(),
                    entry.project,
                    entry.pattern,
                    url
                );
            }
            None => {
                println!(
                    "[WARN]  {} \"{}\" - {} could not be checked (no gh access or invalid URL)",
                    entry.project, entry.pattern, url
                );
            }
        }
    }

    if !to_remove.is_empty() {
        info!("Auto-removing {} closed entries from ignore list", to_remove.len());
        for (project, pattern) in &to_remove {
            ignore.remove(project, pattern);
        }
        ignore.save();
        println!("\nRemoved {} entries for closed issues.", to_remove.len());
    }
}

/// Get the state of a GitHub issue from its URL using `gh` CLI
fn get_issue_state(url: &str) -> Option<String> {
    // Parse GitHub URL: https://github.com/OWNER/REPO/issues/NUMBER
    let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();

    // Find "issues" or "pull" in the URL
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
