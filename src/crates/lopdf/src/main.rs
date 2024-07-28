use lopdf::Document;
use std::env::args;
use std::fs;
use std::io::Cursor;
use std::path::Path;
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
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    let cursor = Cursor::new(content);
    match Document::load_from(cursor) {
        Ok(mut document) => {
            let pages = document.get_pages();

            let mut doc_clone = document.clone();
            doc_clone.decompress();

            for (i, _) in pages.iter().enumerate() {
                let page_number = (i + 1) as u32;
                let _text = document.extract_text(&[page_number]);
            }

            let _ = document.save_to(&mut Cursor::new(Vec::new()));
        }
        Err(err) => {
            eprintln!("Error reading PDF contents: {}", err)
        }
    }
}
