use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use jwalk::WalkDir;
use log::info;
use rand::prelude::*;
use rayon::prelude::*;

use zip::write::FileOptions;
use zip::ZipWriter;

use crate::apps::ruff::calculate_ignored_rules;
use crate::common::collect_output;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

// THIS ONLY WORKS WITH RUFF

pub const MAX_FILE_SIZE: u64 = 500; // Bytes

pub fn check_code(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    match settings.tool_type.as_str() {
        "lint_check" | "lint_check_fix" => {
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

    let ruff_version = obj.get_version();

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
                info!("File {new_name} ({i}) is not broken");
                return None;
            }

            info!("File {new_name} ______________ ({i}) is broken",);
            Some((file_name.to_string(), i, all_str))
        })
        .collect();

    fs::remove_dir_all(&temp_folder).unwrap();
    fs::create_dir_all(&temp_folder).unwrap();

    save_results_to_file_format(settings, collected_items, ruff_version);
}

pub fn save_results_to_file_format(
    settings: &Setting,
    collected_items: Vec<(String, String, String)>,
    ruff_version: String,
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
            file_content += "Panic when formatting file";
        } else {
            file_content += "Formatter cause problem";
        }

        file_content += "\n\n///////////////////////////////////////////////////////\n\n";
        file_content += &r###"Ruff $RUFF_VERSION (latest changes from main branch)
```
ruff format *.py
```

file content:
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
        .replace("$RUFF_VERSION", &ruff_version)
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

    obj.remove_non_parsable_files(&settings.broken_files_dir);
    let files_to_check = collect_broken_files_dir_files(settings);
    // files_to_check.truncate(100);

    let files_to_check_new: Vec<_> = files_to_check
        .iter()
        .cloned()
        .filter(|e| Path::new(&e).metadata().ok().map(|e| e.len()).unwrap_or(u64::MAX) < MAX_FILE_SIZE)
        .collect();

    info!(
        "Using only {} files from {} files, that are smaller than {} bytes",
        files_to_check_new.len(),
        files_to_check.len(),
        MAX_FILE_SIZE
    );
    let files_to_check = files_to_check_new;

    let all_ruff_rules = collect_all_ruff_rules();
    let only_check = settings.tool_type == "lint_check";
    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all = files_to_check.len();
    let collected_rules: Vec<_> = files_to_check
        .into_par_iter()
        .filter_map(|i| {
            let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if idx % 100 == 0 {
                info!("_____ Processsed already {idx} / {all}");
            }
            let file_name = i.split('/').last().unwrap();
            let new_name = format!("{temp_folder}/{file_name}");
            let original_content = fs::read_to_string(&i).unwrap();
            let mut out = String::new();

            fs::write(&new_name, &original_content).unwrap();

            if !check_if_rule_file_crashing(&new_name, &all_ruff_rules, obj, only_check, settings).0 {
                info!("File {new_name} ({i}) is not broken");
                return None;
            }

            let mut valid_remove_rules = all_ruff_rules.clone();
            let mut rules_to_test = all_ruff_rules.clone();

            let mut to_idx = 100;
            while to_idx != 0 {
                fs::write(&new_name, &original_content).unwrap();
                // info!("TO_IDX - {to_idx}");
                to_idx -= 1;
                // Almost sure that  this will not crash with more than 4 rules
                if valid_remove_rules.len() <= 4 {
                    break;
                }

                let (crashing, output) =
                    check_if_rule_file_crashing(&new_name, &rules_to_test, obj, only_check, settings);
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
                // info!("CURR_IDX - {curr_idx}");
                curr_idx -= 1;
                rules_to_test.remove(curr_idx);
                let (crashing, output) =
                    check_if_rule_file_crashing(&new_name, &rules_to_test, obj, only_check, settings);
                if crashing {
                    valid_remove_rules = rules_to_test.clone();
                    out = output;
                } else {
                    rules_to_test = valid_remove_rules.clone();
                }
            }
            info!("For file {i} valid rules are: {}", valid_remove_rules.join(","));

            Some((valid_remove_rules, file_name.to_string(), i, out))
        })
        .collect();

    fs::remove_dir_all(&temp_folder).unwrap();
    fs::create_dir_all(&temp_folder).unwrap();

    let ruff_version = obj.get_version();
    save_results_to_file(settings, collected_rules.clone(), ruff_version);

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
    info!("{items:?}");

    draw_table(items, &temp_folder).unwrap();
}

