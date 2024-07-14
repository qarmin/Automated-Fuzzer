use config::Config;
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

use crate::apps::custom::CustomStruct;
use crate::apps::ruff::RuffStruct;
use crate::broken_files::LANGS;
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;

pub const TIMEOUT_MESSAGE: &str = "timeout: sending signal";

#[derive(Clone, Debug)]
pub struct Setting {
    pub loop_number: u32,
    pub broken_files_for_each_file: u32,
    pub minimize_output: bool,
    pub minimization_attempts: u32,
    pub minimization_attempts_with_signal_timeout: u32,
    pub remove_non_crashing_items_from_broken_files: bool,
    pub temp_folder: String,
    pub current_mode: MODES,
    pub extensions: Vec<String>,
    pub broken_files_dir: String,
    pub valid_input_files_dir: String,
    pub temp_possible_broken_files_dir: String,
    pub app_binary: String,
    pub tool_type: String,
    pub app_config: String,
    pub binary_mode: bool,
    pub debug_print_results: bool,
    pub timeout: usize,
    pub allowed_error_statuses: Vec<i32>,
    pub error_when_found_signal: bool,
    pub debug_print_broken_files_creator: bool,
    pub max_collected_files: usize,
    pub find_minimal_rules: bool,
    pub check_if_file_is_parsable: bool,
    pub disable_exceptions: bool,
    pub ignore_timeout_errors: bool,
    pub grouping: u32,
    pub debug_executed_commands: bool,
    pub custom_items: Option<CustomItems>,
}

#[derive(Clone, Debug)]
pub struct CustomItems {
    pub group_mode: CheckGroupFileMode,
    pub command_parts: Vec<String>,
    pub search_items: Vec<String>,
    pub ignored_items: Vec<String>,
    pub file_type: LANGS,
}

pub fn process_custom_struct(general: &HashMap<String, String>, tool_hashmap: &HashMap<String, String>) -> CustomItems {
    let group_mode = match tool_hashmap["group_mode"].as_str() {
        "none" => CheckGroupFileMode::None,
        "by_files" => CheckGroupFileMode::ByFilesGroup,
        "by_group" => CheckGroupFileMode::ByFolder,
        _ => panic!("Invalid group mode {}", tool_hashmap["group_mode"]),
    };

    let timeout_time: u32 = general["timeout"].parse().unwrap();
    let mut command_parts = Vec::new();
    if timeout_time != 0 {
        command_parts.push("timeout".to_string());
        command_parts.push("-v".to_string());
        command_parts.push(timeout_time.to_string());
    }
    if tool_hashmap["command"]
        .split('|')
        .filter_map(|e| {
            let r = e.trim();
            if r.is_empty() {
                None
            } else {
                Some(r)
            }
        })
        .count()
        == 0
        || !tool_hashmap["command"].contains("FILE_PATHS_TO_PROVIDE")
    {
        panic!("No command found in the custom tool or FILE_PATHS_TO_PROVIDE is not found in the command");
    }
    command_parts.extend(tool_hashmap["command"].split('|').map(str::to_string));

    let search_item_keys: Vec<_> = tool_hashmap
        .keys()
        .filter(|e| e.starts_with("search_item_"))
        .cloned()
        .collect();
    let search_items: Vec<_> = search_item_keys.iter().map(|e| tool_hashmap[e].clone()).collect();
    assert!(!search_items.is_empty(), "No search items found in the custom tool");

    let ignored_item_keys: Vec<_> = tool_hashmap
        .keys()
        .filter(|e| e.starts_with("ignored_item_"))
        .cloned()
        .collect();
    let ignored_items = ignored_item_keys.iter().map(|e| tool_hashmap[e].clone()).collect();

    let file_type = match tool_hashmap["file_type"].as_str() {
        "text" => LANGS::TEXT,
        "binary" => LANGS::BINARY,
        "js" => LANGS::JAVASCRIPT,
        "go" => LANGS::GO,
        "rust" => LANGS::RUST,
        "lua" => LANGS::LUA,
        _ => panic!("Invalid file type {}", tool_hashmap["file_type"]),
    };

    CustomItems {
        group_mode,
        command_parts,
        search_items,
        ignored_items,
        file_type,
    }
}

