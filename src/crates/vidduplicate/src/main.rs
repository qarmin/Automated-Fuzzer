use std::env::args;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use vid_dup_finder_lib::{ffmpeg_builder, CreationOptions, Cropdetect};
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
    // assert!(ffmpeg_cmdline_utils::ffmpeg_and_ffprobe_are_callable());

    let builders = [
        ffmpeg_builder::VideoHashBuilder::default(),
        ffmpeg_builder::VideoHashBuilder::from_options(CreationOptions {
            skip_forward_amount: -200.0,
            duration: -200.0,
            cropdetect: Cropdetect::None,
        }),
        ffmpeg_builder::VideoHashBuilder::from_options(CreationOptions {
            skip_forward_amount: 0.0,
            duration: 0.0,
            cropdetect: Cropdetect::Letterbox,
        }),
        ffmpeg_builder::VideoHashBuilder::from_options(CreationOptions {
            skip_forward_amount: 140.0,
            duration: 250.0,
            cropdetect: Cropdetect::Motion,
        }),
    ];

    for i in builders {
        let _ = i.hash(PathBuf::from(file_path));
    }
}
