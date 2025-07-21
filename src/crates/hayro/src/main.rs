use std::env::args;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use hayro::{render, InterpreterSettings, Pdf, RenderSettings};
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    let save_path = args().nth(2);
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
    }

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            check_file(&path, save_path.as_deref());
        }
    } else {
        check_file(&path, save_path.as_deref());
    }
}
fn check_file(file_path: &str, save_path: Option<&str>) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    println!("Checking file: {file_path}");

    let data = Arc::new(content);
    if let Ok(pdf) = Pdf::new(data) {
        for (idx, page) in pdf.pages().iter().enumerate() {
            let pixmap = render(page, &InterpreterSettings::default(), &RenderSettings::default());
            if let Some(save_path) = save_path {
                fs::write(format!("{save_path}_{}.png", idx + 1), pixmap.take_png()).unwrap();
            }
        }
    }
}
