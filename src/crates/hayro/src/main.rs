use std::env::args;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use hayro_render::render_png;
use walkdir::WalkDir;
use hayro_syntax::pdf::Pdf;

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
    println!("Checking file: {file_path}");

    let data = Arc::new(content);
    if let Some(pdf) = Pdf::new(data) {
        let _pixmaps = render_png(&pdf, 1.0, None).unwrap();
    }
}
