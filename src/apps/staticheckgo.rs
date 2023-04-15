use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct StaticCheckGoStruct {
    pub settings: Setting,
}

impl ProgramConfig for StaticCheckGoStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("internal error")
            || content.contains("panic:")
            || content.contains("fatal error:")
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command()
            .env("PATH", "/usr/local/go/bin")
            .arg("-checks")
            .arg("ALL")
            .arg(full_name)
            .spawn()
            .unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::GENERAL)
        } else {
            create_broken_files(self, LANGS::GO)
        }
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
