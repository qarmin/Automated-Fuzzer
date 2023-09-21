use log::{error, info};
use std::cmp::max;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::prelude::ExitStatusExt;
use std::path::Path;
use std::process::Output;

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::obj::ProgramConfig;
use crate::settings::{Setting, TIMEOUT_MESSAGE};

pub const STRING_MINIMIZATION_LIMIT: usize = 3;

pub fn create_new_file_name(setting: &Setting, old_name: &str) -> String {
    loop {
        let pat = Path::new(&old_name);
        let extension = pat.extension().unwrap().to_str().unwrap().to_string();
        let file_name = pat.file_stem().unwrap().to_str().unwrap().to_string();
        let new_name = format!(
            "{}/{file_name}{}.{extension}",
            setting.broken_files_dir,
            rand::thread_rng().gen_range(1..10000)
        );
        if !Path::new(&new_name).exists() {
            return new_name;
        }
    }
}

pub fn collect_output(output: &Output) -> String {
    let stdout = &output.stdout;
    let stderr = &output.stderr;
    let stdout_str = String::from_utf8_lossy(stdout);
    let stderr_str = String::from_utf8_lossy(stderr);
    format!("{stdout_str}\n{stderr_str}")
}

pub fn try_to_save_file(setting: &Setting, full_name: &str, new_name: &str) -> bool {
    if setting.copy_broken_files {
        if let Err(e) = fs::copy(full_name, new_name) {
            error!("Failed to copy file {full_name}, reason {e}, (maybe broken files folder not exists?)");
            return true;
        };
        return true;
    }
    false
}

#[allow(clippy::borrowed_box)]
pub fn minimize_string_output(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let Ok(data) = fs::read_to_string(full_name) else {
        info!("INFO: Cannot read content of {full_name}, probably because is not valid UTF-8");
        return;
    };

    let (is_really_broken, output) = execute_command_and_connect_output(obj, full_name);
    assert!(
        (is_really_broken || obj.is_broken(&output)),
        "At start should be broken!"
    );

    let mut lines = data.lines().map(str::to_string).collect::<Vec<String>>();
    let mut rng = rand::thread_rng();

    let old_line_number = lines.len();

    let mut attempts = if is_really_broken {
        obj.get_settings().minimization_attempts_with_signal_timeout
    } else {
        obj.get_settings().minimization_attempts
    };
    let mut minimized_output = false;
    let mut valid_output = false;
    let mut current_alternative_idx: i32 = STRING_MINIMIZATION_LIMIT as i32;
    let mut tries = 0;

    while attempts > 0 {
        attempts -= 1;
        tries += 1;

        let new_lines;

        if lines.len() <= STRING_MINIMIZATION_LIMIT {
            if current_alternative_idx >= lines.len() as i32 {
                current_alternative_idx = lines.len() as i32 - 1;
            }
            if current_alternative_idx >= 0 {
                new_lines =
                    minimize_lines_one_by_one(full_name, &lines, current_alternative_idx as usize);
                current_alternative_idx -= 1;
            } else {
                break;
            }
        } else {
            new_lines = minimize_lines(full_name, &lines, &mut rng);
        }

        if new_lines.len() == lines.len() {
            break;
        };

        let (is_really_broken, output) = execute_command_and_connect_output(obj, full_name);
        if is_really_broken || obj.is_broken(&output) {
            attempts = if is_really_broken {
                obj.get_settings().minimization_attempts_with_signal_timeout
            } else {
                obj.get_settings().minimization_attempts
            };
            lines = new_lines;
            minimized_output = true;
            valid_output = true;
            current_alternative_idx = STRING_MINIMIZATION_LIMIT as i32;
        } else {
            valid_output = false;
        }
    }

    if !minimized_output || !valid_output {
        let mut output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(false)
            .open(full_name)
            .unwrap();

        // Restore content of file
        if !minimized_output {
            write!(output_file, "{data}").unwrap();
        }
        // If minimization was successful, but last run broke file, restore latest good configuration
        else if !valid_output {
            write!(output_file, "{}", lines.join("\n")).unwrap();
        }
    }

    let (is_really_broken, output) = execute_command_and_connect_output(obj, full_name);
    assert!(is_really_broken || obj.is_broken(&output));

    if old_line_number == lines.len() {
        info!(
            "File {full_name}, was not minimized after {tries} attempts, had {old_line_number} lines",
        );
    } else {
        info!(
            "File {full_name}, minimized from {old_line_number} to {} lines after {tries} attempts",
            lines.len(),
        );
    }
}

