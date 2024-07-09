use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct ZipStruct {
    pub settings: Setting,
}

impl ProgramConfig for ZipStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::GENERAL)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        CheckGroupFileMode::ByFolder
    }
}
