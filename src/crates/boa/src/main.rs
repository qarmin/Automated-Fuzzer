use std::env::args;
use std::fs;
use std::path::Path;
// use dicom_core::header::Header;
use walkdir::WalkDir;
use boa_engine::Source; use boa_engine::Context;
fn main() {
    let path = args().nth(1).unwrap().clone();
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
    }

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            check_file(&path);
        }
    } else {
        check_file(&path);
    }
}

fn check_file(path: &str) {
    let Ok(file_content) = fs::read(path) else {
        return;
    };
    println!("Checking file: {path}");
    let mut context = Context::default();

    let _result = context.eval(Source::from_bytes(&file_content));
}