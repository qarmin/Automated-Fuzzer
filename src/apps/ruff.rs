use std::process::Child;

use crate::broken_files::{create_broken_files, LANGS};
use crate::common::{create_new_file_name, try_to_save_file};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RuffStruct {
    pub settings: Setting,
    pub ignored_rules: String,
}

// This errors are not critical, and can be ignored when found two issues and one with it
const BROKEN_ITEMS_NOT_CRITICAL: &[&str] = &[
    "into scope due to name conflict", // Expected, name conflict cannot really be fixed automatically
    "UnnecessaryCollectionCall",       // 6809
    "due to late binding",             // 6842
    "error: Failed to create fix for FormatLiterals: Unable to identify format literals", // 6717
    "Unable to use existing symbol due to incompatible context", // 7060
    "UnnecessaryMap: Expected tuple for dict comprehension", // 7071
];

// Try to not add D* rules if you are not really sure that this rule is broken
// With this rule here, results can be invalid
const BROKEN_ITEMS: &[&str] = &[
    "crates/ruff_source_file/src/line_index.rs", // 4406
    "Failed to extract expression from source",  // 6809 - probably rust python-parser problem
    "ruff_python_parser::string::StringParser::parse_fstring", // 6831
    "locator.rs",                                // 7058
];
const INVALID_RULES: &[&str] = &[
    "W292",    // 4406
    "RUF001",  // 4519
    "F601",    // 4897
    "Q002",    // 6785
    "PTH116",  // 6785
    "ICN001",  // 6786
    "EM101",   // 6811
    "ERA001",  // 6831
    "F632",    // 6891
    "E712",    // 6891
    "NPY001",  // 6952
    "ANN401",  // 6987
    "W605",    // 6987
    "EM102",   // 6988
    "E231",    // 6890
    "E202",    // 6890
    "D209",    // 7058
    "E203",    // 7070
    "E231",    // 7070
    "UP032",   // 7074
    "RET503",  // 7075
    "SIM210",  // 7076
    "NPY003",  // 7077
    "C402",    // 7086
    "C404",    // 7087
    "D204",    // 7088
    "FURB113", // 7095
    "PERF102", // 7097
    "UP024",   // 7101
    "UP037",   // 7102
    "B013",    // 7120
    "C417",    // 7121
    "COM812",  // 7122
    "PT014",   // 7122
    "SIM105",  // 7123
    "SIM118",  // 7124
    "SIM300",  // 7125
    "SIM118",  // 7126
    "UP022",   // 7127
    "SIM222",  // 7128
];

#[must_use]
pub fn calculate_ignored_rules() -> String {
    INVALID_RULES
        .iter()
        .filter(|e| &&e.to_uppercase().as_str() == e)
        .collect::<Vec<_>>()
        .iter()
        .map(|&&s| s)
        .collect::<Vec<_>>()
        .join(",")
}

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
        let mut command = self._get_basic_run_command();
        command
            .arg("check")
            .arg(full_name)
            .arg("--select")
            .arg("ALL,NURSERY")
            .arg("--no-cache")
            .arg("--fix");
        if !self.ignored_rules.is_empty() {
            command.arg("--ignore").arg(&self.ignored_rules);
        }
        command.spawn().unwrap()

        // .arg("--config")
        // .arg(&self.settings.app_config) // For now config is not

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

    fn init(&mut self) {
        self.ignored_rules = calculate_ignored_rules();
    }
}
