use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use std::process::{Child, Command};

pub struct BiomeStruct {
    pub settings: Setting,
}

const BROKEN_ITEMS_TO_IGNORE: &[&str] = &[];
const BROKEN_ITEMS_TO_FOUND: &[&str] = &["RUST_BACKTRACE", "Biome encountered an unexpected error"];

impl ProgramConfig for BiomeStruct {
    fn is_broken(&self, content: &str) -> bool {
        BROKEN_ITEMS_TO_FOUND.iter().any(|e| content.contains(e))
            && !BROKEN_ITEMS_TO_IGNORE.iter().any(|e| content.contains(e))
    }

    fn get_only_run_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.arg("lint").arg(full_name);
        command
    }

    fn get_run_command(&self, full_name: &str) -> Child {
        self.get_only_run_command(full_name).spawn().unwrap()
    }
    fn get_group_files_command(&self, _files: &[String]) -> Child {
        unimplemented!("Biome does not support group files")
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
