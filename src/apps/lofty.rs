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
        ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x)) && !self.ignored_signal_output(&content)
    }

    fn ignored_signal_output(&self, content: &str) -> bool {
        content.contains("iff/aiff/properties.rs")
            || content.contains("mp4/ilst/read.rs")
            || content.contains("src/mp4/atom_info.rs")
            || content.contains("lofty/src/iff/wav/read.rs")
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::BINARY)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        CheckGroupFileMode::ByFolder
    }
}
