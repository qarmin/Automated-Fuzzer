use crate::broken_files::create_broken_files;
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::{CustomItems, Setting};
use std::process::{Child, Command, Stdio};

pub struct CustomStruct {
    pub settings: Setting,
    pub custom_items: CustomItems,
}

impl ProgramConfig for CustomStruct {
    fn is_broken(&self, content: &str) -> bool {
        self.custom_items.search_items.iter().any(|x| content.contains(x)) && !self.ignored_signal_output(content)
    }
    fn ignored_signal_output(&self, content: &str) -> bool {
        !self.custom_items.ignored_items.is_empty()
            && self.custom_items.ignored_items.iter().any(|x| content.contains(x))
    }
    fn get_full_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.args(
            self.custom_items
                .command_parts
                .iter()
                .skip(1)
                .map(|e| e.replace("FILE_PATHS_TO_PROVIDE", full_name)),
        );
        command
    }
    fn get_group_command(&self, full_name: &[String]) -> Command {
        let mut command = self._get_basic_run_command();
        command.args(
            self.custom_items
                .command_parts
                .iter()
                .skip(1)
                .map(|e| e.replace("FILE_PATHS_TO_PROVIDE", &full_name.join(" "))),
        );
        command
    }
    fn _get_basic_run_command(&self) -> Command {
        let mut comm = Command::new(&self.custom_items.command_parts[0]);
        comm.stderr(Stdio::piped()).stdout(Stdio::piped());
        comm
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, self.custom_items.file_type)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        self.custom_items.group_mode
    }
}
