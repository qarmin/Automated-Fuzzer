use std::env::args;
use std::fs;
use std::path::Path;

use lofty::file::{AudioFile, FileType};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
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
const ALL_FILE_TYPES: &[FileType] = &[FileType::Aac, FileType::Aiff, FileType::Ape, FileType::Flac, FileType::Mpeg, FileType::Mp4, FileType::Mpc, FileType::Opus, FileType::Vorbis, FileType::Speex, FileType::Wav, FileType::WavPack];

fn check_file(path: &str) {
    let content = match fs::read(&path) {
        Ok(content) => content,
        Err(e) => {
            println!("{e}");
            return;
        }
    };

    for i in ALL_FILE_TYPES {
        let s = std::io::Cursor::new(&content);
        let tagged_file = match Probe::with_file_type(s, *i).read() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        tagged_file.properties();
        tagged_file.tags();
        tagged_file.primary_tag();
    }
}
