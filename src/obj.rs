use crate::common::{
    create_new_file_name, create_new_file_name_for_minimization, try_to_save_file, CheckGroupFileMode, OutputResult,
};
use crate::settings::{Setting, StabilityMode};
use log::error;
use std::process::{Child, Command};
pub trait ProgramConfig: Sync {
    fn get_broken_items_list(&self) -> &[String];
    fn get_ignored_items_list(&self) -> &[String];
    fn get_minimize_additional_command(&self) -> Option<String> {
        None
    }

    fn is_broken(&self, content: &str) -> bool;
    fn validate_output_and_save_file(&self, full_name: String, output: &str) -> Option<String> {
        let new_name = create_new_file_name(self.get_settings(), &full_name);
        error!("File {full_name} saved to {new_name}\n{output}");

        try_to_save_file(&full_name, &new_name);
        Some(new_name)
    }
    fn get_stability_mode(&self) -> StabilityMode;

    fn validate_txt_and_save_file(&self, full_name: String, data: &[String]) -> Option<String> {
        let new_name = create_new_file_name(self.get_settings(), &full_name);
        let mut diff = match self.get_stability_mode() {
            StabilityMode::None => unreachable!(),
            StabilityMode::FileContent => "File content between runs differs",
            StabilityMode::ConsoleOutput => "Console output between runs differs",
            StabilityMode::OutputContent => "Console output or file content between runs differs",
        }
        .to_string();

        for d in data {
            diff.push_str(&format!("\n=========================\n{}", d));
        }
        diff.push_str("\n=========================\n");

        error!("File {full_name} saved to {new_name}\n{diff}");

        try_to_save_file(&full_name, &new_name);
        Some(new_name)
    }

    // When app crashes, sometimes it not gives any status code
    // To be able to ignore certain groups of files, we can use this function
    fn ignored_signal_output(&self, _output: &str) -> bool {
        false
    }

    fn get_full_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.arg(full_name);
        command
    }
    fn get_group_command(&self, files: &[String]) -> Command {
        let mut command = self._get_basic_run_command();
        command.args(files);
        command
    }

    fn run_command(&self, full_name: &str) -> Child {
        self.get_full_command(full_name).spawn().unwrap()
    }
    fn run_group_command(&self, files: &[String]) -> Child {
        self.get_group_command(files).spawn().unwrap()
    }
    fn get_minimize_command(&self, full_name: &str) -> Command {
        let new_full_name = create_new_file_name_for_minimization(self.get_settings(), full_name);
        let temp_file_name = "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ";
        let run_command = self.get_full_command(temp_file_name);

        let run_command_as_string = format!(
            "'{}' '{}'",
            run_command.get_program().to_string_lossy(),
            run_command
                .get_args()
                .map(|e| e.to_string_lossy().replace(temp_file_name, &new_full_name))
                .collect::<Vec<_>>()
                .join("' '")
        );
        // minimizer --input-file input.txt --output-file output.txt --command "echo {}" --attempts 300 --broken-info "BROKEN"
        let mut minimize_command = Command::new("minimizer");
        let broken_info = self
            .get_broken_items_list()
            .iter()
            .flat_map(|e| vec!["--broken-info".to_string(), e.to_string()])
            .collect::<Vec<_>>();
        let ignored_info = self
            .get_ignored_items_list()
            .iter()
            .flat_map(|e| vec!["--ignored-info".to_string(), e.to_string()])
            .collect::<Vec<_>>();
        minimize_command.args([
            "--input-file", full_name, "--output-file", &new_full_name, "--command", &run_command_as_string,
            "--attempts", "1000", "-r",
        ]);
        if let Some(additional_minimize_command) = self.get_minimize_additional_command() {
            minimize_command.args(["--additional-command", &additional_minimize_command]);
        }
        minimize_command.args(broken_info);
        minimize_command.args(ignored_info);
        minimize_command
    }
    fn _get_basic_run_command(&self) -> Command;
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
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        CheckGroupFileMode::None
    }
    fn get_number_of_minimization(&self, output_result: &OutputResult) -> u32 {
        if output_result.is_only_signal_broken() {
            self.get_settings().minimization_attempts_with_signal_timeout
        } else {
            self.get_settings().minimization_attempts
        }
    }
    // fn remove_not_needed_lines_from_output(&self, output: String) -> String {
    //     output
    // }
}
