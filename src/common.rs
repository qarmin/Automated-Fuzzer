use std::cmp::max;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::prelude::ExitStatusExt;
use std::path::Path;

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
            setting.output_dir,
            rand::thread_rng().gen_range(1..10000)
        );
        if !Path::new(&new_name).exists() {
            return new_name;
        }
    }
}

pub fn try_to_save_file(setting: &Setting, full_name: &str, new_name: &str) -> bool {
    if !setting.safe_run && setting.copy_broken_files {
        if let Err(e) = fs::copy(full_name, new_name) {
            eprintln!("Failed to copy file {full_name}, reason {e}, (maybe broken files folder not exists?)");
            return true;
        };
        return true;
    }
    false
}

#[allow(clippy::borrowed_box)]
pub fn minimize_string_output(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let Ok(data) = fs::read_to_string(full_name) else {
        println!("INFO: Cannot read content of {full_name}, probably because is not valid UTF-8");
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

    println!(
        "File {full_name}, minimized from {old_line_number} to {} lines after {tries} attempts",
        lines.len(),
    );
}

#[allow(clippy::borrowed_box)]
pub fn minimize_binary_output(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let Ok(data) = fs::read(full_name) else {
        println!("INFO: Cannot read content of {full_name}");
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

    println!(
        "File {full_name}, minimized from {items_number} to {} bytes after {tries} attempts",
        old_new_data.len(),
    );
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

pub fn minimize_lines(full_name: &str, lines: &Vec<String>, rng: &mut ThreadRng) -> Vec<String> {
    assert!(lines.len() >= STRING_MINIMIZATION_LIMIT);
    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let number = rng.gen_range(0..=25);
    let content;

    let limit = max(1, rng.gen_range(0..(max(1, lines.len() / 5))));

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

    write!(output_file, "{}", content.join("\n")).unwrap();
    content
}
pub fn minimize_lines_one_by_one(full_name: &str, lines: &Vec<String>, idx: usize) -> Vec<String> {
    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let mut temp_lines = lines.clone();
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

    let mut out = output.stderr.clone();
    out.push(b'\n');
    out.extend(output.stdout);
    let mut str_out = String::from_utf8_lossy(&out).to_string();
    str_out.push_str(&format!(
        "\n##### Automatic Fuzzer note, output status \"{:?}\", output signal \"{:?}\"\n",
        output.status.code(),
        output.status.signal()
    ));

    if obj.get_settings().error_when_found_signal {
        if let Some(_signal) = output.status.signal() {
            // println!("Non standard output signal {}", signal);
            is_signal_code_timeout_broken = true;
        }
    }
    if obj.get_settings().error_statuses_different_than_0_1
        && ![Some(0), Some(1)].contains(&output.status.code())
    {
        // println!("Non standard output status {:?}", output.status.code());
        is_signal_code_timeout_broken = true;
    }
    if obj.get_settings().timeout > 0 && str_out.contains(TIMEOUT_MESSAGE) {
        // println!("Timeout found");
        is_signal_code_timeout_broken = true;
    }

    fs::write(full_name, content_before).unwrap(); // TODO read and save only in usafe mode, most of tools not works unsafe - not try to fix things, but only reads content of file, so the no need to save previous content of file
    (is_signal_code_timeout_broken, str_out)
}
