use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct SeleneStruct {
    pub settings: Setting,
}

impl ProgramConfig for SeleneStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::LUA)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
