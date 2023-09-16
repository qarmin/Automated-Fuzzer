mod error_in_format_tool;
mod find_black_ruff_differences;
mod settings;
mod test_ruff_format_stability;

use crate::error_in_format_tool::error_in_format_ttol;
use crate::find_black_ruff_differences::check_differences;
use crate::settings::load_settings;
use crate::test_ruff_format_stability::test_ruff_format_stability;
use handsome_logger::{format_description, ColorChoice, ConfigBuilder, TerminalMode, TimeFormat};

fn main() {
    let config = ConfigBuilder::new()
        .set_time_format(
            TimeFormat::Custom(format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
            )),
            None,
        )
        .build();
    handsome_logger::TermLogger::init(config, TerminalMode::Mixed, ColorChoice::Always).unwrap();

    let settings = load_settings();

    if settings.mode == "check_difference" {
        check_differences(&settings);
    } else if settings.mode == "error_in_format_tool" {
        error_in_format_ttol(&settings);
    } else if settings.mode == "test_ruff_format_stability" {
        test_ruff_format_stability(&settings);
    } else {
        panic!("Unknown mode: {}", settings.mode);
    }
}
