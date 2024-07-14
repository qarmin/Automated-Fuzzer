use jwalk::WalkDir;
use log::{error, info};
use rand::Rng;
use rayon::prelude::*;
use std::fs;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicU32;

use crate::broken_files::{create_broken_files, LANGS};
use crate::common::{
    collect_output, create_new_file_name, find_broken_files_by_cpython, run_ruff_format_check, try_to_save_file,
    CheckGroupFileMode,
};
use crate::obj::ProgramConfig;
use crate::settings::Setting;

pub struct RuffStruct {
    pub settings: Setting,
    pub ignored_rules: String,
}

const DISABLE_EXCEPTIONS: bool = false;

// This errors are not critical, and can be ignored when found two issues and one with it
const BROKEN_ITEMS_NOT_CRITICAL: &[&str] = &[
    "into scope due to name conflict", // Expected, name conflict cannot really be fixed automatically
    "Failed to create fix for UnnecessaryLiteralDict", // Mostly expected 7455
    "ReimplementedStarmap",            // Mostly expected 7455
    "due to late binding",             // Mostly expected 6842
    "UnnecessaryCollectionCall",       // 6809
    "error: Failed to create fix for FormatLiterals: Unable to identify format literals", // 6717 - UP030
    "Unable to use existing symbol due to incompatible context", // 6842
];

// Try to not add D* rules if you are not really sure that this rule is broken
// With this rule here, results can be invalid
const BROKEN_ITEMS_TO_IGNORE: &[&str] = &[];

const BROKEN_ITEMS_TO_FIND: &[&str] = &[
    "std::rt::lang_start_internal", "catch_unwind::{{closure}}", "stack backtrace:",
    "0: rust_begin_unwind",
    // "AddressSanitizer:",
    // "LeakSanitizer:",
    // "Failed to create fix", // Do not report that, probably not worth to fix
    // "Fix introduced a syntax error", "Fix introduced a syntax error", "This indicates a bug in",
];

const INVALID_RULES: &[&str] = &[
    // "FURB171", // 8402
    // "E223",    // 8402
    // "RUF015",  // 8402
    // "C405",    // 8402
    // "RUF022",  // 8402
    // "RUF023",  // 8402
    // // "PLR1706", // 8402
    // "C413", // 8402
    //
    "E999",                          // Cannot use with preview
    "PGH001",                        // Remapped
    "PGH002",                        // Remapped
    "RUF011",                        // Remapped
    "TRY200",                        // Remapped
    "one-blank-line-before-class",   // incompatible with "no-blank-line-before-class"
    "multi-line-summary-first-line", // incompatible with "multi-line-summary-second-line"
];