#[allow(clippy::borrowed_box)]
pub fn minimize_binary_output(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let Ok(data) = fs::read(full_name) else {
        info!("INFO: Cannot read content of {full_name}");
        return;
    };

    let (is_really_broken, output) = execute_command_and_connect_output(obj, full_name);
    assert!(
        (is_really_broken || obj.is_broken(&output)),
        "At start should be broken!"
    );

    let mut rng = rand::thread_rng();

    let mut old_new_data = data.clone();
    let items_number = data.len();

    let mut attempts = if is_really_broken {
        obj.get_settings().minimization_attempts_with_signal_timeout
    } else {
        obj.get_settings().minimization_attempts
    };
    let mut minimized_output = false;
    let mut valid_output = false;
    let mut tries = 0;

    while attempts > 0 {
        attempts -= 1;
        tries += 1;

        let Some(new_data) = minimize_binaries(full_name, &old_new_data, &mut rng) else {
            break;
        };
        if new_data.len() == old_new_data.len() {
            break;
        }

        let (is_really_broken, output) = execute_command_and_connect_output(obj, full_name);
        if is_really_broken || obj.is_broken(&output) {
            attempts = if is_really_broken {
                obj.get_settings().minimization_attempts_with_signal_timeout
            } else {
                obj.get_settings().minimization_attempts
            };
            old_new_data = new_data;
            minimized_output = true;
            valid_output = true;
        } else {
            valid_output = false;
        }
    }

    if !minimized_output || !valid_output {
        let mut output_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(false)
            .open(full_name)
            .unwrap();

        // Restore content of file
        if !minimized_output {
            output_file.write_all(&data).unwrap();
        }
        // If minimization was successful, but last run broke file, restore latest good configuration
        else if !valid_output {
            output_file.write_all(&old_new_data).unwrap();
        }
    }

    let (is_really_broken, output) = execute_command_and_connect_output(obj, full_name);
    assert!(is_really_broken || obj.is_broken(&output));

    if items_number == old_new_data.len() {
        info!(
            "File {full_name}, was not minimized after {tries} attempts, had {items_number} bytes",
        );
    } else {
        info!(
            "File {full_name}, minimized from {items_number} to {} bytes after {tries} attempts",
            old_new_data.len(),
        );
    }
}

pub fn minimize_binaries(full_name: &str, data: &Vec<u8>, rng: &mut ThreadRng) -> Option<Vec<u8>> {
    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    if data.len() <= 3 {
        if data.len() == 1 {
            return None;
        }
        let mut temp_data = data.clone();
        temp_data.remove(rng.gen_range(0..data.len()));
        output_file.write_all(&temp_data).unwrap();
        return Some(temp_data);
    }

    let number = rng.gen_range(0..=20);
    let content;

    let limit = max(1, rng.gen_range(0..(max(1, data.len() / 5))));

    if number == 0 {
        // Removing from start
        content = data[limit..].to_vec();
    } else if number < 8 {
        // Removing from end
        let limit = data.len() - limit;
        content = data[..limit].to_vec();
    } else {
        content = remove_random_from_middle(rng, data);
    }

    output_file.write_all(&content).unwrap();
    Some(content)
}
pub fn remove_single_def(lines: &Vec<String>, rng: &mut ThreadRng) -> Option<Vec<String>> {
    let mut list_def = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.trim().starts_with("def ") {
            list_def.push(idx);
        }
    }
    if list_def.len() <= 1 {
        return None;
    }
    let start_idx = rng.gen_range(0..list_def.len());
    let start = list_def[start_idx];
    let end = if start_idx == list_def.len() - 1 {
        lines.len() - 1
    } else {
        list_def[start_idx + 1]
    };
    let start_end = start..end;

    let mut new_list = Vec::new();
    for (idx, s) in lines.iter().enumerate() {
        if start_end.contains(&idx) {
            continue;
        }
        new_list.push(s.clone());
    }
    Some(new_list)
}

