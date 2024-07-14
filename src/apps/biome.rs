use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use std::process::{Child, Command};

pub struct BiomeStruct {
    pub settings: Setting,
}

impl ProgramConfig for BiomeStruct {
    fn is_broken(&self, content: &str) -> bool {
        ["RUST_BACKTRACE", "panicked at", "Biome encountered an unexpected error"]
            .iter()
            .any(|&x| content.contains(x))
    }

    fn get_full_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.arg("lint").arg(full_name);
        command
    }

    fn run_command(&self, full_name: &str) -> Child {
        self.get_full_command(full_name).spawn().unwrap()
    }
    fn run_group_command(&self, _files: &[String]) -> Child {
        unimplemented!("Biome does not support group files")
    }
    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::BINARY)
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
