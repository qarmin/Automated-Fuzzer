use std::process::{Child, Command};

use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct StaticCheckGoStruct {
    pub settings: Setting,
}

impl ProgramConfig for StaticCheckGoStruct {
    fn is_broken(&self, content: &str) -> bool {
        let contains_internal_compiler_error = content.contains("internal compiler error:"); // TODO - probably should be reported to golang
        let contains_internal_error = content.contains("internal error:") && !content.contains("\"internal error");
        let contains_panic =
            content.contains("panic:") && !content.contains("\"panic:") && !content.contains("panic: %v");
        let contains_fatal_error = content.contains("fatal error:") && !content.contains("No such file or directory");

        let is_stack_overflow = content.contains("goroutine stack exceeds"); // TODO for https://github.com/dominikh/go-tools/issues/310

        !contains_internal_compiler_error
            && !is_stack_overflow
            && (contains_internal_error || contains_panic || contains_fatal_error)
    }
    fn get_full_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command
            .arg("-checks")
            .arg("ALL")
            .arg(full_name)
            .arg(&self.get_settings().app_config);
        command
    }
    fn run_command(&self, full_name: &str) -> Child {
        self.get_full_command(full_name).spawn().unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::BINARY)
        } else {
            create_broken_files(self, LANGS::GO)
        }
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        // Looks that when checking multiple files, go try to find some modules
        CheckGroupFileMode::None
        // CheckGroupFileMode::ByFilesGroup
    }
}
