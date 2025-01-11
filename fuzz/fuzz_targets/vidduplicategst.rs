#![no_main]

use std::env::{args, temp_dir};
use std::path::{Path, PathBuf};
use std::{fs, io};

use libfuzzer_sys::{fuzz_target, Corpus};
use tempfile::tempdir;
use vid_dup_finder_lib_gst::ffmpeg_builder;

fuzz_target!(|data: &[u8]| -> Corpus {
    if calculate_hash(data.to_vec()).is_ok() {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});

pub fn calculate_hash(content: Vec<u8>) -> Result<(), String> {
    let temp_dir = tempdir().map_err(|e| e.to_string())?;
    let temp_file = temp_dir.path().join(format!("fuzz_file_{}", rand::random::<u32>()));

    fs::write(&temp_file, content).map_err(|e| e.to_string())?;

    let res = ffmpeg_builder::VideoHashBuilder::default().hash(PathBuf::from(&temp_file));

    let _ = fs::remove_file(temp_file);

    res.map_err(|e| e.to_string())?;
    Ok(())
}
