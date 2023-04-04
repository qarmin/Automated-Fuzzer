use std::process::{Child, Command, Stdio};

use crate::common::{create_broken_javascript_files, create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::Setting;

pub struct DlintStruct {
    pub settings: Setting,
}

impl ProgramConfig for DlintStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("RUST_BACKTRACE")
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        Command::new(&self.settings.app_binary)
            .arg("run")
            .arg(full_name)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn broken_file_creator(&self) -> Child {
        create_broken_javascript_files(self)
    }
    fn validate_output(&self, full_name: String, output: String) -> Option<String> {
        let new_name = create_new_file_name(self,&full_name);
        println!("\n_______________ File {full_name} saved to {new_name} _______________________");
        println!("{output}");

        if try_to_save_file(self,&full_name, &new_name) {
            Some(new_name)
        } else {
            None
        }
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
}
