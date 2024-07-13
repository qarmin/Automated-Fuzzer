use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use std::process::Child;

pub struct RustBuzzStruct {
    pub settings: Setting,
}

impl ProgramConfig for RustBuzzStruct {
    fn is_broken(&self, content: &str) -> bool {
        ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x)) && !self.ignored_signal_output(&content)
    }
    fn ignored_signal_output(&self, content: &str) -> bool {
        content.contains("ttf-parser-0.24.0/src/lib.rs:351:9") ||
            content.contains("src/hb/set_digest.rs:93") ||
            content.contains("src/hb/ot_layout_gsubgpos.rs:610:67") ||
            content.contains("src/hb/ot_layout_gsubgpos.rs:451:57") ||
            content.contains("src/hb/ot_layout_gsubgpos.rs:615:63") ||
            content.contains("src/hb/ot_layout_gsubgpos.rs:605:67") ||
            content.contains("ttf-parser-0.24.0/src/lib.rs:351:9") ||
            content.contains("src/hb/face.rs:331:42") ||
            content.contains("src/hb/set_digest.rs:93:12") ||
            content.contains("ttf-parser-0.24.0/src/var_store.rs:144:49") ||
            content.contains("ttf-parser-0.24.0/src/tables/gpos.rs:97:34") ||
            content.contains("src/hb/buffer.rs:1236:22") ||
            content.contains("src/hb/buffer.rs:1187:43")
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
