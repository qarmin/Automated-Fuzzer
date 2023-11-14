use crate::common::collect_output;
use jwalk::WalkDir;
use log::info;
use rayon::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::minimal_rules::collect_broken_files_dir_files;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub fn verify_if_files_are_still_broken(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let used_rules = find_used_rules();
    info!("Found {} files to check", used_rules.len());
    let mut res = used_rules
        .into_par_iter()
        .map(|(full_name, rules, issue)| {
            let file_content = std::fs::read_to_string(&full_name).unwrap();
            let res;
            if check_if_rule_file_crashing(&full_name, &rules, obj).0 {
                res = format!(
                    "File {full_name}, from issue {issue} with rules {} is still broken",
                    rules.join(",")
                );
            } else {
                res = format!(
                    "[NOT BROKEN]File {full_name}, from issue {issue} with rules {} is not broken",
                    rules.join(",")
                );
            }
            // Save content to same file
            std::fs::write(&full_name, file_content).unwrap();
            res
        })
        .collect::<Vec<_>>();

    res.sort();

    for e in res {
        println!("{}", e);
    }
}

pub fn find_used_rules() -> Vec<(String, Vec<String>, String)> {
    let files = collect_files_to_validate();

    files
        .into_iter()
        .filter_map(|e| {
            let file_name = Path::new(&e).file_name()?.to_string_lossy().to_string();
            let issue_splits = file_name.split("__").collect::<Vec<_>>();
            let rules = (*issue_splits.first()?).to_string();
            let issues = (*issue_splits.get(1)?).to_string();
            let rules = rules.split('_').map(str::to_string).collect::<Vec<_>>();
            Some((e, rules, issues))
        })
        .collect::<Vec<_>>()
}
fn check_if_rule_file_crashing(test_file: &str, rules: &[String], obj: &Box<dyn ProgramConfig>) -> (bool, String) {
    assert!(!rules.is_empty());
    let mut command = Command::new("ruff");
    let command = command
        .arg("check")
        .arg(test_file)
        .arg("--select")
        .arg(rules.join(","))
        .arg("--preview")
        .arg("--fix")
        .arg("--unsafe-fixes")
        .arg("--isolated")
        .arg("--no-cache");
    command.stderr(Stdio::piped()).stdout(Stdio::piped());
    let output = command.spawn().unwrap().wait_with_output().unwrap();
    let all_std = collect_output(&output);
    // dbg!(&all_std);
    (obj.is_broken(&all_std), all_std)
}

pub fn collect_files_to_validate() -> Vec<String> {
    WalkDir::new("broken_files")
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if entry.file_type().is_file() && entry.path().to_string_lossy().ends_with(".py") {
                return Some(entry.path().to_string_lossy().to_string());
            }
            None
        })
        .collect()
}
