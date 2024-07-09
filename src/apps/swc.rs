use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct SwcStruct {
    pub settings: Setting,
}

const BROKEN_ITEMS_TO_IGNORE: &[&str] = &[];
const BROKEN_ITEMS_TO_FOUND: &[&str] = &["RUST_BACKTRACE"];

impl ProgramConfig for SwcStruct {
    fn is_broken(&self, content: &str) -> bool {
        BROKEN_ITEMS_TO_FOUND.iter().any(|e| content.contains(e))
            && !BROKEN_ITEMS_TO_IGNORE.iter().any(|e| content.contains(e))
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command()
            .arg("compile")
            .arg(full_name)
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
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        CheckGroupFileMode::ByFolder
    }
}
