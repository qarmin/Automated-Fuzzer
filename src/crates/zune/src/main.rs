use std::env::args;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use zune_core::bytestream::ZCursor;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;

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

    for options in [DecoderOptions::new_fast(), DecoderOptions::new_cmd(), DecoderOptions::new_safe()] {
        let img = Image::read(ZCursor::new(&file_content), options);
        match img {
            Ok(_) => {
                println!("Successfully checked file: {path}");}
            Err(_) => {            }
        }
    }

}