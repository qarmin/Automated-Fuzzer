use std::collections::HashMap;
use config::Config;
use strum_macros::EnumString;
use std::str::FromStr;

#[derive(Clone)]
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
    pub app_config: String
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
    let broken_files_dir: String = general["broken_files_dir"].parse().unwrap();
    let non_destructive_input_dir: String = curr_setting["non_destructive_input_dir"].parse().unwrap();
    let input_dir = if copy_broken_files {
        broken_files_dir
    } else {
        non_destructive_input_dir
    };
    Setting {
        loop_number: general["loop_number"].parse().unwrap(),
        broken_files_for_each_file: general["broken_files_for_each_file"].parse().unwrap(),
        copy_broken_files,
        generate_files: general["generate_files"].parse().unwrap(),
        minimize_output: general["minimize_output"].parse().unwrap(),
        minimization_attempts: general["minimization_attempts"].parse().unwrap(),
        current_mode,
        extensions: curr_setting["extensions"].split(',').map(str::trim).filter_map(|e| if e.is_empty() { None } else {
            Some(format!(".{e}"))
        }).collect(),
        output_dir: curr_setting["output_dir"].parse().unwrap(),
        base_of_valid_files: curr_setting["base_of_valid_files"].parse().unwrap(),
        input_dir,
        app_binary: curr_setting["app_binary"].parse().unwrap(),
        app_config: curr_setting["app_config"].parse().unwrap(),
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
}
