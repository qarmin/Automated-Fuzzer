use std::env::args;
use std::fs;
use std::path::Path;
use little_exif::metadata::Metadata;
use little_exif::filetype::FileExtension;
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

    // Try to read metadata from path
    let path_obj = Path::new(path);
    let _ = Metadata::new_from_path(path_obj);

    // Try all file types to find crashes, regardless of actual extension
    let file_types = vec![
        FileExtension::JPEG,
        FileExtension::PNG { as_zTXt_chunk: false },
        FileExtension::PNG { as_zTXt_chunk: true },
        FileExtension::WEBP,
        FileExtension::TIFF,
        FileExtension::HEIF,
        FileExtension::JXL,
    ];

    for file_type in file_types {
        let _ = Metadata::new_from_vec(&content, file_type);
    }
}

