use std::env::args;
use std::fs::File;
use std::path::Path;
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
    match File::open(&file_path) {
        Ok(file) => {
            if let Err(e) = zip::ZipArchive::new(file) {
                println!("Failed to open zip file {e}");
            }
        }
        Err(_inspected) => (),
    }
}
