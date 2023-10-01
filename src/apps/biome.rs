use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct BiomeStruct {
    pub settings: Setting,
}

const BROKEN_ITEMS_TO_IGNORE: &[&str] = &[
    "begin <= end (1 <= 0) ",         // 4323
    "assertion failed: start <= end", // 4323
    "is not a char boundary",         // 4323
    "is out of bounds of",            // 4323
];

impl ProgramConfig for BiomeStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")
            || content.contains("Biome encountered an unexpected error")
                && !BROKEN_ITEMS_TO_IGNORE.iter().any(|e| content.contains(e))
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command()
            .arg("check")
            .arg(full_name)
            // .arg("--max-diagnostics") // This probably disable diagnostics instead hiding them from output
            // .arg("0")
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
}
