use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use jwalk::WalkDir;
use log::info;
use rand::prelude::*;
use rand::{random, rng};
use rayon::prelude::*;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::apps::ruff::calculate_ignored_rules;
use crate::common::{check_if_app_ends, collect_output, remove_and_create_entire_folder};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

// THIS ONLY WORKS WITH RUFF

pub const MAX_FILE_SIZE: u64 = 1000; // Bytes
pub const USE_MAX_FILE_SIZE: bool = false;

pub fn check_code(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let tool_type = settings.non_custom_items.as_ref().unwrap().tool_type.clone();
    match tool_type.as_str() {
        "lint_check" | "lint_check_fix" => {
            find_minimal_rules(settings, obj);
        }
        "format" => {
            report_problem_with_format(settings, obj);
        }
        "ty" => {
            // Nothing to do
            // there is
        }
        _ => {
            panic!("Unknown tool type: {tool_type}");
        }
    }
}

pub fn report_problem_with_format(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let files_to_check = collect_broken_files_dir_files(settings);

    let ruff_version = obj.get_version();

    let collected_items: Vec<_> = files_to_check
        .into_par_iter()
        .filter_map(|i| {
            let file_name = i.split('/').next_back().unwrap();
            let new_name = format!("{}/{file_name}", settings.temp_folder);
            let original_content = fs::read_to_string(&i).unwrap();

            fs::write(&new_name, original_content).unwrap();

            let output = obj.run_command(&new_name).wait_with_output().unwrap();
            let all_str = collect_output(&output);
            if !obj.is_broken(&all_str) {
                info!("File {new_name} ({i}) is not broken");
                return None;
            }

            info!("File {new_name} ______________ ({i}) is broken",);
            Some((file_name.to_string(), i, all_str))
        })
        .collect();

    remove_and_create_entire_folder(&settings.temp_folder);

    save_results_to_file_format(settings, collected_items, &ruff_version);
}

pub fn save_results_to_file_format(
    settings: &Setting,
    collected_items: Vec<(String, String, String)>,
    ruff_version: &str,
) {
    for (file_name, name, output) in collected_items {
        let file_code = fs::read_to_string(&name).unwrap();
        let file_steam = file_name.split('.').next().unwrap();
        let folder = format!(
            "{}/FORMAT_({} bytes) - {} __ {}",
            settings.temp_folder,
            file_code.len(),
            file_steam,
            random::<u64>()
        );
        let _ = fs::create_dir_all(&folder);
        let mut file_content = String::new();

        if output.contains("panicked") {
            file_content += "Panic when formatting file";
        } else {
            file_content += "Formatter cause problem";
        }
        file_content += "\n\n///////////////////////////////////////////////////////\n\n";
        file_content += &r"Ruff $RUFF_VERSION
```
ruff format *.py
```

file content(at the bottom should be attached raw, not formatted file - github removes some non-printable characters, so copying from here may not work properly):
```
$FILE_CONTENT
```

error
```
$ERROR
```

"
            .replace("$FILE_CONTENT", &file_code)
            .replace("$ERROR", &output)
            .replace("$RUFF_VERSION", ruff_version)
            .replace("\n\n```", "\n```");

        fs::write(format!("{folder}/to_report.txt"), &file_content).unwrap();

        fs::write(format!("{folder}/python_code.py"), &file_code).unwrap();

        let zip_filename = format!("{folder}/raw_file.zip");
        zip_file(&zip_filename, &file_name, file_code.as_bytes());
    }
}

