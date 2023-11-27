use crate::apps::biome::BiomeStruct;
use crate::apps::dlint::DlintStruct;
use crate::apps::eslint::EslintStruct;
use crate::apps::image::ImageStruct;
use crate::apps::libcst::LibCSTStruct;
use crate::apps::lofty::LoftyStruct;
use crate::apps::mypy::MypyStruct;
use crate::apps::oxc::OxcStruct;
use crate::apps::pdfrs::PdfRsStruct;
use crate::apps::quick_lint_js::QuickLintStruct;
use crate::apps::ruff::RuffStruct;
use crate::apps::rust_parser::RustParserStruct;
use crate::apps::rustfmt::RustFmtStruct;
use crate::apps::selene::SeleneStruct;
use crate::apps::staticheckgo::StaticCheckGoStruct;
use crate::apps::symphonia::SymphoniaStruct;
use crate::obj::ProgramConfig;
use config::Config;
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::EnumString;

pub const TIMEOUT_MESSAGE: &str = "timeout: sending signal";

#[derive(Clone, Debug)]
pub struct Setting {
    pub loop_number: u32,
    pub broken_files_for_each_file: u32,
    pub copy_broken_files: bool,
    pub generate_files: bool,
    pub minimize_output: bool,
    pub minimization_attempts: u32,
    pub minimization_attempts_with_signal_timeout: u32,
    pub remove_non_crashing_items_from_broken_files: bool,
    pub clean_base_files: bool,
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
    pub ignore_generate_copy_files_step: bool,
    pub find_minimal_rules: bool,
    pub check_if_file_is_parsable: bool,
    pub verify_if_files_are_still_broken: bool,
    pub disable_exceptions: bool,
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
    let verify_if_files_are_still_broken = general["verify_if_files_are_still_broken"].parse().unwrap();
    let find_minimal_rules = general["find_minimal_rules"].parse().unwrap();
    let remove_non_crashing_items_from_broken_files =
        general["remove_non_crashing_items_from_broken_files"].parse().unwrap();
    let disable_exceptions =
        verify_if_files_are_still_broken && !(find_minimal_rules || remove_non_crashing_items_from_broken_files);
    Setting {
        loop_number: general["loop_number"].parse().unwrap(),
        broken_files_for_each_file: general["broken_files_for_each_file"].parse().unwrap(),
        copy_broken_files: general["copy_broken_files"].parse().unwrap(),
        generate_files: general["generate_files"].parse().unwrap(),
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
        ignore_generate_copy_files_step: general["ignore_generate_copy_files_step"].parse().unwrap(),
        clean_base_files: general["clean_base_files"].parse().unwrap(),
        temp_folder: general["temp_folder"].clone(),
        find_minimal_rules,
        tool_type: curr_setting["tool_type"].clone(),
        check_if_file_is_parsable: general["check_if_file_is_parsable"].parse().unwrap(),
        verify_if_files_are_still_broken,
        disable_exceptions,
    }
}

pub fn get_object(settings: Setting) -> Box<dyn ProgramConfig> {
    match settings.current_mode {
        MODES::OXC => Box::new(OxcStruct { settings }),
        MODES::MYPY => Box::new(MypyStruct { settings }),
        MODES::DLINT => Box::new(DlintStruct { settings }),
        MODES::BIOME => Box::new(BiomeStruct { settings }),
        MODES::RUFF => Box::new(RuffStruct {
            settings,
            ignored_rules: String::new(),
        }),
        MODES::LIBCST => Box::new(LibCSTStruct { settings }),
        MODES::LOFTY => Box::new(LoftyStruct { settings }),
        MODES::IMAGE => Box::new(ImageStruct { settings }),
        MODES::SYMPHONIA => Box::new(SymphoniaStruct { settings }),
        MODES::SELENE => Box::new(SeleneStruct { settings }),
        MODES::STATICCHECKGO => Box::new(StaticCheckGoStruct { settings }),
        MODES::QUICKLINTJS => Box::new(QuickLintStruct { settings }),
        MODES::PDFRS => Box::new(PdfRsStruct { settings }),
        MODES::RUSTFMT => Box::new(RustFmtStruct { settings }),
        MODES::ESLINT => Box::new(EslintStruct { settings }),
        MODES::RUSTPARSER => Box::new(RustParserStruct { settings }),
    }
}

#[derive(Debug, PartialEq, EnumString, Copy, Clone)]
pub enum MODES {
    #[strum(ascii_case_insensitive)]
    RUFF,
    #[strum(ascii_case_insensitive)]
    MYPY,
    #[strum(ascii_case_insensitive)]
    BIOME,
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
    #[strum(ascii_case_insensitive)]
    SELENE,
    #[strum(ascii_case_insensitive)]
    STATICCHECKGO,
    #[strum(ascii_case_insensitive)]
    QUICKLINTJS,
    #[strum(ascii_case_insensitive)]
    PDFRS,
    #[strum(ascii_case_insensitive)]
    RUSTFMT,
    #[strum(ascii_case_insensitive)]
    ESLINT,
    #[strum(ascii_case_insensitive)]
    RUSTPARSER,
    #[strum(ascii_case_insensitive)]
    LIBCST,
}
