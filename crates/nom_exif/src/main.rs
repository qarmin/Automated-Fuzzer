use std::fs;
use std::io::Cursor;

use nom_exif::{ExifIter, MediaParser, MediaSource};

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    println!("Checking file: {path}");

    let mut parser = MediaParser::new();

    // Parse unseekable
    let reader = Cursor::new(&content);
    let Ok(ms) = MediaSource::unseekable(reader) else {
        return;
    };
    let iter: Result<ExifIter, _> = parser.parse_exif(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps();
        for s in iter {
            let _ = s.value();
            let _ = s.result();
            s.tag();
        }
    }

    // Parse seekable
    let reader = Cursor::new(&content);
    let Ok(ms) = MediaSource::seekable(reader) else {
        return;
    };
    let iter: Result<ExifIter, _> = parser.parse_exif(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps();
        for s in iter {
            let _ = s.value();
            let _ = s.result();
            s.tag();
        }
    }
}
