mod common;
mod error_in_format_tool;
mod find_black_ruff_differences;
mod find_parse_difference;
mod settings;
mod test_ruff_format_stability;

use crate::error_in_format_tool::error_in_format_ttol;
use crate::find_black_ruff_differences::check_differences;
use crate::settings::load_settings;
use crate::test_ruff_format_stability::test_ruff_format_stability;

fn main() {
    handsome_logger::init().unwrap();

    let settings = load_settings();

    if settings.mode == "check_difference" {
        check_differences(&settings);
    } else if settings.mode == "error_in_format_tool" {
        error_in_format_ttol(&settings);
    } else if settings.mode == "test_ruff_format_stability" {
        test_ruff_format_stability(&settings);
    } else if settings.mode == "find_parse_difference" {
        find_parse_difference::find_parse_difference(&settings);
    } else {
        panic!("Unknown mode: {}", settings.mode);
    }
}
