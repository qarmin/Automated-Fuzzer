use std::path::Path;
use std::sync::atomic::AtomicBool;

use similario_core::visual::{SignatureConfig, VideoSignature};

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let stop_flag = AtomicBool::new(false);
    let config = SignatureConfig::default();
    let _ = VideoSignature::from_path(Path::new(path), &config, &stop_flag);
}