pub fn load_settings() -> Setting {
    let settings = Config::builder()
        .add_source(config::File::with_name("fuzz_settings"))
        .build()
        .unwrap();
    let config = settings
        .try_deserialize::<HashMap<String, HashMap<String, String>>>()
        .unwrap();

    let general = config["general"].clone();
    let current_mode_string = general["current_mode"].clone();
    let current_mode = MODES::from_str(&current_mode_string).unwrap();
    let curr_setting = config[&current_mode_string].clone();

    let extensions = curr_setting["extensions"]
        .split(',')
        .map(str::trim)
        .filter_map(|e| if e.is_empty() { None } else { Some(format!(".{e}")) })
        .collect();
    let find_minimal_rules = general["find_minimal_rules"].parse().unwrap();
    let remove_non_crashing_items_from_broken_files =
        general["remove_non_crashing_items_from_broken_files"].parse().unwrap();
    let disable_exceptions = !(find_minimal_rules || remove_non_crashing_items_from_broken_files);
    let ignore_timeout_errors = general["ignore_timeout_errors"].parse().unwrap();
    let grouping = general["grouping"].parse().unwrap();
    let debug_executed_commands = general["debug_executed_commands"].parse().unwrap();

    let custom_items = if current_mode == MODES::CUSTOM {
        Some(process_custom_struct(&general, &curr_setting))
    } else {
        None
    };

    Setting {
        loop_number: general["loop_number"].parse().unwrap(),
        broken_files_for_each_file: general["broken_files_for_each_file"].parse().unwrap(),
        minimize_output: general["minimize_output"].parse().unwrap(),
        minimization_attempts: general["minimization_attempts"].parse().unwrap(),
        minimization_attempts_with_signal_timeout: general["minimization_attempts_with_signal_timeout"]
            .parse()
            .unwrap(),
        remove_non_crashing_items_from_broken_files,
        current_mode,
        extensions,
        timeout: general["timeout"].parse().unwrap(),
        broken_files_dir: curr_setting["broken_files_dir"].parse().unwrap(),
        valid_input_files_dir: curr_setting["valid_input_files_dir"].parse().unwrap(),
        temp_possible_broken_files_dir: general["temp_possible_broken_files_dir"].parse().unwrap(),
        app_binary: curr_setting["app_binary"].parse().unwrap(),
        app_config: curr_setting["app_config"].parse().unwrap(),
        binary_mode: curr_setting["binary_mode"].parse().unwrap(),
        debug_print_results: general["debug_print_results"].parse().unwrap(),
        allowed_error_statuses: general["allowed_error_statuses"]
            .split(',')
            .map(|e| e.parse().unwrap())
            .collect(),
        error_when_found_signal: general["error_when_found_signal"].parse().unwrap(),
        debug_print_broken_files_creator: general["debug_print_broken_files_creator"].parse().unwrap(),
        max_collected_files: general["max_collected_files"].parse().unwrap(),
        temp_folder: general["temp_folder"].clone(),
        find_minimal_rules,
        tool_type: curr_setting["tool_type"].clone(),
        check_if_file_is_parsable: general["check_if_file_is_parsable"].parse().unwrap(),
        disable_exceptions,
        ignore_timeout_errors,
        grouping,
        debug_executed_commands,
        custom_items,
    }
}

pub fn get_object(settings: Setting) -> Box<dyn ProgramConfig> {
    let custom_items = settings.custom_items.clone();
    match settings.current_mode {
        MODES::RUFF => Box::new(RuffStruct {
            settings,
            ignored_rules: String::new(),
        }),
        MODES::CUSTOM => Box::new(CustomStruct {
            custom_items: custom_items.unwrap(),
            settings,
        }),
    }
}

#[derive(Debug, PartialEq, EnumString, Copy, Clone, Display)]
pub enum MODES {
    #[strum(ascii_case_insensitive)]
    CUSTOM,
    #[strum(ascii_case_insensitive)]
    RUFF,
}
