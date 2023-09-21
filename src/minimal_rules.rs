use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

use jwalk::WalkDir;
use rand::prelude::*;
use rayon::prelude::*;

use zip::write::FileOptions;
use zip::ZipWriter;

use crate::apps::ruff::calculate_ignored_rules;
use crate::clean_base_files::check_if_file_is_parsable_by_cpython;
use crate::common::collect_output;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

// THIS ONLY WORKS WITH RUFF

pub fn check_code(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    match settings.tool_type.as_str() {
        "check" => {
            find_minimal_rules(settings, obj);
        }
        "format" => {
            report_problem_with_format(settings, obj);
        }
        _ => {
            panic!("Unknown tool type: {}", settings.tool_type);
        }
    }
}

pub fn report_problem_with_format(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let temp_folder = settings.temp_folder.clone();
    let files_to_check = collect_broken_files_dir_files(settings);

    let collected_items: Vec<_> = files_to_check
        .into_par_iter()
        .filter_map(|i| {
            let file_name = i.split('/').last().unwrap();
            let new_name = format!("{temp_folder}/{file_name}");
            let original_content = fs::read_to_string(&i).unwrap();

            fs::write(&new_name, original_content).unwrap();

            let output = obj.get_run_command(&new_name).wait_with_output().unwrap();
            let all_str = collect_output(&output);
            if !obj.is_broken(&all_str) {
                println!("File {new_name} ({i}) is not broken");
                return None;
            }

            println!("File {new_name} ______________ ({i}) is broken",);
            Some((file_name.to_string(), i, all_str))
        })
        .collect();

    fs::remove_dir_all(&temp_folder).unwrap();
    fs::create_dir_all(&temp_folder).unwrap();

    save_results_to_file_format(settings, collected_items);
}

pub fn save_results_to_file_format(
    settings: &Setting,
    collected_items: Vec<(String, String, String)>,
) {
    for (file_name, name, output) in collected_items {
        let file_code = fs::read_to_string(&name).unwrap();
        let file_steam = file_name.split('.').next().unwrap();
        let folder = format!(
            "{}/FORMAT_({} bytes) - {}",
            settings.temp_folder,
            file_code.len(),
            file_steam,
        );
        let _ = fs::create_dir_all(&folder);
        let mut file_content = String::new();

        if output.contains("panicked") {
            file_content += "Format cause panic";
        } else {
            file_content += "Format cause problem";
        }

        file_content += "\n\n///////////////////////////////////////////////////////\n\n";
        file_content += &r###"Ruff 0.0.290 (latest changes from main branch)
```
ruff format *.py
```

file content(at least simple cpython script shows that this is valid python file):
```
$FILE_CONTENT
```

error
```
$ERROR
```


"###
        .replace("$FILE_CONTENT", &file_code)
        .replace("$ERROR", &output)
        .replace("\n\n```", "\n```");

        fs::write(format!("{folder}/to_report.txt"), &file_content).unwrap();

        fs::write(format!("{folder}/python_code.py"), &file_code).unwrap();

        let zip_filename = format!("{folder}/python_compressed.zip");
        zip_file(&zip_filename, &file_name, &file_code);
    }
}

