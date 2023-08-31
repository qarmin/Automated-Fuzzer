use std::process::Child;

use crate::broken_files::{create_broken_files, LANGS};
use crate::common::{create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RuffStruct {
    pub settings: Setting,
}

// This errors are not critical, and can be ignored when found two issues and one with it
const BROKEN_ITEMS_NOT_CRITICAL: &[&str] = &[
    "into scope due to name conflict", // Expected, name conflict cannot really be fixed automatically
    "UnnecessaryCollectionCall",       // 6809
    "due to late binding",             // 6842
    "error: Failed to create fix for FormatLiterals: Unable to identify format literals", // 6717
];

// Try to not add D* rules if you are not really sure that this rule is broken
// With this rule here, results can be invalid
const BROKEN_ITEMS: &[&str] = &[
    "crates/ruff_source_file/src/line_index.rs", // 4406
    "Failed to extract expression from source",  // 6809 - probably rust python-parser problem
    "W292",                                      // 4406
    "Q002",                                      // 6785
    "Q000",                                      // 6785
    "ICN001",                                    // 6786
    "EM101",                                     // 6811
    "F401",                                      // 6811
    "CPY001",                                    // 6890
    "E202",                                      // 6890
    "E702",                                      // 6890
    "E999",                                      // 6890
    "F632",                                      // 6891
    "F821",                                      // 6891
    "PLR0133",                                   // 6891
    "RUF001",                                    // 4519
    "ANN001",                                    // 6952
    "ANN201",                                    // 6952
    "ARG001",                                    // 6952
    "CPY001",                                    // 6952
    "NPY001",                                    // 6952
    "E201",                                      // 6987
    "E203",                                      // 6987
    "I001",                                      // 6987
    "UP025",                                     // 6987
    "W291",                                      // 6987
    "W605",                                      // 6987
    "EM102",                                     // 6988
    "B009",                                      // 6989
    "ruff_source_file/src/locator.rs",           // 7012
];

impl ProgramConfig for RuffStruct {
    fn is_broken(&self, content: &str) -> bool {
        let mut content = content.to_string();
        if !BROKEN_ITEMS_NOT_CRITICAL.is_empty() {
            // Remove lines that contains not critical errors
            content = content
                .lines()
                .filter(|line| !BROKEN_ITEMS_NOT_CRITICAL.iter().any(|e2| line.contains(e2)))
                .collect::<Vec<&str>>()
                .join("\n")
                .to_string();
        }

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
        // .arg("--config")
        // .arg(&self.settings.app_config) // For now config is not

        self._get_basic_run_command()
            .arg("check")
            .arg(full_name)
            .arg("--select")
            ////////////////////////////////////////
            .arg("ALL,NURSERY")
            // .arg("NURSERY")
            // .arg("ALL") // Nursery enable after fixing bugs related to it
            ////////////////////////////////////////
            .arg("--no-cache")
            .arg("--fix")
            .spawn()
            .unwrap()

        // self._get_basic_run_command()
        //     .arg("format")
        //     .arg(full_name)
        //     .spawn()
        //     .unwrap()
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
