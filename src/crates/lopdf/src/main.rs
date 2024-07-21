use std::env::args;
use std::io::Cursor;
use std::path::Path;
use lopdf::Document;
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    if !Path::new(&path).exists() {
        panic!("Missing file");
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
    match Document::load(file_path) {
        Ok(document) => {
            let pages = document.get_pages();

            for (i, _) in pages.iter().enumerate() {
                let page_number = (i + 1) as u32;
                let _text = document.extract_text(&[page_number]);
            }
        }
        Err(err) => {
            eprintln!("Error reading PDF contents: {}", err)
        }
    }
}