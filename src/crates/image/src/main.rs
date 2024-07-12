use std::env::args;
use std::io::Cursor;
use std::path::Path;

use image::ImageFormat;
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();

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
    let res = match image::open(file_path) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

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
        ImageFormat::Avif,
        ImageFormat::Qoi,
    ]
        .into_iter()
    {
        let buffer: Vec<u8> = Vec::new();
        if let Err(e) = res.write_to(&mut Cursor::new(buffer), format) {
            eprintln!("Error: {}", e);
        };
    }
}
