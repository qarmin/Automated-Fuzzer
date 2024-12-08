use std::os::unix::fs::PermissionsExt;
use std::os::unix::prelude::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Instant;
use std::{fs, process};

use jwalk::WalkDir;
use log::{error, info};
use once_cell::sync::{Lazy, OnceCell};
use rand::Rng;

use crate::obj::ProgramConfig;
use crate::settings::{Setting, TIMEOUT_MESSAGE};

pub static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
pub static TIMEOUT_SECS: OnceCell<u64> = OnceCell::new();

#[derive(PartialOrd, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CheckGroupFileMode {
    None,
    ByFolder,
    ByFilesGroup,
}

pub fn check_if_app_ends() -> bool {
    let elapsed = START_TIME.elapsed().as_secs();
    let timeout = TIMEOUT_SECS.get().unwrap();
    elapsed > *timeout
}

pub fn close_app_if_timeouts() {
    if check_if_app_ends() {
        info!("Timeout reached, closing app");
        std::process::exit(0);
    }
}

pub fn remove_and_create_entire_folder(folder_name: &str) {
    if Path::new(folder_name).exists() {
        info!("Removing folder {folder_name}");
        if let Err(e) = fs::remove_dir_all(folder_name) {
            let number_of_files = WalkDir::new(folder_name).max_depth(999).into_iter().count();
            panic!("Failed to remove folder {folder_name} (number of files {number_of_files}), reason {e}");
        }
        info!("Folder {folder_name} removed");
    } else {
        info!("Folder {folder_name} not exists, so not removing it");
    }

    info!("Creating folder {folder_name}");
    fs::create_dir_all(folder_name).unwrap();
    info!("Folder {folder_name} created");
}

pub fn create_new_file_name(setting: &Setting, old_name: &str) -> String {
    let mut random_number = rand::thread_rng().gen_range(1..100_000);
    loop {
        let pat = Path::new(&old_name);
        let extension = pat.extension().unwrap().to_str().unwrap().to_string();
        let file_name = pat.file_stem().unwrap().to_str().unwrap().to_string();
        let new_name = format!("{}/{file_name}{}.{extension}", setting.broken_files_dir, random_number);
        if !Path::new(&new_name).exists() {
            return new_name;
        }
        random_number += 1;
    }
}
pub fn create_new_file_name_for_minimization(setting: &Setting, old_name: &str) -> String {
    let mut random_number = rand::thread_rng().gen_range(1..1000);
    loop {
        let pat = Path::new(&old_name);
        let extension = pat.extension().unwrap().to_str().unwrap().to_string();
        let file_name = pat.file_stem().unwrap().to_str().unwrap().to_string();
        let new_name = format!(
            "{}/{file_name}_minimized_{random_number}.{extension}",
            setting.broken_files_dir,
        );
        if !Path::new(&new_name).exists() {
            return new_name;
        }
        random_number += 1;
    }
}

pub fn collect_output(output: &Output) -> String {
    let stdout = &output.stdout;
    let stderr = &output.stderr;
    let stdout_str = String::from_utf8_lossy(stdout);
    let stderr_str = String::from_utf8_lossy(stderr);
    format!("{stdout_str}\n{stderr_str}")
}

pub fn try_to_save_file(full_name: &str, new_name: &str) {
    if let Err(e) = fs::copy(full_name, new_name) {
        error!("Failed to copy file {full_name}, reason {e}, (maybe broken files folder not exists?)");
    };
}

pub fn minimize_new(obj: &Box<dyn ProgramConfig>, full_name: &str) {
    let mut minimization_command = obj.get_minimize_command(full_name);
    let minimization_command_str = collect_command_to_string(&minimization_command);
    let output = minimization_command.spawn().unwrap().wait_with_output().unwrap();
    let str_out = collect_output(&output);

    if obj.get_settings().debug_print_results {
        info!("Minimization output: {str_out}");
        info!("Minimization command: {minimization_command_str}");
    }
}

