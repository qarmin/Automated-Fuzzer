use std::env::args;
use std::fs;
use std::path::Path;

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

fn check_file(path: &str) {
    let Ok(data) = fs::read_to_string(path) else {
        return;
    };

    let r = full_moon::parse_fallible(&data, full_moon::LuaVersion::new());
    if !r.errors().is_empty() {
        println!("Error: {:?}", r.errors());
    }
}
