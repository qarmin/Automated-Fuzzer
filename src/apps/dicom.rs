use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct DicomStruct {
    pub settings: Setting,
}

const BROKEN_ITEMS_TO_IGNORE: &[&str] = &[];
const BROKEN_ITEMS_TO_FOUND: &[&str] = &["RUST_BACKTRACE"];

impl ProgramConfig for DicomStruct {
    fn is_broken(&self, content: &str) -> bool {
        BROKEN_ITEMS_TO_FOUND.iter().any(|e| content.contains(e))
            && !BROKEN_ITEMS_TO_IGNORE.iter().any(|e| content.contains(e))
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command().arg(full_name).spawn().unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::GENERAL)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
