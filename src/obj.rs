use jwalk::WalkDir;
use log::error;
use std::process::{Child, Command, Stdio};

use crate::common::{create_new_file_name, try_to_save_file};
use crate::settings::Setting;

pub trait ProgramConfig: Sync {
    fn is_broken(&self, content: &str) -> bool;
    fn validate_output_and_save_file(&self, full_name: String, output: String) -> Option<String> {
        let new_name = create_new_file_name(self.get_settings(), &full_name);
        let new_name_not_minimized = create_new_file_name(self.get_settings(), &full_name);
        error!("File {full_name} saved to {new_name}\n{output}");

        if try_to_save_file(self.get_settings(), &full_name, &new_name) {
            let _ = try_to_save_file(self.get_settings(), &full_name, &new_name_not_minimized);
            Some(new_name)
        } else {
            None
        }
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command().arg(full_name).spawn().unwrap()
    }
    fn _get_basic_run_command(&self) -> Command {
        let timeout = self.get_settings().timeout;

        let mut comm = if timeout == 0 {
            Command::new(&self.get_settings().app_binary)
        } else {
            let mut a = Command::new("timeout");
            a.arg("-v")
                .arg(&timeout.to_string())
                .arg(&self.get_settings().app_binary);
            a
        };
        comm.stderr(Stdio::piped()).stdout(Stdio::piped());
        comm
    }

    fn collect_files_in_dir_with_extension(&self, dir_to_check: &str) -> Vec<String> {
        let mut files_to_check = Vec::new();
        for i in WalkDir::new(dir_to_check).into_iter().flatten() {
            let path = i.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let Some(file_name) = file_name.to_str() else {
                continue;
            };
            let small_file_name = file_name.to_lowercase();

            if !self
                .get_settings()
                .extensions
                .iter()
                .any(|x| small_file_name.ends_with(x))
            {
                continue;
            }
            files_to_check.push(file_name.to_string());
        }
        files_to_check
    }
    fn broken_file_creator(&self) -> Child;
    fn get_settings(&self) -> &Setting;
    fn init(&mut self) {}
    fn remove_non_parsable_files(&self, _dir_to_check: &str) {}
    fn is_parsable(&self, _file_to_check: &str) -> bool {
        true
    }
    fn get_version(&self) -> String {
        panic!()
    }
    fn remove_not_needed_lines_from_output(&self, output: String) -> String {
        output
    }
}
