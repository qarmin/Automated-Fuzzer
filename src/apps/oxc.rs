use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use std::process::{Child, Command};

pub struct OxcStruct {
    pub settings: Setting,
}

impl ProgramConfig for OxcStruct {
    fn is_broken(&self, content: &str) -> bool {
        ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x)) && !self.ignored_signal_output(&content)
    }
    fn ignored_signal_output(&self, content: &str) -> bool {
        content.contains("crates/oxc_linter/src/utils/tree_shaking.rs:424:41") ||
            content.contains("fatal runtime error: stack overflow") ||
            content.contains("crates/oxc_linter/src/utils/tree_shaking.rs:450:41") ||
            content.contains("crates/oxc_span/src/span.rs:102:9") ||
            content.contains("crates/oxc_linter/src/fixer/mod.rs:200:37") ||
    }
    fn get_only_run_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command
            .arg("-D")
            .arg("all")
            .arg("-D")
            .arg("nursery")
            .arg(full_name)
            .arg("--fix");
        command
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        self.get_only_run_command(full_name).spawn().unwrap()
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