pub fn zip_file(zip_filename: &str, file_name: &str, file_code: &[u8]) {
    let zip_file = File::create(zip_filename).unwrap();
    let mut zip_writer = ZipWriter::new(zip_file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let _ = zip_writer.start_file(file_name, options);
    let _ = zip_writer.write_all(file_code);
}

pub fn find_minimal_rules(settings: &Setting, obj: &Box<dyn ProgramConfig>) {
    let temp_folder = settings.temp_folder.clone();

    // Clean temp folder
    remove_and_create_entire_folder(&settings.temp_folder);

    obj.remove_non_parsable_files(&settings.broken_files_dir);
    let files_to_check = collect_broken_files_dir_files(settings);
    let old_files_number = files_to_check.len();

    let files_to_check = if USE_MAX_FILE_SIZE {
        files_to_check
            .into_iter()
            .filter(|e| Path::new(&e).metadata().map(|e| e.len()).unwrap_or(0) <= MAX_FILE_SIZE)
            .collect::<Vec<_>>()
    } else {
        files_to_check
    };
    if USE_MAX_FILE_SIZE {
        info!(
            "Using only {} files from {} files, that are smaller than {} bytes",
            files_to_check.len(),
            old_files_number,
            MAX_FILE_SIZE
        );
    } else {
        info!(
            "Using all {} files from {} files",
            files_to_check.len(),
            old_files_number
        );
    }

    let all_ruff_rules = collect_all_ruff_rules();
    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all = files_to_check.len();
    let all_rules_to_find = all_ruff_rules
        .iter()
        .map(|e| (e, vec![format!(" {e},"), format!(" {e}:")]))
        .collect::<Vec<_>>();
    let collected_rules: Vec<_> = files_to_check
        .into_par_iter()
        .filter_map(|i| {
            if check_if_app_ends() {
                return None;
            }

            let idx = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if idx % 100 == 0 {
                info!("_____ Processed already {idx} / {all}");
            }
            let file_name = i.split('/').next_back().unwrap();
            let new_name = random::<u64>().to_string();
            let new_name = format!("{temp_folder}/{new_name}.py");
            let original_content = fs::read(&i).unwrap();
            let mut out;

            fs::write(&new_name, &original_content).unwrap();
            let (happens_with_fix, fix_output) =
                check_if_rule_file_crashing(&new_name, &all_ruff_rules, obj, false, settings);
            // println!("{_output}");
            fs::write(&new_name, &original_content).unwrap();
            let (happens_with_check, check_output) =
                check_if_rule_file_crashing(&new_name, &all_ruff_rules, obj, true, settings);
            // println!("{_output}");
            fs::write(&new_name, &original_content).unwrap();

            if !happens_with_fix && !happens_with_check {
                info!("File {new_name} ({i}) is not broken");
                return Some(None);
            }
            // println!("Happens with fix {happens_with_fix} - happens with check {happens_with_check} ({new_name} - {i})");

            if happens_with_fix {
                out = fix_output;
            } else {
                out = check_output;
            }

            let only_check = happens_with_check;

            let mut valid_remove_rules = all_ruff_rules.clone();

            let out_res = check_rules_by_existing_in_output(
                &mut valid_remove_rules, &original_content, &new_name, obj, only_check, settings, &mut out,
                &all_rules_to_find,
            );

            let mut to_idx = 100;
            if to_idx > 6 {
                check_rules_by_dividing(
                    &mut to_idx, &mut valid_remove_rules, &original_content, &new_name, obj, only_check, settings,
                    &mut out,
                );
            }

            let mut rules_check = 0;
            check_rules_one_by_one(
                &mut valid_remove_rules, &original_content, &new_name, obj, only_check, settings, &mut out,
                &mut rules_check,
            );
            info!(
                "For file {i} ({out_res:?} initial check, {} group checks + {rules_check} rules checks) valid rules are: {}",
                100 - to_idx,
                valid_remove_rules.join(",")
            );

            fs::write(&new_name, &original_content).unwrap();

            Some(Some((valid_remove_rules, file_name.to_string(), i, out, only_check)))
        })
        .while_some()
        .collect();

    remove_and_create_entire_folder(&settings.temp_folder);

    let ruff_version = obj.get_version();
    save_results_to_file(settings, collected_rules.clone(), &ruff_version);

    let mut btree_map: BTreeMap<String, u32> = BTreeMap::new();
    for (rules, _, _, _, _) in collected_rules {
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

    if !items.is_empty() {
        draw_table(items, &temp_folder).unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
fn check_rules_by_existing_in_output(
    valid_remove_rules: &mut Vec<String>,
    original_content: &[u8],
    new_name: &str,
    obj: &Box<dyn ProgramConfig>,
    only_check: bool,
    settings: &Setting,
    out: &mut String,
    all_rules_to_find: &[(&String, Vec<String>)],
) -> Option<(i32, i32)> {
    let start_rules_len = valid_remove_rules.len();

    let line_with_rule_codes = out.lines().find(|e| e.contains("with rule codes"));
    let rules_in_output = if let Some(line_with_rule_codes) = line_with_rule_codes {
        all_rules_to_find
            .iter()
            .filter(|(_a, b)| b.iter().any(|c| line_with_rule_codes.contains(c)))
            .map(|(a, _)| *a)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if rules_in_output.is_empty() {
        return None;
    }

    let (crashing, output) = check_if_rule_file_crashing(new_name, &rules_in_output, obj, only_check, settings);
    fs::write(new_name, original_content).unwrap();

    if crashing {
        valid_remove_rules.clone_from(&rules_in_output);
        *out = output;
        Some((rules_in_output.len() as i32, start_rules_len as i32))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn check_rules_by_dividing(
    to_idx: &mut i32,
    valid_remove_rules: &mut Vec<String>,
    original_content: &[u8],
    new_name: &str,
    obj: &Box<dyn ProgramConfig>,
    only_check: bool,
    settings: &Setting,
    out: &mut String,
) {
    while *to_idx != 0 {
        fs::write(new_name, original_content).unwrap();
        // info!("TO_IDX - {to_idx}");
        *to_idx -= 1;
        // Using bigger number, will add const additional checking time, but may decrease number of iterations
        if valid_remove_rules.len() <= 6 {
            break;
        }

        let mut rules_to_test = valid_remove_rules.clone();
        rules_to_test.shuffle(&mut rng());
        rules_to_test.truncate(rules_to_test.len() / 2);
        rules_to_test.sort();

        let (crashing, output) = check_if_rule_file_crashing(new_name, &rules_to_test, obj, only_check, settings);
        // println!("CRASHHHHHH - {} ({only_check}) - {}", crashing, output);
        if crashing {
            valid_remove_rules.clone_from(&rules_to_test);
            *out = output;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_rules_one_by_one(
    valid_remove_rules: &mut Vec<String>,
    original_content: &[u8],
    new_name: &str,
    obj: &Box<dyn ProgramConfig>,
    only_check: bool,
    settings: &Setting,
    out: &mut String,
    rules_check: &mut i32,
) {
    let mut rules_to_test = valid_remove_rules.clone();
    let mut curr_idx = valid_remove_rules.len();
    while curr_idx != 0 {
        *rules_check += 1;
        fs::write(new_name, original_content).unwrap();
        if valid_remove_rules.len() <= 1 {
            break;
        }
        // info!("CURR_IDX - {curr_idx}");
        curr_idx -= 1;
        rules_to_test.remove(curr_idx);
        let (crashing, output) = check_if_rule_file_crashing(new_name, &rules_to_test, obj, only_check, settings);
        if crashing {
            valid_remove_rules.clone_from(&rules_to_test);
            *out = output;
        } else {
            rules_to_test = valid_remove_rules.clone();
        }
    }
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

    writeln!(file, "{str_buf}")?;
    println!("{str_buf}");

    Ok(())
}

pub fn save_results_to_file(
    settings: &Setting,
    rules_with_names: Vec<(Vec<String>, String, String, String, bool)>,
    ruff_version: &str,
) {
    for (rules, file_name, name, output, only_check) in rules_with_names {
        let file_code = fs::read_to_string(&name).unwrap();
        // Max 10 rules
        let rule_str = rules.iter().take(10).clone().cloned().collect::<Vec<_>>().join("_");

        let mut file_content = String::new();
        let type_of_problem;
        if only_check {
            file_content += "Checking file with rule";
        } else {
            file_content += "Fixing file with rule";
        }
        if rules.len() > 1 {
            file_content += "s";
        }

        let joined_rules = rules.join(", ");
        if output.contains("Failed to converge after") {
            file_content += &format!(" {joined_rules} cause infinite loop");
            type_of_problem = "loop";
        } else if output.contains("panicked") {
            file_content += &format!(" {joined_rules} cause panic");
            type_of_problem = "panic";
        } else {
            file_content += &format!(" {joined_rules} cause autofix error");
            type_of_problem = "autofix";
        }

        let start_text = if only_check { "CHECK" } else { "FIX" };

        let folder = format!(
            "{}/{start_text}_{rule_str}_{type_of_problem}___({} bytes) - {}",
            settings.temp_folder,
            file_code.len(),
            random::<u64>()
        );
        let _ = fs::create_dir_all(&folder);
        let output = output
            .lines()
            .filter(|e| !e.contains("has been remapped to"))
            .collect::<Vec<_>>()
            .join("\n");

        let fix_needed = if only_check { "" } else { "--fix --unsafe-fixes" };

        file_content += "\n\n///////////////////////////////////////////////////////\n\n";
        file_content += &r"$RUFF_VERSION
```
ruff check *.py --select $RULES_TO_REPLACE --no-cache $RUFF_FIX --preview --output-format concise --isolated
```

file content(at the bottom should be attached raw, not formatted file - github removes some non-printable characters, so copying from here may not work):
```
$FILE_CONTENT
```

error
```
$ERROR
```

Ruff build, that was used to reproduce problem(compiled on Ubuntu 22.04 with relase mode + debug symbols + debug assertions + overflow checks) - https://github.com/qarmin/Automated-Fuzzer/releases/download/Nightly/ruff.7z
".replace("  ", " ")
            .replace("$RUFF_FIX", fix_needed)
            .replace("$RULES_TO_REPLACE", &rules.join(","))
            .replace("$FILE_CONTENT", &file_code)
            .replace("$RUFF_VERSION", ruff_version)
            .replace("$ERROR", &output)
            .replace("\n\n```", "\n```");

        fs::write(format!("{folder}/to_report.txt"), &file_content).unwrap();

        fs::write(format!("{folder}/python_code.py"), &file_code).unwrap();

        let zip_filename = format!("{folder}/python_compressed.zip");
        zip_file(&zip_filename, &file_name, file_code.as_bytes());
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
                || e.starts_with("## Deprecated")
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
        if line.starts_with("## Removal")
            || line.starts_with("## Removed")
            || line.starts_with("## Deprecation")
            || line.starts_with("## Deprecated")
        {
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
            .arg("--no-cache")
    };
    let app_config = &settings.non_custom_items.as_ref().unwrap().app_config;
    if !app_config.is_empty() {
        command.arg("--config").arg(app_config);
    } else {
        command.arg("--isolated");
    }

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
    // dbg!(&all_std, &rules, obj.is_broken(&all_std));
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
