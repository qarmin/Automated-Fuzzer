use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct ImageStruct {
    pub settings: Setting,
}

impl ProgramConfig for ImageStruct {
    fn is_broken(&self, content: &str) -> bool {
        ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x)) && !self.ignored_signal_output(&content) // TODO, this is already reported
    }
    fn ignored_signal_output(&self, content: &str) -> bool {
        content.contains("zune-jpeg-0.4.11/src/mcu.rs:209:9") || content.contains("src/codecs/tiff.rs:247:21") || content.contains("src/compression/piz/huffman.rs")
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::GENERAL)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        CheckGroupFileMode::None // It is possible that images will use to much memory, also this operation is very heavy, so gain with testing in groups is minimal
    }
}
