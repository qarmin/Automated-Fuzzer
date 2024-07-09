use crate::broken_files::{create_broken_files, LANGS};
use std::process::{Child, Command};

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct EslintStruct {
    pub settings: Setting,
}

impl ProgramConfig for EslintStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("TypeError") || content.contains("Unexpected linter")
    }
    fn get_only_run_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command
            .arg("eslint")
            .arg(full_name)
            .arg("--config")
            .arg(&self.settings.app_config)
            .arg("--fix");
        command
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        self.get_only_run_command(full_name).spawn().unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::RUST)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
