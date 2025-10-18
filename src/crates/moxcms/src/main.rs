use std::env::args;
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::Path;
use moxcms::{ColorProfile, Layout, TransformOptions};
use walkdir::WalkDir;
use zune_jpeg::JpegDecoder;

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

const ALL_LAYOUTS: &[Layout] = &[
    Layout::Rgb,
    Layout::Rgba,
    Layout::Gray,
    Layout::GrayAlpha,
    Layout::Inks5,
    Layout::Inks6,
    Layout::Inks7,
    Layout::Inks8,
    Layout::Inks9,
    Layout::Inks10,
    Layout::Inks11,
    Layout::Inks12,
    Layout::Inks13,
    Layout::Inks14,
    Layout::Inks15,
];

fn check_file(path: &str) {
    let Ok(file_content) = fs::read(path) else {
        return;
    };

    let new_profile =
    match ColorProfile::new_from_slice(&file_content) {
        Ok(profile) => {
            println!("  Found top-level ICC profile, checking...");
            profile
        }
        Err(_) => {
            let mut decoder = JpegDecoder::new(BufReader::new(Cursor::new(file_content.as_slice())));
            if  decoder.decode().is_err() {
                return;
            }
            let Some(icc) = decoder.icc_profile() else {
                return;
            };
            println!("  Found embedded ICC profile, checking...");
            if let Ok(profile) = ColorProfile::new_from_slice(&icc) {
                profile
            } else {
                return;
            }
        }
    };

    let new_srgb = ColorProfile::new_srgb();
    for &src_layout in ALL_LAYOUTS {
        for &dst_layout in ALL_LAYOUTS {
            _ = new_profile.create_transform_8bit(
                src_layout,
                &new_profile,
                dst_layout,
                TransformOptions::default(),
            );
            _ = new_srgb.create_transform_8bit(
                src_layout,
                &new_profile,
                dst_layout,
                TransformOptions::default(),
            );
        }
    }
    println!("Successfully checked file: {path}");
}