use std::process::{Child, Command, Stdio};

use crate::common::{create_broken_python_files, create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::settings::Setting;


pub struct MypyStruct {
    pub settings: Setting,
}

impl ProgramConfig for MypyStruct {
    fn is_broken(&self, content: &str) -> bool {
        content.contains("INTERNAL ERROR") || content.contains("Traceback")
    }
    fn get_run_command(&self, full_name: &str) -> Child {
        Command::new(&self.settings.app_binary)
            .arg(full_name)
            .args("--no-incremental --ignore-missing-imports --disallow-any-unimported --disallow-any-expr --disallow-any-decorated --disallow-any-explicit --disallow-any-generics --disallow-subclassing-any --disallow-untyped-calls --disallow-untyped-defs --disallow-incomplete-defs --check-untyped-defs --disallow-untyped-decorators --warn-redundant-casts --warn-unused-ignores --no-warn-no-return --warn-return-any --warn-unreachable --strict".split(' '))
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn broken_file_creator(&self) -> Child {
        create_broken_python_files(self)
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
