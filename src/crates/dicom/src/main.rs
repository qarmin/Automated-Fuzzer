use dicom_dump::DumpOptions;
use dicom_object::from_reader;
use std::env::args;
use std::fs;
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
    let Ok(file_content) = fs::read(path) else {
        return;
    };
    let cursor = std::io::Cursor::new(file_content.clone());
    let res =
        match from_reader(cursor) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        };
    if let Err(e) = dicom_json::to_string(&res) {
        eprintln!("Error: {}", e);
        return;
    }

    let mut item_to_dump = Vec::new();
    if let Err(e) = DumpOptions::new().dump_object_to(&mut item_to_dump, &res) {
        eprintln!("Error: {}", e);
        return;
    };
    let mut item_to_dump = Vec::new();

    if let Err(e) = res.write_all(&mut item_to_dump) {
        eprintln!("Error: {}", e);
        return;
    }

    if item_to_dump != file_content {
        eprintln!("DIFFERENT CONTENT");
    }
    fs::write("a.dcm", item_to_dump).unwrap();
    // fs::write("b.dcm", file_content).unwrap()
}
