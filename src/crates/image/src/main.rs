use std::env::args;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::ImageFormat;
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
        eprintln!("Error: {:?}", e);
        return;
    };
    let res = match image::load_from_memory(&content) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    println!("Image: {file_path}");
    for format in [
        ImageFormat::Bmp,
        ImageFormat::Farbfeld,
        ImageFormat::Ico,
        ImageFormat::Jpeg,
        ImageFormat::Png,
        ImageFormat::Pnm,
        ImageFormat::Tiff,
        ImageFormat::WebP,
        ImageFormat::Tga,
        ImageFormat::Dds,
        ImageFormat::Hdr,
        ImageFormat::OpenExr,
        // ImageFormat::Avif, // Don't use, it is really slow https://github.com/image-rs/image/issues/2282
        ImageFormat::Qoi,
    ]
        .into_iter()
    {
        let buffer: Vec<u8> = Vec::new();
        println!("Before write_to {format:?}");
        if let Err(e) = res.write_to(&mut Cursor::new(buffer), format) {
            eprintln!("Error: {}", e);
        };
        println!("After write_to {format:?}");
    }
}
