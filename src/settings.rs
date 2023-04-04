use strum_macros::EnumString;

// NON_DESTRUCTIVE_INPUT_DIR - input files if COPY_BROKEN_FILES is not set
// BROKEN_FILES_DIR - input files if COPY_BROKEN_FILES is set
// BASE_OF_VALID_FILES - folder with (almost)valid files, that will be used to create broken ones
// OUTPUT_DIR - where to copy broken files
// CURRENT_MODE - variable to tell app which app needs to be checked

// pub const BROKEN_FILES_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/AA_BROKEN_INPUT_FILES";

// OXC
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/oxc/Broken";
// pub const BASE_OF_VALID_FILES: &str =
//     "/home/rafal/Desktop/RunEveryCommand/AA_JAVASCRIPT_VALID_FILES";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/oxc/Broken";
// pub const EXTENSIONS: &[&str] = &[".js", ".ts", ".mjs", ".mts"];
// pub const CURRENT_MODE: MODES = MODES::OXC;

// DLINT
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Dlint/Broken";
// pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/AA_JAVASCRIPT_VALID_FILES";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Dlint/Broken";
// pub const EXTENSIONS: &[&str] = &[".js", ".ts", ".mjs", ".mts"];
// pub const CURRENT_MODE: MODES = MODES::DLINT;

// ROME
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/Broken";
// pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/AA_JAVASCRIPT_VALID_FILES";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/Broken";
// pub const EXTENSIONS: &[&str] = &[".js", ".ts", ".mjs", ".mts"];
// pub const CURRENT_MODE: MODES = MODES::ROME;

// RUFF
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/Broken";
// pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/AA_PYTHON_VALID_FILES";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/Broken";
// pub const EXTENSIONS: &[&str] = &[".py"];
// pub const CURRENT_MODE: MODES = MODES::RUFF;

// Mypy
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/mypy/Broken";
// pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/AA_PYTHON_VALID_FILES";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/mypy/Broken";
// pub const EXTENSIONS: &[&str] = &[".py"];
// pub const CURRENT_MODE: MODES = MODES::MYPY;

// LOOP_NUMBER - How much creating/removing/checking steps will be executed
// BROKEN_FILES_FOR_EACH_FILE - Number of broken files that will be created for each 1 valid file
// COPY_BROKEN_FILES - If true, will copy broken files that cause problems to OUTPUT_DIR
// GENERATE_FILES - If true will generate broken files and save them to BROKEN_FILES_DIR(this folder will be removed after each run)
// MINIMIZE_OUTPUT - Tries to remove some lines from output file, remember, that not always minimized file will produce same error - usually minimize output 2-100 times

// pub const INPUT_DIR: &str = if COPY_BROKEN_FILES {
//     BROKEN_FILES_DIR
// } else {
//     NON_DESTRUCTIVE_INPUT_DIR
// };
// pub const LOOP_NUMBER: u32 = 10;
// pub const BROKEN_FILES_FOR_EACH_FILE: u32 = 1;
// pub const COPY_BROKEN_FILES: bool = true;
// pub const GENERATE_FILES: bool = true;
// pub const MINIMIZE_OUTPUT: bool = true;
// pub const MINIMIZATION_ATTEMPTS: u32 = 50;

// #[allow(dead_code)]
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
