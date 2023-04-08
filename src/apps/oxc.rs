use crate::broken_files::{create_broken_files,  LANGS};
use std::process::{Child, Command, Stdio};

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct OxcStruct {
    pub settings: Setting,
}

impl ProgramConfig for OxcStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        Command::new(&self.settings.app_binary)
            .arg("lint")
            .arg("-A")
            .arg("all")
            .arg(full_name)
            .arg("--fix")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::JAVASCRIPT)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
