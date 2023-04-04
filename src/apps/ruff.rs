use crate::broken_files::create_broken_python_files;
use std::process::{Child, Command, Stdio};

use crate::common::{create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RuffStruct {
    pub settings: Setting,
}

impl ProgramConfig for RuffStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE") || content.contains("This indicates a bug in")
    }
    fn validate_output(&self, full_name: String, output: String) -> Option<String> {
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
        let output = lines.into_iter().collect::<String>();

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
        Command::new(&self.settings.app_binary)
            .arg(full_name)
            .arg("--config")
            .arg(&self.settings.app_config)
            .arg("--no-cache")
            .arg("--fix")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn broken_file_creator(&self) -> Child {
        create_broken_python_files(self)
    }

    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
