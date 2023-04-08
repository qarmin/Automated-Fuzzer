use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct QuickLintStruct {
    pub settings: Setting,
}

impl ProgramConfig for QuickLintStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("aborted") || content.contains("internal check failed")
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::JAVASCRIPT)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