#[must_use]
pub fn calculate_ignored_rules() -> String {
    if DISABLE_EXCEPTIONS {
        return String::new();
    }

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
        // Fake errors
        if content.contains(r#""stack backtrace:\n""#) {
            return false;
        }
        if DISABLE_EXCEPTIONS || self.settings.disable_exceptions {
            return BROKEN_ITEMS_TO_FIND
                .iter()
                .filter(|line| !BROKEN_ITEMS_NOT_CRITICAL.iter().any(|e2| line.contains(e2)))
                .any(|e| content.contains(e));
        }

        let mut content = content.to_string();
        #[allow(clippy::const_is_empty)]
        if !BROKEN_ITEMS_NOT_CRITICAL.is_empty() {
            // Remove lines that contains not critical errors
            content = content
                .lines()
                .filter(|line| !BROKEN_ITEMS_NOT_CRITICAL.iter().any(|e2| line.contains(e2)))
                .collect::<Vec<&str>>()
                .join("\n")
                .to_string();
        }

        let found_broken_items = BROKEN_ITEMS_TO_FIND.iter().any(|e| content.contains(e));

        let found_ignored_item = BROKEN_ITEMS_TO_IGNORE.iter().any(|e| content.contains(e));

        // Debug check if properly finding broken items
        // dbg!(
        //     BROKEN_ITEMS_TO_IGNORE.iter().find(|e| content.contains(*e)),
        //     found_broken_items
        // );
        found_broken_items && !found_ignored_item
    }

    // fn remove_not_needed_lines_from_output(&self, output: String) -> String {
    //     output
    //         .lines()
    //         .filter(|e| {
    //             !((e.contains(".py") && e.matches(':').count() >= 3)
    //                 || e.trim().is_empty()
    //                 || e.starts_with("warning: `")
    //                 || e.starts_with("Found: `")
    //                 || e.starts_with("Found ")
    //                 || e.starts_with("Ignoring `"))
    //         })
    //         .map(String::from)
    //         .collect::<Vec<String>>()
    //         .join("\n")
    // }

    fn validate_output_and_save_file(&self, full_name: String, output: &str) -> Option<String> {
        let mut lines = output
            .lines()
            .filter(|e| {
                !((e.contains(".py") && e.matches(':').count() >= 3)
                    || e.trim().is_empty()
                    || e.starts_with("warning: `")
                    || e.starts_with("Found: `")
                    || e.starts_with("Found ")
                    || e.starts_with("Ignoring `"))
            })
            .map(String::from)
            .collect::<Vec<String>>();
        lines.dedup();
        // Lines contains info about
        let lines = lines.into_iter().take_while(|e| e != "---").collect::<Vec<_>>();
        let output = lines.join("\n");

        let new_name = create_new_file_name(self.get_settings(), &full_name);
        let new_name_not_minimized = create_new_file_name(self.get_settings(), &full_name);
        error!("File {full_name} saved to {new_name}\n{output}");

        if try_to_save_file(self.get_settings(), &full_name, &new_name) {
            let _ = try_to_save_file(self.get_settings(), &full_name, &new_name_not_minimized);
            Some(new_name)
        } else {
            None
        }
    }
    fn get_full_command(&self, full_name: &str) -> Command {
        let mut command = self._get_basic_run_command();

        match self.settings.tool_type.as_str() {
            "lint_check_fix" => {
                command
                    .arg("check")
                    .arg(full_name)
                    .arg("--select")
                    .arg("ALL")
                    .arg("--preview")
                    .arg("--output-format")
                    .arg("concise")
                    .arg("--no-cache")
                    .arg("--fix")
                    .arg("--unsafe-fixes");

                if !self.get_settings().app_config.is_empty() {
                    command.arg("--config").arg(&self.get_settings().app_config);
                } else {
                    command.arg("--isolated");
                }

                if !self.ignored_rules.is_empty() {
                    command.arg("--ignore").arg(&self.ignored_rules);
                }
            }
            "lint_check" => {
                command
                    .arg("check")
                    .arg(full_name)
                    .arg("--select")
                    .arg("ALL")
                    .arg("--preview")
                    .arg("--output-format")
                    .arg("concise")
                    .arg("--no-cache");
                if !self.ignored_rules.is_empty() {
                    command.arg("--ignore").arg(&self.ignored_rules);
                }
            }
            "format" => {
                command.arg("format").arg(full_name).arg("--check");
            }
            "red_knot" => {
                command.arg(full_name);
            }
            _ => {
                panic!("Unknown tool type: {}", self.settings.tool_type);
            }
        }

        if self.settings.debug_executed_commands {
            info!("Executing command: {:?}", command);
        }
        command
    }
    fn run_command(&self, full_name: &str) -> Child {
        self.get_full_command(full_name).spawn().unwrap()
    }

    fn broken_file_creator(&self) -> Child {
        if self.settings.binary_mode {
            create_broken_files(self, LANGS::BINARY)
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

    fn is_parsable(&self, file_to_check: &str) -> bool {
        if !self.settings.check_if_file_is_parsable {
            return true;
        }
        let output = Command::new("ruff")
            .arg("format")
            .arg(file_to_check)
            .arg("--check")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        let out = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        !(out.contains("error: Failed to format ") || err.contains("error: Failed to format "))
    }

    fn get_version(&self) -> String {
        let output = Command::new("ruff")
            .arg("version")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        let out = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        if !out.is_empty() {
            return out.to_string().trim().to_string();
        }
        if !err.is_empty() {
            return err.to_string().trim().to_string();
        }
        String::new()
    }

    fn remove_non_parsable_files(&self, dir_to_check: &str) {
        if !self.settings.check_if_file_is_parsable {
            return;
        }
        let files_to_check: Vec<_> = WalkDir::new(dir_to_check)
            .into_iter()
            .flatten()
            .filter(|e| e.path().to_str().unwrap().to_lowercase().ends_with(".py"))
            .collect();
        let all_files = files_to_check.len();

        let folders: Vec<_> = files_to_check
            .par_chunks(1000)
            .map(|files| {
                let mut rng = rand::thread_rng();
                let mut folder_name;
                loop {
                    folder_name = format!("{}/FDR_{}", dir_to_check, rng.gen::<u64>());
                    if fs::create_dir_all(&folder_name).is_ok() {
                        break;
                    }
                }
                for file in files {
                    let rand_new_name = format!("{}/F_NAME_{}.py", folder_name, rng.gen::<u64>());
                    if let Err(e) = fs::rename(file.path(), &rand_new_name) {
                        info!("Failed to move file: {:?} with error: {}", file.path(), e);
                        fs::remove_file(file.path()).unwrap();
                    }
                }
                folder_name
            })
            .collect();

        let files_to_remove_ruff = AtomicU32::new(0);
        folders.par_iter().for_each(|folder_name| {
            let output = run_ruff_format_check(folder_name, false);

            let all = collect_output(&output);
            for i in all.lines() {
                if let Some(s) = i.strip_prefix("error: Failed to parse ") {
                    if let Some(idx) = s.find(".py") {
                        let file_name = &s[..idx + 3];
                        if file_name.contains(' ') {
                            continue;
                        }
                        files_to_remove_ruff.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        fs::remove_file(file_name).unwrap();
                    }
                }
            }
        });

        let files_to_remove_cpython = AtomicU32::new(0);
        folders.par_iter().for_each(|folder_name| {
            let not_parsable_files = find_broken_files_by_cpython(folder_name);
            for file_name in &not_parsable_files {
                fs::remove_file(file_name).unwrap();
            }
            files_to_remove_cpython.fetch_add(not_parsable_files.len() as u32, std::sync::atomic::Ordering::Relaxed);
        });

        let files_to_remove_cpython: u32 = files_to_remove_cpython.load(std::sync::atomic::Ordering::Relaxed);
        let files_to_remove_ruff: u32 = files_to_remove_ruff.load(std::sync::atomic::Ordering::Relaxed);

        info!(
            "Removed {}/{all_files} non parsable files - first {files_to_remove_ruff} by ruff, later {files_to_remove_cpython} by cpython",
            files_to_remove_ruff + files_to_remove_cpython,

        );
    }

    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        match self.settings.tool_type.as_str() {
            "lint_check_fix" | "lint_check" | "format" => CheckGroupFileMode::ByFolder,
            "red_knot" => CheckGroupFileMode::None,
            _ => CheckGroupFileMode::None,
        }
    }
}
