use crate::broken_files::{create_broken_files, LANGS};
use std::process::{Child, Command};

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct MypyStruct {
    pub settings: Setting,
}

impl ProgramConfig for MypyStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("INTERNAL ERROR") || content.contains("Traceback")
    }
    fn get_only_run_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.arg(full_name).args("--no-incremental --ignore-missing-imports --disallow-any-unimported --disallow-any-expr --disallow-any-decorated --disallow-any-explicit --disallow-any-generics --disallow-subclassing-any --disallow-untyped-calls --disallow-untyped-defs --disallow-incomplete-defs --check-untyped-defs --disallow-untyped-decorators --warn-redundant-casts --warn-unused-ignores --no-warn-no-return --warn-return-any --warn-unreachable --strict".split(' '))
        ;
        command
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        self.get_only_run_command(full_name).spawn().unwrap()
    }
    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::GENERAL)
        } else {
            create_broken_files(self, LANGS::PYTHON)
        }
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