pub fn zip_file(zip_filename: &str, file_name: &str, file_code: &str) {
    let zip_file = File::create(zip_filename).unwrap();
    let mut zip_writer = ZipWriter::new(zip_file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let _ = zip_writer.start_file(file_name, options);
    let _ = zip_writer.write_all(file_code.as_bytes());
}

pub fn find_minimal_rules(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let temp_folder = settings.temp_folder.clone();
    let files_to_check = collect_broken_files_dir_files(settings);

    let all_ruff_rules = collect_all_ruff_rules();
    let collected_rules: Vec<_> = files_to_check
        .into_par_iter()
        .filter_map(|i| {
            let file_name = i.split('/').last().unwrap();
            let new_name = format!("{temp_folder}/{file_name}");
            let original_content = fs::read_to_string(&i).unwrap();
            let mut out = String::new();

            fs::write(&new_name, &original_content).unwrap();

            if !check_if_file_is_parsable_by_cpython("", &new_name) {
                println!("File {new_name} ({i}) is not parsable");
                return None;
            }

            if !check_if_rule_file_crashing(&new_name, &all_ruff_rules, obj).0 {
                println!("File {new_name} ({i}) is not broken");
                return None;
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

                let (crashing, output) =
                    check_if_rule_file_crashing(&new_name, &rules_to_test, obj);
                if crashing {
                    valid_remove_rules = rules_to_test.clone();
                    rules_to_test.shuffle(&mut thread_rng());
                    rules_to_test.truncate(rules_to_test.len() / 2);
                    rules_to_test.sort();
                    out = output;
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
                let (crashing, output) =
                    check_if_rule_file_crashing(&new_name, &rules_to_test, obj);
                if crashing {
                    valid_remove_rules = rules_to_test.clone();
                    out = output;
                } else {
                    rules_to_test = valid_remove_rules.clone();
                }
            }
            println!(
                "For file {i} valid rules are: {}",
                valid_remove_rules.join(",")
            );

            Some((valid_remove_rules, file_name.to_string(), i, out))
        })
        .collect();

    fs::remove_dir_all(&temp_folder).unwrap();
    fs::create_dir_all(&temp_folder).unwrap();

    save_results_to_file(settings, collected_rules.clone());

    let mut btree_map: BTreeMap<String, u32> = BTreeMap::new();
    for (rules, _, _, _) in collected_rules {
        for j in rules {
            *btree_map.entry(j).or_insert(0) += 1;
        }
    }
    // Reorder items to have in vec most common used rules
    let mut items = Vec::new();
    for (k, v) in btree_map {
        items.push((k, v));
    }
    items.sort_by(|a, b| b.1.cmp(&a.1));
    println!("{items:?}");
}

pub fn save_results_to_file(
    settings: &Setting,
    rules_with_names: Vec<(Vec<String>, String, String, String)>,
) {
    for (rules, file_name, name, output) in rules_with_names {
        let file_code = fs::read_to_string(&name).unwrap();
        let file_steam = file_name.split('.').next().unwrap();
        let rule_str = rules.join("_");

        let mut file_content = String::new();
        let type_of_problem;
        if rules.len() == 1 {
            file_content += "Rule";
        } else {
            file_content += "Rules";
        }
        if output.contains("Failed to converge after") {
            file_content += &format!(" {} cause infinite loop", rules.join(", "));
            type_of_problem = "loop";
        } else if output.contains("panicked") {
            file_content += &format!(" {} cause panic", rules.join(", "));
            type_of_problem = "panic";
        } else {
            file_content += &format!(" {} cause autofix error", rules.join(", "));
            type_of_problem = "autofix";
        }

        let folder = format!(
            "{}/CHECK_{}_{}___({} bytes) - {}",
            settings.temp_folder,
            rule_str,
            type_of_problem,
            file_code.len(),
            file_steam,
        );
        let _ = fs::create_dir_all(&folder);

        file_content += "\n\n///////////////////////////////////////////////////////\n\n";
        file_content += &r###"Ruff 0.0.290 (latest changes from main branch)
```
ruff  *.py --select $RULES_TO_REPLACE --no-cache --fix --preview
```

file content(at least simple cpython script shows that this is valid python file):
```
$FILE_CONTENT
```

error
```
$ERROR
```


"###
        .replace("$RULES_TO_REPLACE", &rules.join(","))
        .replace("$FILE_CONTENT", &file_code)
        .replace("$ERROR", &output)
        .replace("\n\n```", "\n```");

        fs::write(format!("{folder}/to_report.txt"), &file_content).unwrap();

        fs::write(format!("{folder}/python_code.py"), &file_code).unwrap();

        let zip_filename = format!("{folder}/python_compressed.zip");
        zip_file(&zip_filename, &file_name, &file_code);
    }
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
) -> (bool, String) {
    assert!(!rules.is_empty());
    let mut command = Command::new("ruff");
    let ignored_rules = calculate_ignored_rules();
    let command = command
        .arg("check")
        .arg(test_file)
        .arg("--select")
        .arg(rules.join(","))
        .arg("--preview")
        .arg("--fix")
        .arg("--no-cache");
    if !ignored_rules.is_empty() {
        command.arg("--ignore").arg(&ignored_rules);
    }
    command.stderr(Stdio::piped()).stdout(Stdio::piped());
    let output = command.spawn().unwrap().wait_with_output().unwrap();
    let all_std = collect_output(&output);
    // Debug save results
    // dbg!(&all_std);
    // let mut file = OpenOptions::new()
    //     .write(true)
    //     .append(true)
    //     .create(true)
    //     .open("/home/rafal/test/rr/a.txt")
    //     .unwrap();
    // file.write(all_std.as_bytes()).unwrap();
    (obj.is_broken(&all_std), all_std)
}

fn collect_broken_files_dir_files(settings: &Setting) -> Vec<String> {
    WalkDir::new(&settings.broken_files_dir)
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
