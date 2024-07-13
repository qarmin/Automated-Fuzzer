use std::env::args;
use std::fs::File;
use std::io::Read;
use std::path::Path;
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
    match File::open(&file_path) {
        Ok(file) => {
            let mut zip = match zip::ZipArchive::new(file) {
                Ok(t) => t,
                Err(e) => {
                    println!("{e}");
                    return;
                }
            };

            for i in 0..zip.len() {
                match zip.by_index(i) {
                    Ok(mut file) => {
                        let mut buf = Vec::new();
                        let _ = file.read(&mut buf);
                    }
                    Err(e) => {
                        println!("{e}");
                    }
                }
            }
        }
        Err(_inspected) => (),
    }
}