pub fn draw_table(items: Vec<(String, u32)>, temp_folder: &str) -> Result<(), std::io::Error> {
    let file_name = format!("{temp_folder}/table.txt");
    let mut file = File::create(file_name)?;

    let mut str_buf = String::new();
    str_buf += "+--------------+----------------+\n";
    str_buf += "| Rule         | Number         |\n";
    str_buf += "+--------------+----------------+\n";

    for (rule, number) in items {
        str_buf += &format!("| {rule:<12} | {number:<14} |\n");
    }

    str_buf += "+--------------+----------------+\n";

    writeln!(file, "{}", str_buf)?;
    println!("{}", str_buf);

    Ok(())
}

pub fn save_results_to_file(
    settings: &Setting,
    rules_with_names: Vec<(Vec<String>, String, String, String)>,
    ruff_version: String,
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
        let output = output
            .lines()
            .filter(|e| !e.contains("has been remapped to"))
            .collect::<Vec<_>>()
            .join("\n");

        file_content += "\n\n///////////////////////////////////////////////////////\n\n";
        file_content += &r###"$RUFF_VERSION (latest changes from main branch)
```
ruff  *.py --select $RULES_TO_REPLACE --no-cache --fix --unsafe-fixes --preview --output-format concise --isolated
```

file content:
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
        .replace("$RUFF_VERSION", &ruff_version)
        .replace("$ERROR", &output)
        .replace("\n\n```", "\n```");

        fs::write(format!("{folder}/to_report.txt"), &file_content).unwrap();

        fs::write(format!("{folder}/python_code.py"), &file_code).unwrap();

        let zip_filename = format!("{folder}/python_compressed.zip");
        zip_file(&zip_filename, &file_name, &file_code);
    }
}

pub fn collect_all_ruff_rules() -> Vec<String> {
    let stdout: Vec<_> = Command::new("ruff").arg("rule").arg("--all").output().unwrap().stdout;
    let stdout_str = String::from_utf8(stdout).unwrap();
    let lines: Vec<_> = stdout_str
        .split('\n')
        .filter(|e| {
            e.starts_with("## Removal")
                || e.starts_with("## Removed")
                || e.starts_with("## Deprecation")
                || (e.starts_with("# ") && e.ends_with(')'))
        })
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
        if line.starts_with("## Removal") || line.starts_with("## Removed") || line.starts_with("## Deprecation") {
            rules.pop();
        }
    }
    rules.sort();
    rules
}

fn check_if_rule_file_crashing(
    test_file: &str,
    rules: &[String],
    obj: &Box<dyn ProgramConfig>,
    only_check: bool,
    settings: &Setting,
) -> (bool, String) {
    assert!(!rules.is_empty());
    let mut command = Command::new("ruff");
    let ignored_rules = calculate_ignored_rules();
    let command = if only_check {
        command
            .arg("check")
            .arg(test_file)
            .arg("--select")
            .arg(rules.join(","))
            .arg("--preview")
            .arg("--output-format")
            .arg("concise")
            .arg("--no-cache")
    } else {
        command
            .arg("check")
            .arg(test_file)
            .arg("--select")
            .arg(rules.join(","))
            .arg("--preview")
            .arg("--output-format")
            .arg("concise")
            .arg("--fix")
            .arg("--unsafe-fixes")
            .arg("--isolated")
            .arg("--no-cache")
    };
    if !ignored_rules.is_empty() {
        command.arg("--ignore").arg(&ignored_rules);
    }
    command.stderr(Stdio::piped()).stdout(Stdio::piped());
    let output = command.spawn().unwrap().wait_with_output().unwrap();
    let all_std = collect_output(&output);
    if settings.debug_print_results {
        info!("{all_std}");
    }
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

pub fn collect_broken_files_dir_files(settings: &Setting) -> Vec<String> {
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