pub fn execute_command_on_pack_of_files(
    obj: &Box<dyn ProgramConfig>,
    folder_name: &str,
    files: &[String],
) -> OutputResult {
    let (child, command) = match obj.get_files_group_mode() {
        CheckGroupFileMode::ByFolder => (obj.run_command(folder_name), obj.get_full_command(folder_name)),
        CheckGroupFileMode::ByFilesGroup => (obj.run_group_command(files), obj.get_group_command(files)),
        CheckGroupFileMode::None => panic!("Invalid mode"),
    };
    let output = child.wait_with_output().unwrap();
    let mut str_out = collect_output(&output);

    let is_signal_broken = obj.get_settings().error_when_found_signal
        && output.status.signal().is_some()
        && !obj.ignored_signal_output(&str_out);

    let is_code_broken = !obj.get_settings().allowed_error_statuses.is_empty()
        && output.status.code().map_or(false, |code| {
            if obj.get_settings().ignore_timeout_errors && code == 124 {
                false
            } else {
                !obj.get_settings().allowed_error_statuses.contains(&code)
            }
        });

    let timeouted = obj.get_settings().timeout > 0
        && str_out.contains(TIMEOUT_MESSAGE)
        && !obj.get_settings().ignore_timeout_errors;

    str_out.push_str(&format!(
        "\n##### Automatic Fuzzer note, output status \"{:?}\", output signal \"{:?}\"\n",
        output.status.code(),
        output.status.signal()
    ));

    OutputResult::new(
        output.status.code(),
        output.status.signal(),
        is_signal_broken,
        is_code_broken,
        obj.is_broken(&str_out),
        timeouted,
        str_out,
        collect_command_to_string(&command),
    )
}

#[allow(unused)]
#[derive(Debug)]
pub(crate) struct OutputResult {
    output: String,
    command_str: String,
    code: Option<i32>,
    signal: Option<i32>,
    is_signal_broken: bool,
    is_code_broken: bool,
    // timeouted is only field to check if timeout was reached
    // if timeouted is true, then always is_signal_broken is also true
    have_invalid_output: bool,
    timeouted: bool,
}
impl OutputResult {
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn new(
        code: Option<i32>,
        signal: Option<i32>,
        is_signal_broken: bool,
        is_code_broken: bool,
        have_invalid_output: bool,
        timeouted: bool,
        output: String,
        command_str: String,
    ) -> Self {
        Self {
            output,
            code,
            signal,
            is_signal_broken,
            is_code_broken,
            have_invalid_output,
            timeouted,
            command_str,
        }
    }
    pub fn is_broken(&self) -> bool {
        self.is_signal_broken || self.have_invalid_output || self.is_code_broken || self.timeouted
    }
    // pub fn is_only_signal_broken(&self) -> bool {
    //     self.is_signal_broken && !self.have_invalid_output && !self.is_code_broken
    // }
    pub fn get_output(&self) -> &str {
        &self.output
    }
    pub fn get_command_str(&self) -> &str {
        &self.command_str
    }
    pub fn debug_print(&self) {
        info!("Is broken: {}, is_signal_broken - {}, have_invalid_output - {}, is_code_broken - {}, timeouted - {}, code - {:?}, signal - {:?}", self.is_broken(), self.is_signal_broken, self.have_invalid_output, self.is_code_broken, self.timeouted, self.code, self.signal);
    }
}

#[allow(clippy::borrowed_box)]
pub fn execute_command_and_connect_output(obj: &Box<dyn ProgramConfig>, full_name: &str) -> OutputResult {
    let content_before = fs::read(full_name).unwrap(); // In each iteration be sure that before and after, file is the same

    let command = obj.get_full_command(full_name);

    let child = obj.run_command(full_name);
    let output = child.wait_with_output().unwrap();

    let mut str_out = collect_output(&output);

    let is_signal_broken = obj.get_settings().error_when_found_signal
        && output.status.signal().is_some()
        && !obj.ignored_signal_output(&str_out);

    let is_code_broken = !obj.get_settings().allowed_error_statuses.is_empty()
        && output.status.code().map_or(false, |code| {
            if obj.get_settings().ignore_timeout_errors && code == 124 {
                false
            } else {
                !obj.get_settings().allowed_error_statuses.contains(&code)
            }
        });

    let timeouted = obj.get_settings().timeout > 0
        && str_out.contains(TIMEOUT_MESSAGE)
        && !obj.get_settings().ignore_timeout_errors;

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

    OutputResult::new(
        output.status.code(),
        output.status.signal(),
        is_signal_broken,
        is_code_broken,
        obj.is_broken(&str_out),
        timeouted,
        str_out,
        collect_command_to_string(&command),
    )
}

pub fn run_ruff_format_check(item_to_check: &str, print_info: bool) -> Output {
    if print_info {
        info!("Ruff checking format on: {item_to_check}");
    }
    let output = std::process::Command::new("ruff")
        .arg("format")
        .arg(item_to_check)
        .arg("--check")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    if print_info {
        info!("Ruff checked format on: {item_to_check}");
    }
    output
}

#[allow(dead_code)]
pub fn run_ruff_format(item_to_check: &str, print_info: bool) -> Output {
    if print_info {
        info!("Ruff formatted on: {item_to_check}");
    }
    let output = Command::new("ruff")
        .arg("format")
        .arg(item_to_check)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    if print_info {
        info!("Ruff formatted on: {item_to_check}");
    }
    output
}

