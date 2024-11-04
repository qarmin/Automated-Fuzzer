use std::env::args;
use std::fs;
use std::io::Read;
use std::path::Path;
use ttf_parser::GlyphId;
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
fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    let face = match ttf_parser::Face::parse(&content, 0) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {}.", e);
            return;
        }
    };
    let gid = GlyphId(0);
    let _ = face.glyph_raster_image(gid, 0);
    let _ = face.glyph_raster_image(gid, 96);
    let _ = face.glyph_raster_image(gid, u16::MAX);
    let _ = face.glyph_name(gid);
}
