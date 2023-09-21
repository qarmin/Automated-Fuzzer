use log::info;
use std::process::{Child, Command, Stdio};

use crate::common::{create_new_file_name, try_to_save_file};
use crate::settings::Setting;

pub trait ProgramConfig: Sync {
    fn is_broken(&self, content: &str) -> bool;
    fn validate_output_and_save_file(&self, full_name: String, output: String) -> Option<String> {
        let new_name = create_new_file_name(self.get_settings(), &full_name);
        let new_name_not_minimized = create_new_file_name(self.get_settings(), &full_name);
        info!("\n_______________ File {full_name} saved to {new_name} _______________________");
        info!("{output}");

        if try_to_save_file(self.get_settings(), &full_name, &new_name) {
            let _ = try_to_save_file(self.get_settings(), &full_name, &new_name_not_minimized);
            Some(new_name)
        } else {
            None
        }
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command()
            .arg(full_name)
            .spawn()
            .unwrap()
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
    fn broken_file_creator(&self) -> Child;
    fn get_settings(&self) -> &Setting;
    fn init(&mut self) {}
    fn remove_non_parsable_files(&self, _dir_to_check: &str) {}
}
