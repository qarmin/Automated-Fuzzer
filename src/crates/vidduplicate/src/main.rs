use std::env::args;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use vid_dup_finder_lib::ffmpeg_builder;
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
fn check_file(file_path: &str) {
    println!("Checking file: {:?}", file_path);
    assert!(ffmpeg_cmdline_utils::ffmpeg_and_ffprobe_are_callable());

    let _vhash = match ffmpeg_builder::VideoHashBuilder::default().hash(PathBuf::from(file_path)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error while hashing file: {:?}", e);
            return;
        }
    };
}
