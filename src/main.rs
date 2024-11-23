#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::borrowed_box)]

use std::fs;
use std::path::Path;

use log::info;

use crate::common::{calculate_number_of_files, check_files_number, TIMEOUT_SECS};
use crate::finding_different_output::find_broken_files_by_different_output;
use crate::finding_text_status::find_broken_files_by_text_status;
use crate::settings::{get_object, load_settings, StabilityMode};

pub mod apps;
mod broken_files;
mod common;
mod finding_different_output;
mod finding_text_status;
mod minimal_rules;
mod obj;
mod remove_non_crashing_files;
mod settings;

fn main() {
    handsome_logger::init().unwrap();

    let first_arg: u64 = std::env::args()
        .nth(1)
        .map_or(999_999_999_999_999, |x| x.parse().unwrap());
    info!("Timeout set to {first_arg} seconds");
    TIMEOUT_SECS.set(first_arg).unwrap();

    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(8)
    //     .build_global()
    //     .unwrap();

    let settings = load_settings();

    // info!("{settings:#?}");

    let mut obj = get_object(settings.clone());

    obj.init();

    let _ = fs::create_dir_all(&settings.temp_folder);
    let _ = fs::create_dir_all(&settings.broken_files_dir);

    if settings.remove_non_crashing_items_from_broken_files {
        info!("RUNNING REMOVE NON CRASHING FILES");
        remove_non_crashing_files::remove_non_crashing_files(&settings, &obj);
        return;
    }
    if settings.find_minimal_rules {
        info!("RUNNING MINIMAL RULES");
        minimal_rules::check_code(&settings, &obj);
        return;
    }

    check_files_number("Valid input dir", &settings.valid_input_files_dir);
    check_files_number("Broken files dir", &settings.broken_files_dir);
    check_files_number(
        "Temp possible broken files dir", &settings.temp_possible_broken_files_dir,
    );

    assert!(Path::new(&settings.valid_input_files_dir).exists());
    assert!(Path::new(&settings.broken_files_dir).exists());

    info!(
        "Found {} files in valid input dir",
        calculate_number_of_files(&settings.valid_input_files_dir)
    );

    if settings.check_for_stability && obj.get_stability_mode() != StabilityMode::None {
        find_broken_files_by_different_output(&settings, &obj);
    } else {
        find_broken_files_by_text_status(&settings, &obj);
    }
}
