use crate::broken_files::create_broken_javascript_files;
use std::process::{Child, Command, Stdio};

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RomeStruct {
    pub settings: Setting,
}

impl ProgramConfig for RomeStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")
            || content.contains("Rome encountered an unexpected error")
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        Command::new(&self.settings.app_binary)
            .arg("check")
            .arg(full_name)
            // .arg("--max-diagnostics") // This probably disable diagnostics instead hiding them from output
            // .arg("0")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_javascript_files(self)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
