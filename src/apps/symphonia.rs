use crate::broken_files::create_broken_general_files;
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct SymphoniaStruct {
    pub settings: Setting,
}

impl ProgramConfig for SymphoniaStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")// && !content.contains("codec_ima.rs")
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_general_files(self)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
