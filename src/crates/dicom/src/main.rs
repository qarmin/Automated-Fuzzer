use dicom_object::from_reader;
use std::env::args;
use std::fs;
use std::path::Path;
use dicom_core::header::Header;
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

    let all_items = res.clone().into_iter().collect::<Vec<_>>();
    let hash_map = all_items.into_iter().map(|item| (item.tag(), item)).collect::<std::collections::HashMap<_, _>>();

    let mut item_to_dump = Vec::new();
    if let Err(e) = res.write_all(&mut item_to_dump) {
        eprintln!("Error: {}", e);
        return;
    };
    let Ok(res2) = from_reader(std::io::Cursor::new(item_to_dump.clone())) else {
        panic!("DIFFERENT CONTENT, This was properly loaded and saved before");
    };
    let all_items2 = res2.clone().into_iter().collect::<Vec<_>>();
    let hash_map2 = all_items2.into_iter().map(|item| (item.tag(), item)).collect::<std::collections::HashMap<_, _>>();
    assert_eq!(hash_map.len(), hash_map2.len(), "DIFFERENT CONTENT, Different number of items");
    for (tag, item) in hash_map {
        let item2 = &hash_map2[&tag];
        assert_eq!(item.value(), item2.value(), "DIFFERENT CONTENT, tag: {:?}", tag);
    }
    // fs::write("a.dcm", &item_to_dump).unwrap();
}