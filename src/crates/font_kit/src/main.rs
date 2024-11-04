use std::env::args;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use font_kit::handle::Handle;
use font_kit::source::Source;
use font_kit::sources::mem::MemSource;

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
    let handle = Handle::Path {
        path: PathBuf::from(path),
        font_index: 0
    };
    match MemSource::from_fonts([handle.clone()].into_iter()) {
        Ok(mut source) => {
            let _ = source.all_families();
            let _ = source.select_best_match(&[], &font_kit::properties::Properties::new());
            let _ = source.select_by_postscript_name("");
            let _ = source.as_any();
            let _ = source.add_font(handle);
            let _ = source.all_families();
            let _ = source.select_best_match(&[], &font_kit::properties::Properties::new());
            let _ = source.select_by_postscript_name("");
            let _ = source.as_any();
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}