pub fn remove_single_docstring(lines: &[String], rng: &mut ThreadRng) -> Option<Vec<String>> {
    let mut list_def = Vec::new();
    let mut curr_docstring = None;
    for (idx, line) in lines.iter().enumerate() {
        let line_trim = line.trim();
        if line_trim.starts_with("\"\"\"") && line_trim.ends_with("\"\"\"") && line_trim.len() > 3 {
            list_def.push((idx, idx));
        } else if line_trim.starts_with("\"\"\"") || line_trim.ends_with("\"\"\"") {
            if curr_docstring.is_none() {
                curr_docstring = Some(idx);
            } else {
                list_def.push((curr_docstring.unwrap(), idx));
                curr_docstring = None;
            }
        }
    }
    if list_def.is_empty() {
        return None;
    }
    let start_idx = rng.gen_range(0..list_def.len());
    let llen = list_def[start_idx];

    let start_end = llen.0..=llen.1;
    let mut new_list = Vec::new();
    for (idx, s) in lines.iter().enumerate() {
        if start_end.contains(&idx) {
            continue;
        }
        new_list.push(s.clone());
    }
    Some(new_list)
}

pub fn remove_all_comments(lines: &Vec<String>) -> Vec<String> {
    let mut new_data = Vec::new();
    for line in lines {
        if !line.trim().starts_with('#') {
            new_data.push(line.clone());
        }
    }
    new_data
}

pub fn minimize_lines(full_name: &str, lines: &Vec<String>, rng: &mut ThreadRng) -> Vec<String> {
    assert!(lines.len() >= STRING_MINIMIZATION_LIMIT);
    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let number = rng.gen_range(0..=25);
    let mut content = Vec::new();

    let limit = max(1, rng.gen_range(0..(max(1, lines.len() / 5))));

    // Methods to better remove lines but only for python code
    if rng.gen_bool(0.25) {
        if rng.gen_bool(0.5) {
            if let Some(new_data) = remove_single_def(lines, rng) {
                content = new_data;
            }
        } else if rng.gen_bool(0.9) {
            if let Some(new_data) = remove_single_docstring(lines, rng) {
                content = new_data;
            }
        } else {
            content = remove_all_comments(lines);
        }
    }

    // If python code was not changed, try again
    if content.is_empty() || content.len() == lines.len() {
        if number < 3 {
            // Removing from start
            content = lines[limit..].to_vec();
        } else if number < 6 {
            // Removing from end
            let limit = lines.len() - limit;
            content = lines[..limit].to_vec();
        } else if number < 15 {
            // Removing code between empty lines
            content = remove_code_between_empty_lines(rng, lines);
        } else if number <= 23 {
            content = remove_random_from_middle(rng, lines);
        } else {
            // Removing randoms
            content = remove_random_items(rng, lines, limit);
        }
    }

    write!(output_file, "{}", content.join("\n")).unwrap();
    content
}
pub fn minimize_lines_one_by_one(full_name: &str, lines: &[String], idx: usize) -> Vec<String> {
    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let mut temp_lines = lines.to_vec();
    temp_lines.remove(idx);
    write!(output_file, "{}", temp_lines.join("\n")).unwrap();
    temp_lines
}

pub fn remove_code_between_empty_lines(rng: &mut ThreadRng, orig: &[String]) -> Vec<String> {
    let mut indexes = Vec::new();
    for (idx, line) in orig.iter().enumerate() {
        if line.trim().is_empty() {
            indexes.push(idx);
        }
    }
    if indexes.len() < 2 {
        return orig[0..(orig.len() / 2)].to_vec();
    }

    let limits = get_two_random_not_equal_ints(rng, orig.len());

    orig[(limits.0)..(limits.1)].to_vec()
}

pub fn remove_random_from_middle<T>(rng: &mut ThreadRng, orig: &[T]) -> Vec<T>
where
    T: Clone,
{
    let limits = get_two_random_not_equal_ints(rng, orig.len());
    orig[(limits.0)..(limits.1)].to_vec()
}

pub fn remove_random_items<T>(rng: &mut ThreadRng, orig: &[T], limit: usize) -> Vec<T>
where
    T: Clone,
{
    let content = orig.to_vec();
    let mut indexes_to_remove = HashSet::new();
    for _ in 0..limit {
        indexes_to_remove.insert(rng.gen_range(0..content.len()));
    }

    let mut new_data = Vec::new();
    for (idx, line) in content.into_iter().enumerate() {
        if !indexes_to_remove.contains(&idx) {
            new_data.push(line);
        }
    }
    new_data
}

