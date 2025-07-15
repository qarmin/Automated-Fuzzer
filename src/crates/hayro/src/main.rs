use std::env::args;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use hayro_render::{render_png, InterpreterSettings};
use walkdir::WalkDir;
use hayro_syntax::pdf::Pdf;

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
    if let Some(pdf) = Pdf::new(data) {
        let _pages = pdf.pages();
        let pixmaps = render_png(&pdf, 1.0, InterpreterSettings::default(), None);

        if let Some(save_path) = save_path {
            if let Some(pixmaps) = pixmaps {
                for (idx, i) in pixmaps.into_iter().enumerate() {
                    fs::write(format!("{save_path}_{}.png", idx + 1), i).unwrap();
                }
            }
        }

    }
}
