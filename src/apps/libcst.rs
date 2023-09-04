use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct LibCSTStruct {
    pub settings: Setting,
}

impl ProgramConfig for LibCSTStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("stack overflow") || content.contains("RUST_BACKTRACE")
    }

    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::GENERAL)
        } else {
            create_broken_files(self, LANGS::PYTHON)
        }
    }

    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
