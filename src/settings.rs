use config::Config;
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Clone, Debug)]
pub struct Setting {
    pub loop_number: u32,
    pub broken_files_for_each_file: u32,
    pub copy_broken_files: bool,
    pub generate_files: bool,
    pub minimize_output: bool,
    pub minimization_attempts: u32,
    pub current_mode: MODES,
    pub extensions: Vec<String>,
    pub output_dir: String,
    pub base_of_valid_files: String,
    pub input_dir: String,
    pub app_binary: String,
    pub app_config: String,
    pub binary_mode: bool,
    pub debug_print_results: bool,
    pub debug_print_broken_files_creator: bool,
    pub safe_run: bool,
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

    let copy_broken_files = general["copy_broken_files"].parse().unwrap();
    Setting {
        loop_number: general["loop_number"].parse().unwrap(),
        broken_files_for_each_file: general["broken_files_for_each_file"].parse().unwrap(),
        copy_broken_files,
        generate_files: general["generate_files"].parse().unwrap(),
        minimize_output: general["minimize_output"].parse().unwrap(),
        minimization_attempts: general["minimization_attempts"].parse().unwrap(),
        current_mode,
        extensions: curr_setting["extensions"]
            .split(',')
            .map(str::trim)
            .filter_map(|e| {
                if e.is_empty() {
                    None
                } else {
                    Some(format!(".{e}"))
                }
            })
            .collect(),
        output_dir: curr_setting["output_dir"].parse().unwrap(),
        base_of_valid_files: curr_setting["base_of_valid_files"].parse().unwrap(),
        input_dir: general["broken_files_dir"].parse().unwrap(),
        app_binary: curr_setting["app_binary"].parse().unwrap(),
        app_config: curr_setting["app_config"].parse().unwrap(),
        binary_mode: curr_setting["binary_mode"].parse().unwrap(),
        debug_print_results: general["debug_print_results"].parse().unwrap(),
        debug_print_broken_files_creator: general["debug_print_broken_files_creator"].parse().unwrap(),
        safe_run: general["safe_run"].parse().unwrap(),
    }
}

#[derive(Debug, PartialEq, EnumString, Copy, Clone)]
pub enum MODES {
    #[strum(ascii_case_insensitive)]
    RUFF,
    #[strum(ascii_case_insensitive)]
    MYPY,
    #[strum(ascii_case_insensitive)]
    ROME,
    #[strum(ascii_case_insensitive)]
    DLINT,
    #[strum(ascii_case_insensitive)]
    OXC,
    #[strum(ascii_case_insensitive)]
    IMAGE,
    #[strum(ascii_case_insensitive)]
    LOFTY,
    #[strum(ascii_case_insensitive)]
    SYMPHONIA,
}