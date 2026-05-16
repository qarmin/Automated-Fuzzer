use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const IGNORE_LIST_PATH: &str = "ignore_list.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreEntry {
    pub project: String,
    pub pattern: String,
    pub issue_url: String,
    pub added_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IgnoreList {
    #[serde(default)]
    pub ignore: Vec<IgnoreEntry>,
}

impl IgnoreList {
    pub fn load_or_default() -> Self {
        if Path::new(IGNORE_LIST_PATH).exists() {
            let content = fs::read_to_string(IGNORE_LIST_PATH).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        self.save_to(IGNORE_LIST_PATH);
    }

    pub fn save_to(&self, path: &str) {
        let content = toml::to_string_pretty(self).unwrap();
        fs::write(path, content).unwrap();
    }

    pub fn add(&mut self, entry: IgnoreEntry) {
        // Check for duplicate
        if self
            .ignore
            .iter()
            .any(|e| e.project == entry.project && e.pattern == entry.pattern)
        {
            println!(
                "Entry already exists for project '{}' with pattern '{}'",
                entry.project, entry.pattern
            );
            return;
        }
        self.ignore.push(entry);
    }

    pub fn remove(&mut self, project: &str, pattern: &str) -> bool {
        let before = self.ignore.len();
        self.ignore.retain(|e| !(e.project == project && e.pattern == pattern));
        self.ignore.len() < before
    }

    pub fn print_list(&self, project_filter: Option<&str>) {
        let entries: Vec<_> = self
            .ignore
            .iter()
            .filter(|e| project_filter.is_none() || project_filter == Some(e.project.as_str()))
            .collect();

        if entries.is_empty() {
            println!(
                "No ignored entries{}.",
                project_filter
                    .map(|p| format!(" for project '{p}'"))
                    .unwrap_or_default()
            );
            return;
        }

        for entry in entries {
            println!(
                "[{}] \"{}\" -> {} (added {})",
                entry.project, entry.pattern, entry.issue_url, entry.added_date
            );
        }
    }
}

/// Print ignored_item_N entries from all crates/*/fuzz_settings_ci.toml files.
pub fn print_config_ignored_items(project_filter: Option<&str>) {
    let crates_dir = Path::new("crates");
    if !crates_dir.is_dir() {
        return;
    }

    let mut found_any = false;

    let mut dirs: Vec<_> = fs::read_dir(crates_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    dirs.sort_by_key(|e| e.file_name());

    for entry in dirs {
        let config_path = entry.path().join("fuzz_settings_ci.toml");
        if !config_path.exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&config_path) else {
            continue;
        };

        // Extract project name from config
        let project_name = content
            .lines()
            .find(|l| l.trim().starts_with("name"))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());

        if let Some(filter) = project_filter {
            if project_name != filter {
                continue;
            }
        }

        // Collect ignored_item_N entries
        let mut items: Vec<(String, String)> = Vec::new(); // (pattern, comment)
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if !trimmed.starts_with("ignored_item_") {
                continue;
            }
            let Some((key_part, rest)) = trimmed.split_once('=') else {
                continue;
            };
            let _ = key_part; // ignored_item_N

            // Split value from comment: "pattern" # url
            let rest = rest.trim();
            let (value, comment) = if let Some(hash_pos) = rest.find(" #") {
                (&rest[..hash_pos], rest[hash_pos + 2..].trim())
            } else {
                (rest, "")
            };
            let pattern = value.trim().trim_matches('"').to_string();
            if !pattern.is_empty() {
                items.push((pattern, comment.to_string()));
            }
        }

        if items.is_empty() {
            continue;
        }

        if !found_any {
            println!("\n Ignored patterns from fuzz configs ");
            found_any = true;
        }
        println!("\n  {} ({}):", project_name, config_path.display());
        for (pattern, comment) in &items {
            if comment.is_empty() {
                println!("    \"{}\"", pattern);
            } else {
                println!("    \"{}\"  {}", pattern, comment);
            }
        }
    }

    if !found_any {
        if let Some(filter) = project_filter {
            println!("\nNo ignored patterns in fuzz configs for '{filter}'.");
        }
    }
}
