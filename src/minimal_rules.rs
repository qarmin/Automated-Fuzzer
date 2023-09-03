use crate::apps::ruff::calculate_ignored_rules;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use jwalk::WalkDir;
use rand::prelude::*;
use rayon::prelude::*;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn find_minimal_rules(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let temp_folder = settings.temp_folder.clone();
    let files_to_check = collect_output_dir_files(settings);

    let all_ruff_rules = collect_all_ruff_rules();

    // Execute
    let atomic_counter = AtomicUsize::new(0);
    let all_files = files_to_check.len();

    files_to_check.into_par_iter().for_each(|i| {
        let idx = atomic_counter.fetch_add(1, Ordering::Relaxed);
        if idx % 10 == 0 {
            println!("Checking file {idx} of {all_files}");
        }

        let file_name = i.split('/').last().unwrap();
        let new_name = format!("{temp_folder}/{file_name}");
        let original_content = fs::read_to_string(&i).unwrap();

        fs::write(&new_name, &original_content).unwrap();

        if !check_if_rule_file_crashing(&new_name, &all_ruff_rules, obj) {
            println!("File {new_name} ({i}) is not broken");
            return;
        }

        let mut valid_remove_rules = all_ruff_rules.clone();
        let mut rules_to_test = all_ruff_rules.clone();

        let mut to_idx = 100;
        while to_idx != 0 {
            fs::write(&new_name, &original_content).unwrap();
            // println!("TO_IDX - {to_idx}");
            to_idx -= 1;
            // Almost sure that  this will not crash with more than 4 rules
            if valid_remove_rules.len() <= 4 {
                break;
            }

            let crashing = check_if_rule_file_crashing(&new_name, &rules_to_test, obj);
            if crashing {
                valid_remove_rules = rules_to_test.clone();
                rules_to_test.shuffle(&mut thread_rng());
                rules_to_test.truncate(rules_to_test.len() / 2);
                rules_to_test.sort();
            } else {
                rules_to_test = valid_remove_rules.clone();
            }
        }

        rules_to_test = valid_remove_rules.clone();
        let mut curr_idx = valid_remove_rules.len();
        while curr_idx != 0 {
            fs::write(&new_name, &original_content).unwrap();
            if valid_remove_rules.len() <= 1 {
                break;
            }
            // println!("CURR_IDX - {curr_idx}");
            curr_idx -= 1;
            rules_to_test.remove(curr_idx);
            let crashing = check_if_rule_file_crashing(&new_name, &rules_to_test, obj);
            if crashing {
                valid_remove_rules = rules_to_test.clone();
            } else {
                rules_to_test = valid_remove_rules.clone();
            }
        }
        println!(
            "For file {i} valid rules are: {}  - {new_name}",
            valid_remove_rules.join(",")
        );
    });
}

pub fn collect_all_ruff_rules() -> Vec<String> {
    let stdout: Vec<_> = Command::new("ruff")
        .arg("rule")
        .arg("--all")
        .output()
        .unwrap()
        .stdout;
    let stdout_str = String::from_utf8(stdout).unwrap();
    let lines: Vec<_> = stdout_str
        .split('\n')
        .filter(|e| e.starts_with("# ") && e.ends_with(')'))
        .map(ToString::to_string)
        .collect();
    let mut rules = Vec::new();
    for line in lines {
        if let Some(start_idx) = line.find('(') {
            if let Some(end_idx) = line.find(')') {
                let rule = &line[start_idx + 1..end_idx];
                if rule.to_uppercase() != rule {
                    continue;
                }
                rules.push(rule.to_string());
            }
        }
    }
    rules.sort();
    rules
}

fn check_if_rule_file_crashing(
    test_file: &str,
    rules: &[String],
    obj: &Box<dyn ProgramConfig>,
) -> bool {
    assert!(!rules.is_empty());
    let mut command = Command::new("ruff");
    let ignored_rules = calculate_ignored_rules();
    let command = command
        .arg("check")
        .arg(test_file)
        .arg("--select")
        .arg(rules.join(","))
        .arg("--fix")
        .arg("--no-cache");
    if !ignored_rules.is_empty() {
        command.arg("--ignore").arg(&ignored_rules);
    }
    command.stderr(Stdio::piped()).stdout(Stdio::piped());
    let output = command.spawn().unwrap().wait_with_output().unwrap();
    let stdout: Vec<_> = output.stdout;
    let stderr: Vec<_> = output.stderr;
    let stdout_str = String::from_utf8(stdout).unwrap();
    let stderr_str = String::from_utf8(stderr).unwrap();
    let all_std = format!("{stdout_str}{stderr_str}");

    obj.is_broken(&all_std)
}

fn collect_output_dir_files(settings: &Setting) -> Vec<String> {
    WalkDir::new(&settings.output_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if entry.file_type().is_file() {
                return Some(entry.path().to_string_lossy().to_string());
            }
            None
        })
        .collect()
}
