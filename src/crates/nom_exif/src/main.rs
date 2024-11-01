use std::env::args;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use nom_exif::{ExifIter, MediaParser, MediaSource, TrackInfo};
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
    let content = match fs::read(&path) {
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
        return ;
    };
    let iter: Result<ExifIter, _> = parser.parse(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps_info();
        for i in iter {
            let s = i;
            s.tag_code();
            s.get_value();
            let _ = s.get_result();
            s.tag();
            s.ifd_index();
            s.has_value();
        }
    }

    let reader = Cursor::new(&content);
    let Ok(ms) = MediaSource::unseekable(reader) else {
        return ;
    };
    let _: Result<TrackInfo, _> = parser.parse(ms);

    // Parse seekable
    let reader = Cursor::new(&content);
    let Ok(ms) = MediaSource::seekable(reader) else {
        return ;
    };
    let iter: Result<ExifIter, _> = parser.parse(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps_info();
        for i in iter {
            let s = i;
            s.tag_code();
            s.get_value();
            let _ = s.get_result();
            s.tag();
            s.ifd_index();
            s.has_value();
        }
    }

    let reader = Cursor::new(&content);
    let Ok(ms) = MediaSource::seekable(reader) else {
        return ;
    };
    let _: Result<TrackInfo, _> = parser.parse(ms);
}