fn get_two_random_not_equal_ints(rng: &mut ThreadRng, length: usize) -> (usize, usize) {
    loop {
        let limits = (rng.gen_range(0..length), rng.gen_range(0..length));
        if limits.0 == limits.1 {
            continue;
        }
        if limits.0 > limits.1 {
            return (limits.1, limits.0);
        }
        return (limits.0, limits.1);
    }
}

#[allow(clippy::borrowed_box)]
pub fn execute_command_and_connect_output(
    obj: &Box<dyn ProgramConfig>,
    full_name: &str,
) -> (bool, String) {
    let content_before = fs::read(full_name).unwrap(); // In each iteration be sure that before and after, file is the same

    let command = obj.get_run_command(full_name);
    let output = command.wait_with_output().unwrap();
    let mut is_signal_code_timeout_broken = false;

    let mut str_out = collect_output(&output);

    if obj.get_settings().error_when_found_signal {
        if let Some(_signal) = output.status.signal() {
            // info!("Non standard output signal {}", signal);
            is_signal_code_timeout_broken = true;
        }
    }
    if !obj.get_settings().allowed_error_statuses.is_empty()
        && output.status.code().is_some()
        && !obj
            .get_settings()
            .allowed_error_statuses
            .contains(&output.status.code().unwrap())
    {
        // info!("Non standard output status {:?}", output.status.code());
        is_signal_code_timeout_broken = true;
    }
    if obj.get_settings().timeout > 0 && str_out.contains(TIMEOUT_MESSAGE) {
        // info!("Timeout found");
        is_signal_code_timeout_broken = true;
    }

    let res = fs::write(full_name, content_before); // TODO read and save only in unsafe mode, most of tools not works unsafe - not try to fix things, but only reads content of file, so the no need to save previous content of file

    assert!(
        res.is_ok(),
        "{res:?} - {full_name} - probably you need to set write permissions to this file"
    );

    str_out.push_str(&format!(
        "\n##### Automatic Fuzzer note, output status \"{:?}\", output signal \"{:?}\"\n",
        output.status.code(),
        output.status.signal()
    ));

    (is_signal_code_timeout_broken, str_out)
}

#[test]
fn test_remove_single_docstring() {
    for _i in 0..100 {
        let mut rng = rand::thread_rng();
        let lines = r###"
    """
    DOCSTRING
    """
    def function():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();
        let expected = r###"
    def function():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();

        assert_eq!(remove_single_docstring(&lines, &mut rng).unwrap(), expected);
    }
}
#[test]
fn test_remove_single_docstring2() {
    for _i in 0..100 {
        let mut rng = rand::thread_rng();
        let lines = r###"
    """
    DOCSTRING
    """
    def function():
        pass
    """
    PORTORYKO
    """
    def romma():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();
        let expected1 = r###"
    def function():
        pass
    """
    PORTORYKO
    """
    def romma():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();
        let expected2 = r###"
    """
    DOCSTRING
    """
    def function():
        pass
    def romma():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();

        let result = remove_single_docstring(&lines, &mut rng).unwrap();
        assert!([expected1, expected2].contains(&result));
    }
}

#[test]
fn test_remove_single_docstring3() {
    for _i in 0..100 {
        let mut rng = rand::thread_rng();
        let lines = r###"
    """DOCSTRING"""
    def function():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();
        let expected = r###"
    def function():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();

        assert_eq!(remove_single_docstring(&lines, &mut rng).unwrap(), expected);
    }
}

#[test]
fn test_remove_single_def() {
    for _i in 0..100 {
        let mut rng = rand::thread_rng();
        let lines = r###"
    def function():
        pass
    def function2():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();
        let expected1 = r###"
    def function2():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();
        let expected2 = r###"
    def function():
        pass
    "###
        .split("\n")
        .map(String::from)
        .collect::<Vec<String>>();

        let ret = remove_single_def(&lines, &mut rng).unwrap();
        if ![&expected2, &expected1].contains(&&ret) {
            info!("RET {:?}", ret);
            info!("EXP1 {:?}", expected1);
            info!("EXP2 {:?}", expected2);
            assert!([expected2, expected1].contains(&ret));
        }
    }
}

#[test]
fn test_remove_all_comments() {
    let lines = r###"
    # comment
    def function():
        pass
    "###
    .split("\n")
    .map(String::from)
    .collect::<Vec<String>>();
    let expected = r###"
    def function():
        pass
    "###
    .split("\n")
    .map(String::from)
    .collect::<Vec<String>>();

    assert_eq!(remove_all_comments(&lines), expected);
}