pub fn find_broken_files_by_cpython(dir_to_check: &str) -> Vec<String> {
    let output = Command::new("python3")
        .arg("-m")
        .arg("compileall")
        .arg(dir_to_check)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    let all = collect_output(&output);
    // error!("{all}");

    let mut next_file_is_broken = false;
    let mut broken_files = vec![];
    for line in all.lines().rev() {
        if line.starts_with("Compiling '") {
            if next_file_is_broken {
                let file_name = line.strip_prefix("Compiling '").unwrap().strip_suffix("'...").unwrap();
                next_file_is_broken = false;
                if !Path::new(&file_name).is_file() {
                    error!("BUG: Invalid missing file name '{file_name}' in line '{line}'");
                    continue;
                }
                broken_files.push(file_name.to_string());
            }
        } else if line.contains("Error:") {
            next_file_is_broken = true;
        }
    }

    let _ = fs::remove_dir_all(format!("{dir_to_check}/__pycache__"));
    broken_files
}

pub fn check_files_number(name: &str, dir: &str) {
    info!(
        "{name} - {} - Files Number {}.",
        dir,
        WalkDir::new(dir)
            .max_depth(999)
            .into_iter()
            .flatten()
            .filter(|e| e.path().is_file())
            .count()
    );
}
pub fn calculate_number_of_files(dir: &str) -> usize {
    let mut number_of_files = 0;
    for i in WalkDir::new(dir).max_depth(999).into_iter().flatten() {
        if i.path().is_file() {
            number_of_files += 1;
        }
    }
    number_of_files
}

pub fn generate_files(obj: &Box<dyn ProgramConfig>, settings: &Setting) {
    let command = obj.broken_file_creator();
    let output = command.wait_with_output().unwrap();
    let out = String::from_utf8(output.stdout).unwrap();
    if !output.status.success() {
        error!("{:?}", output.status);
        error!("{out}");
        error!("Failed to generate files");
        process::exit(1);
    }
    if settings.debug_print_broken_files_creator {
        info!("{out}");
    };
}

pub fn collect_files(settings: &Setting) -> (Vec<String>, u64) {
    let mut size_all = 0;
    let mut files = Vec::new();
    assert!(Path::new(&settings.temp_possible_broken_files_dir).is_dir());
    for i in WalkDir::new(&settings.temp_possible_broken_files_dir)
        .max_depth(999)
        .into_iter()
        .flatten()
    {
        let path = i.path();
        if !path.is_file() {
            continue;
        }
        let Ok(metadata) = i.metadata() else {
            continue;
        };
        metadata.permissions().set_mode(0o777);
        let Some(s) = path.to_str() else {
            continue;
        };
        if settings.extensions.iter().any(|e| s.to_lowercase().ends_with(e)) {
            files.push(s.to_string());
            size_all += metadata.len();
        }
    }
    if files.len() > settings.max_collected_files {
        files.truncate(settings.max_collected_files);
    }

    if files.is_empty() {
        dbg!(&settings);
        assert!(!files.is_empty());
    }

    (files, size_all)
}

pub(crate) fn collect_command_to_string(command: &Command) -> String {
    let args = command
        .get_args()
        .map(|e| {
            let tmp_string = e.to_string_lossy();
            if [" ", "\"", "\\", "/"].iter().any(|e| tmp_string.contains(e)) {
                format!("\"{}\"", tmp_string.replace("\"", "\\\""))
            } else {
                tmp_string.to_string()
            }
        })
        .collect::<Vec<_>>();
    format!("{} {}", command.get_program().to_string_lossy(), args.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn test_collect_command_to_string_simple() {
        let mut command = Command::new("echo");
        command.arg("Hello");
        let result = collect_command_to_string(&command);
        assert_eq!(result, "echo Hello");
    }

    #[test]
    fn test_collect_command_to_string_with_spaces() {
        let mut command = Command::new("echo");
        command.arg("Hello World");
        let result = collect_command_to_string(&command);
        assert_eq!(result, "echo \"Hello World\"");
    }

    #[test]
    fn test_collect_command_to_string_with_special_chars() {
        let mut command = Command::new("echo");
        command.arg("Hello \"World\"");
        let result = collect_command_to_string(&command);
        assert_eq!(result, "echo \"Hello \\\"World\\\"\"");
    }

    #[test]
    fn test_collect_command_to_string_with_multiple_args() {
        let mut command = Command::new("echo");
        command.args(&["Hello", "World"]);
        let result = collect_command_to_string(&command);
        assert_eq!(result, "echo Hello World");
    }

    #[test]
    fn test_collect_command_to_string_with_os_str() {
        let mut command = Command::new("echo");
        command.arg(OsStr::new("Hello"));
        let result = collect_command_to_string(&command);
        assert_eq!(result, "echo Hello");
    }
}
