use std::cmp::max;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use rand::prelude::ThreadRng;
use rand::Rng;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

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
    let output = execute_command_and_connect_output(obj, full_name);

    if !obj.is_broken(&output) {
        return;
    }

    let mut lines = data.lines().map(str::to_string).collect::<Vec<String>>();
    let mut rng = rand::thread_rng();

    let old_line_number = lines.len();

    let mut attempts = obj.get_settings().minimization_attempts;
    let mut minimized_output = false;
    let mut valid_output = false;
    while attempts > 0 {
        let Some(new_lines) = minimize_lines(full_name, &lines, &mut rng) else {
            break;
        };
        if new_lines.len() == lines.len() {
            break;
        }

        let output = execute_command_and_connect_output(obj, full_name);
        if obj.is_broken(&output) {
            attempts = obj.get_settings().minimization_attempts;
            lines = new_lines;
            minimized_output = true;
            valid_output = true;
        } else {
            attempts -= 1;
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
        if !valid_output {
            write!(output_file, "{}", lines.join("\n")).unwrap();
        }
    }

    assert!(obj.is_broken(&execute_command_and_connect_output(obj, full_name)));

    println!(
        "File {full_name}, minimized from {old_line_number} to {} lines",
        lines.len()
    );
}

#[allow(clippy::borrowed_box)]
pub fn minimize_binary_output(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let Ok(data) = fs::read(full_name) else {
        println!("INFO: Cannot read content of {full_name}");
        return;
    };
    let output = execute_command_and_connect_output(obj, full_name);

    if !obj.is_broken(&output) {
        return;
    }

    let mut rng = rand::thread_rng();

    let mut old_new_data = data.clone();
    let items_number = data.len();

    let mut attempts = obj.get_settings().minimization_attempts;
    let mut minimized_output = false;
    let mut valid_output = false;
    while attempts > 0 {
        let Some(new_data) = minimize_binaries(full_name, &old_new_data, &mut rng) else {
            break;
        };
        if new_data.len() == old_new_data.len() {
            break;
        }

        let output = execute_command_and_connect_output(obj, full_name);
        if obj.is_broken(&output) {
            attempts = obj.get_settings().minimization_attempts;
            old_new_data = new_data;
            minimized_output = true;
            valid_output = true;
        } else {
            attempts -= 1;
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
        if !valid_output {
            output_file.write_all(&old_new_data).unwrap();
        }
    }

    assert!(obj.is_broken(&execute_command_and_connect_output(obj, full_name)));

    println!(
        "File {full_name}, minimized from {items_number} to {} bytes",
        old_new_data.len()
    );
}

#[allow(clippy::comparison_chain)]
pub fn minimize_binaries(
    full_name: &str,
    data: &Vec<u8>,
    rng: &mut ThreadRng,
) -> Option<Vec<u8>> {
    if data.len() <= 3 {
        return None;
    }

    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let number = rng.gen_range(0..=20);
    let  content;

    let limit = max(1, rng.gen_range(0..(max(1, data.len() / 5))));

    if number == 0 {
        // Removing from start
        content = data[limit..].to_vec();
    } else if number < 8 {
        // Removing from end
        let limit = data.len() - limit;
        content = data[..limit].to_vec();
    } else {
        // Removing random from middle
        let limit_upper;
        let limit_lower;
        loop {
            let limit1 = rng.gen_range(0..data.len());
            let limit2 = rng.gen_range(0..data.len());
            if limit1 > limit2 {
                limit_lower = limit2;
                limit_upper = limit1;
                break;
            } else if limit2 > limit1 {
                limit_lower = limit1;
                limit_upper = limit2;
                break;
            }
        }
        content = data[limit_lower..limit_upper].to_vec();
    }

    output_file.write_all(&content).unwrap();
    Some(content)
}


#[allow(clippy::comparison_chain)]
pub fn minimize_lines(
    full_name: &str,
    lines: &Vec<String>,
    rng: &mut ThreadRng,
) -> Option<Vec<String>> {
    if lines.len() <= 3 {
        return None;
    }

    let mut output_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(full_name)
        .unwrap();

    let number = rng.gen_range(0..=3);
    let mut content;

    let limit = max(1, rng.gen_range(0..(max(1, lines.len() / 5))));

    if number == 0 {
        // Removing from start
        content = lines[limit..].to_vec();
    } else if number == 1 {
        // Removing from end
        let limit = lines.len() - limit;
        content = lines[..limit].to_vec();
    } else if number == 2 {
        // Removing random from middle
        let limit_upper;
        let limit_lower;
        loop {
            let limit1 = rng.gen_range(0..lines.len());
            let limit2 = rng.gen_range(0..lines.len());
            if limit1 > limit2 {
                limit_lower = limit2;
                limit_upper = limit1;
                break;
            } else if limit2 > limit1 {
                limit_lower = limit1;
                limit_upper = limit2;
                break;
            }
        }
        content = lines[limit_lower..limit_upper].to_vec();
    } else {
        // Removing randoms
        content = lines.to_vec();
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
        content = new_data
    }

    write!(output_file, "{}", content.join("\n")).unwrap();
    Some(content)
}

#[allow(clippy::borrowed_box)]
pub fn execute_command_and_connect_output(obj: &Box<dyn ProgramConfig>, full_name: &str) -> String {
    let command = obj.get_run_command(full_name);
    let output = command.wait_with_output().unwrap();

    let mut out = output.stderr.clone();
    out.push(b'\n');
    out.extend(output.stdout);
    String::from_utf8(out).unwrap()
}
