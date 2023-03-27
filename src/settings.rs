// NON_DESTRUCTIVE_INPUT_DIR - input files if COPY_BROKEN_FILES is not set
// DESTRUCTIVE_INPUT_DIR - input files if COPY_BROKEN_FILES is set
// BASE_OF_VALID_FILES - folder with (almost)valid files, that will be used to create broken ones
// OUTPUT_DIR - where to copy broken files
// CURRENT_MODE - variable to tell app which app needs to be checked

// ROME
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/Broken";
// pub const DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/InvalidFiles";
// pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/ValidFiles";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Rome/Broken";
// pub const EXTENSIONS: &[&str] = &[".js", ".ts"];
// pub const CURRENT_MODE: MODES = MODES::ROME;

// RUFF
pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/Broken";
pub const DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/InvalidFiles";
pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/ValidFiles";
pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/Broken";
pub const EXTENSIONS: &[&str] = &[".py"];
pub const CURRENT_MODE: MODES = MODES::RUFF;

// Mypy
// pub const NON_DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/mypy/Broken";
// pub const DESTRUCTIVE_INPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/InvalidFiles";
// pub const BASE_OF_VALID_FILES: &str = "/home/rafal/Desktop/RunEveryCommand/Ruff/ValidFiles";
// pub const OUTPUT_DIR: &str = "/home/rafal/Desktop/RunEveryCommand/mypy/Broken";
// pub const EXTENSIONS: &[&str] = &[".py"];
// pub const CURRENT_MODE: MODES = MODES::MYPY;

// LOOP_NUMBER - How much creating/removing/checking steps will be executed
// BROKEN_FILES_FOR_EACH_FILE - Number of broken files that will be created for each 1 valid file
// COPY_BROKEN_FILES - If true, will copy broken files that cause problems to OUTPUT_DIR
// GENERATE_FILES - If true will generate broken files and save them to DESTRUCTIVE_INPUT_DIR(this folder will be removed after each run)
// MINIMIZE_OUTPUT - Tries to remove some lines

pub const INPUT_DIR: &str = if COPY_BROKEN_FILES {
    DESTRUCTIVE_INPUT_DIR
} else {
    NON_DESTRUCTIVE_INPUT_DIR
};
pub const LOOP_NUMBER: u32 = 1;
pub const BROKEN_FILES_FOR_EACH_FILE: u32 = 1;
pub const COPY_BROKEN_FILES: bool = true;
pub const GENERATE_FILES: bool = true;
pub const MINIMIZE_OUTPUT: bool = true;

#[allow(dead_code)]
pub enum MODES {
    RUFF,
    MYPY,
    ROME,
}
