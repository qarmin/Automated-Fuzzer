use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use similario_core::audio::compute_fingerprint;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    if fs::read(path).is_err() {
        return;
    }

    println!("Checking file: {path:?}");
    let stop_flag = AtomicBool::new(false);
    let _ = compute_fingerprint(Path::new(path), &stop_flag);
}
