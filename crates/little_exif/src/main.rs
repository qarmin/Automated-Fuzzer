use std::fs;
use std::path::Path;

use little_exif::filetype::FileExtension;
use little_exif::metadata::Metadata;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let content = match fs::read(path) {
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
