use crate::broken_files::{create_broken_files, LANGS};
use std::process::{Child, Command};

use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct DlintStruct {
    pub settings: Setting,
}

const BROKEN_ITEMS_TO_IGNORE: &[&str] = &[
    "Parser should not call bump()",                  // 1145
    "on a `None` value', src/rules/getter_return.rs", // 1145
    "on a `None` value', src/js_regex/validator.rs",  // 1145
];

impl ProgramConfig for DlintStruct {
    fn is_broken(&self, content: &str) -> bool {
        ["RUST_BACKTRACE", "panicked at"].iter().any(|&x| content.contains(x))
            && !BROKEN_ITEMS_TO_IGNORE.iter().any(|e| content.contains(e))
    }

    fn get_full_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();
        command.arg("run").arg(full_name);
        command
    }
    fn run_command(&self, full_name: &str) -> Child {
        self.get_full_command(full_name).spawn().unwrap()
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
}
