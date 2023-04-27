use crate::broken_files::{create_broken_files, LANGS};
use std::process::Child;

use crate::common::{create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RuffStruct {
    pub settings: Setting,
}

impl ProgramConfig for RuffStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("Failed to create fix")
            || content.contains("RUST_BACKTRACE")
            || content.contains("This indicates a bug in")
    }
    fn validate_output_and_save_file(&self, full_name: String, output: String) -> Option<String> {
        let mut lines = output
            .lines()
            .filter(|e| {
                !((e.contains(".py") && e.matches(':').count() >= 3)
                    || e.starts_with("warning: `")
                    || e.starts_with("Ignoring `"))
            })
            .map(String::from)
            .collect::<Vec<String>>();
        lines.dedup();
        let output = lines.join("\n");

        let new_name = create_new_file_name(self.get_settings(), &full_name);
        println!("\n_______________ File {full_name} saved to {new_name} _______________________");
        println!("{output}");

        if try_to_save_file(self.get_settings(), &full_name, &new_name) {
            Some(new_name)
        } else {
            None
        }
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        self._get_basic_run_command()
            .arg(full_name)
            .arg("--config")
            .arg(&self.settings.app_config)
            .arg("--no-cache")
            .arg("--fix")
            .spawn()
            .unwrap()
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
