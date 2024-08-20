use pdf::file::FileOptions;
use std::env::args;
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
    match FileOptions::cached().open(&file_path) {
        Ok(file) => {
            for idx in 0..file.num_pages() {
                if let Ok(page) = file.get_page(idx) {
                    let _ = page.media_box();
                    let _ = page.crop_box();
                    let _ = page.resources();
                }
                let _ = file.get_root();
            }
        }
        Err(e) => println!("Error {}", e),
    }
}
