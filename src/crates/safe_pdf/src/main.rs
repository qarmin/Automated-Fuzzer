use std::env::args;
use std::path::Path;
use walkdir::WalkDir;
use std::fs;
use pdf_document::PdfDocument;

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

fn check_file(file_path: &str) {
    match fs::read(file_path) {
        Ok(bytes) => {
            if let Err(e) = PdfDocument::from(&bytes) {
                eprintln!("Error {}", e);
            }
        }
        Err(e) => eprintln!("Error {}", e),
    }
}
