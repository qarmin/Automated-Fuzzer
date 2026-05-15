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

    pub fn load_from(path: &str) -> Self {
        if Path::new(path).exists() {
            let content = fs::read_to_string(path).unwrap_or_default();
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
        if self.ignore.iter().any(|e| e.project == entry.project && e.pattern == entry.pattern) {
            println!("Entry already exists for project '{}' with pattern '{}'", entry.project, entry.pattern);
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
            println!("No ignored entries{}.", project_filter.map(|p| format!(" for project '{p}'")).unwrap_or_default());
            return;
        }

        for entry in entries {
            println!(
                "[{}] \"{}\" -> {} (added {})",
                entry.project, entry.pattern, entry.issue_url, entry.added_date
            );
        }
    }

    /// Check if a crash output matches any ignore pattern for a given project
    pub fn is_ignored(&self, project: &str, output: &str) -> bool {
        self.ignore
            .iter()
            .filter(|e| e.project == project)
            .any(|e| output.contains(&e.pattern))
    }

    /// Get matching ignore entries for a given crash output
    pub fn find_matching(&self, project: &str, output: &str) -> Vec<&IgnoreEntry> {
        self.ignore
            .iter()
            .filter(|e| e.project == project && output.contains(&e.pattern))
            .collect()
    }
}
