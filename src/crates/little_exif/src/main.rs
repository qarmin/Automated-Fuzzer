use std::env::args;
use std::fs;
use std::path::Path;
use little_exif::metadata::Metadata;
use walkdir::WalkDir;


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
    let content = match fs::read(&path) {
        Ok(content) => content,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    println!("Checking file: {path}");

    let path_obj = Path::new(path);
    if let Ok(metadata) = Metadata::new_from_path(path_obj) {
        let _ = metadata.data();
    }
}

