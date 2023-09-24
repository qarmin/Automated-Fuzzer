use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RustFmtStruct {
    pub settings: Setting,
}

impl ProgramConfig for RustFmtStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("stack backtrace") && content.contains("RUST_BACKTRACE") && content.contains("panicked at")
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command()
            .arg("+nightly")
            .arg(full_name)
            .arg("--config-path")
            .arg(&self.get_settings().app_config)
            .spawn()
            .unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::RUST)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
