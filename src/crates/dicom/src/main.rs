use dicom_object::open_file;
use std::env::args;
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

fn check_file(path: &str) {
    let res = open_file(path);
    if let Err(e) = res {
        // eprintln!("Error: {}", e);
    } else {
        // println!("Result: {:?}", res);
    }
    // dbg!(res);
}
