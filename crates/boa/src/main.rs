use std::fs;

use boa_engine::{Context, Source};
fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(file_content) = fs::read(path) else {
        return;
    };
    println!("Checking file: {path}");
    let mut context = Context::default();

    let _result = context.eval(Source::from_bytes(&file_content));
}
