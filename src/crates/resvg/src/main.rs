use std::env::args;
use std::path::Path;
use resvg::{tiny_skia, usvg};
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
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let Ok(svg_data) = std::fs::read(&file_path) else {
        println!("Failed to render(std::fs::read): {file_path}");
        return;
    };
    let Ok(tree) = usvg::Tree::from_data(&svg_data, &opt) else {
        println!("Failed to render(usvg::Tree::from_data): {file_path}");
        return
    };

    let pixmap_size = tree.size().to_int_size();
    if let Some(mut pixmap) = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()) {
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
        println!("Properly rendered: {file_path}");
    } else {
        println!("Failed to render(Pixmap::new()): {file_path}");
    };

}
