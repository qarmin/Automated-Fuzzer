use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use std::process::{Child, Command};

pub struct SwcStruct {
    pub settings: Setting,
}

impl ProgramConfig for SwcStruct {
    fn is_broken(&self, content: &str) -> bool {
        ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x)) && !self.ignored_signal_output(&content)
    }

    fn ignored_signal_output(&self, content: &str) -> bool {
        content.contains("crates/swc_ecma_compat_es2015/src/destructuring.rs:1165:44") ||
            content.contains("crates/swc_common/src/source_map.rs:662:60") ||
            content.contains("crates/swc_ecma_compat_es2021/src/logical_assignments.rs:155:50") ||
            content.contains("crates/swc_ecma_compat_es2015/src/destructuring.rs:409:52") ||
            content.contains("crates/swc_ecma_compat_es2022/src/class_properties/mod.rs:959:21") ||
            content.contains("crates/swc_ecma_transforms_typescript/src/transform.rs:265:18") ||
            content.contains("timeout: the monitored command dumped core")
    }
    fn get_only_run_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.arg("compile").arg(full_name);
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
