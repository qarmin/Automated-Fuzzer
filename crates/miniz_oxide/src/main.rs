use std::fs;

use miniz_oxide::inflate::{decompress_to_vec_with_limit, decompress_to_vec_zlib_with_limit};

const MAX_OUTPUT: usize = 64 * 1024 * 1024;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else {
        return;
    };

    let _ = decompress_to_vec_with_limit(&data, MAX_OUTPUT);
    let _ = decompress_to_vec_zlib_with_limit(&data, MAX_OUTPUT);
}
