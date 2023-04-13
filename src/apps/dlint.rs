use crate::broken_files::{create_broken_files, LANGS};
use std::process::{Child, Command, Stdio};

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct DlintStruct {
    pub settings: Setting,
}

impl ProgramConfig for DlintStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE") || content.contains("timeout: sending signal")
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        Command::new(&self.settings.app_binary)
        // Command::new("timeout").arg("-v").arg("20").arg(&self.settings.app_binary) // TODO timeout
            .arg("run")
            .arg(full_name)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::GENERAL)
        } else {
            create_broken_files(self, LANGS::JAVASCRIPT)
        }
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
