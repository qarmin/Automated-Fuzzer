use std::env::args;
use std::fs::File;

use lofty::file::TaggedFileExt;
use lofty::file::{AudioFile};
use lofty::read_from;
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

fn check_file(path: &str) {
    let mut file = match File::open(&path) {
        Ok(t) => t,
        Err(e) => {
            println!("{e} - {path}");
            return;
        }
    };
    let tagged_file = match read_from(&mut file) {
        Ok(t) => t,
        Err(e) => {
            println!("{e}");
            return;
        }
    };

    // let Ok(mut file) = File::open(&path) else { return; };
    // let Ok(tagged_file) = read_from(&mut file) else { return; };

    tagged_file.properties();
    tagged_file.tags();
    tagged_file.primary_tag();
}
