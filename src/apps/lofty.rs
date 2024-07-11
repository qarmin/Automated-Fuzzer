use std::process::Child;

use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct LoftyStruct {
    pub settings: Setting,
}

impl ProgramConfig for LoftyStruct {
    fn is_broken(&self, content: &str) -> bool {
        let contains_rust_backtrace = content.contains("RUST_BACKTRACE");
        let contains_memory_allocation_failure = content.contains("memory allocation of");

        contains_rust_backtrace && !contains_memory_allocation_failure
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
