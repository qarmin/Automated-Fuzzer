use std::path::PathBuf;

use vid_dup_finder_lib::{ffmpeg_builder, CreationOptions, Cropdetect};

fn main() {
    fuzz_utils::run(check_file);
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
