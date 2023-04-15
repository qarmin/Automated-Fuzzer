use crate::apps::dlint::DlintStruct;
use crate::apps::eslint::EslintStruct;
use crate::apps::image::ImageStruct;
use crate::apps::lofty::LoftyStruct;
use crate::apps::mypy::MypyStruct;
use crate::apps::oxc::OxcStruct;
use crate::apps::pdfrs::PdfRsStruct;
use crate::apps::quick_lint_js::QuickLintStruct;
use crate::apps::rome::RomeStruct;
use crate::apps::ruff::RuffStruct;
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
    pub current_mode: MODES,
    pub extensions: Vec<String>,
    pub output_dir: String,
    pub base_of_valid_files: String,
    pub input_dir: String,
    pub app_binary: String,
    pub app_config: String,
    pub binary_mode: bool,
    pub debug_print_results: bool,
    pub timeout: usize,
    pub error_statuses_different_than_0_1: bool,
    pub error_when_found_signal: bool,
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

    let extensions = curr_setting["extensions"]
        .split(',')
        .map(str::trim)
        .filter_map(|e| {
            if e.is_empty() {
                None
            } else {
                Some(format!(".{e}"))
            }
        })
        .collect();
    Setting {
        loop_number: general["loop_number"].parse().unwrap(),
        broken_files_for_each_file: general["broken_files_for_each_file"].parse().unwrap(),
        copy_broken_files: general["copy_broken_files"].parse().unwrap(),
        generate_files: general["generate_files"].parse().unwrap(),
        minimize_output: general["minimize_output"].parse().unwrap(),
        minimization_attempts: general["minimization_attempts"].parse().unwrap(),
        minimization_attempts_with_signal_timeout: general
            ["minimization_attempts_with_signal_timeout"]
            .parse()
            .unwrap(),
        current_mode,
        extensions,
        timeout: general["timeout"].parse().unwrap(),
        output_dir: curr_setting["output_dir"].parse().unwrap(),
        base_of_valid_files: curr_setting["base_of_valid_files"].parse().unwrap(),
        input_dir: general["broken_files_dir"].parse().unwrap(),
        app_binary: curr_setting["app_binary"].parse().unwrap(),
        app_config: curr_setting["app_config"].parse().unwrap(),
        binary_mode: curr_setting["binary_mode"].parse().unwrap(),
        debug_print_results: general["debug_print_results"].parse().unwrap(),
        error_statuses_different_than_0_1: general["error_statuses_different_than_0_1"]
            .parse()
            .unwrap(),
        error_when_found_signal: general["error_when_found_signal"].parse().unwrap(),
        debug_print_broken_files_creator: general["debug_print_broken_files_creator"]
            .parse()
            .unwrap(),
        safe_run: general["safe_run"].parse().unwrap(),
    }
}

pub fn get_object(settings: Setting) -> Box<dyn ProgramConfig> {
    match settings.current_mode {
        MODES::OXC => Box::new(OxcStruct { settings }),
        MODES::MYPY => Box::new(MypyStruct { settings }),
        MODES::DLINT => Box::new(DlintStruct { settings }),
        MODES::ROME => Box::new(RomeStruct { settings }),
        MODES::RUFF => Box::new(RuffStruct { settings }),
        MODES::LOFTY => Box::new(LoftyStruct { settings }),
        MODES::IMAGE => Box::new(ImageStruct { settings }),
        MODES::SYMPHONIA => Box::new(SymphoniaStruct { settings }),
        MODES::SELENE => Box::new(SeleneStruct { settings }),
        MODES::STATICCHECKGO => Box::new(StaticCheckGoStruct { settings }),
        MODES::QUICKLINTJS => Box::new(QuickLintStruct { settings }),
        MODES::PDFRS => Box::new(PdfRsStruct { settings }),
        MODES::RUSTFMT => Box::new(RustFmtStruct { settings }),
        MODES::ESLINT => Box::new(EslintStruct { settings }),
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
}
