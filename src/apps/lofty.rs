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
        let contains_rust_backtrace = ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x));
        let contains_aiff = content.contains("iff/aiff/properties.rs");
        let contains_mp4 = content.contains("mp4/ilst/read.rs");
        let contains_atom_info = content.contains("src/mp4/atom_info.rs");

        contains_rust_backtrace && !contains_aiff && !contains_mp4 && !contains_atom_info
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
