use std::process::Child;

use crate::broken_files::{create_broken_files, LANGS};
use crate::common::{create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RuffStruct {
    pub settings: Setting,
}

const BROKEN_ITEMS: &[&str] = &[
    "Failed to create fix for ImplicitOptional", // Probably expected
    "out of bounds",                             // 4406
    "is not a char boundary",                    // 4406
    "error: Failed to create fix for FormatLiterals: Unable to identify format literals", // 6717
    "Failed to create fix for UnnecessaryMap: Currently not supporting default values", // 6715
    "W292",                                      // 4406
    "UP009",                                     // 6756
    "Q002",                                      // 6785
    "Q000",                                      // 6785
    "ICN001",                                    // 6786
    "end_of_last_statement",                     // 6787
    "PT009",                                     // 6788
    "SIM300",                                    // 6788
    "UP018",                                     // 6788
];

impl ProgramConfig for RuffStruct {
    fn is_broken(&self, content: &str) -> bool {
        let found_broken_items = content.contains("Failed to create fix")
            || content.contains("RUST_BACKTRACE")
            || content.contains("catch_unwind::{{closure}}")
            || content.contains("This indicates a bug in")
            || content.contains("Autofix introduced a syntax error");
        // Debug check if properly
        // dbg!(
        //     BROKEN_ITEMS.iter().find(|e| content.contains(*e)),
        //     found_broken_items
        // );
        found_broken_items && !BROKEN_ITEMS.iter().any(|e| content.contains(e))
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
            // .arg("--config")
            // .arg(&self.settings.app_config)
            .arg("--select")
            // .arg("ALL,NURSERY")
            // .arg("NURSERY")
            .arg("ALL") // Nursery enable after fixing bugs related to it
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
