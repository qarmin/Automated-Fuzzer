use std::env::args;
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::Path;
use moxcms::{ColorProfile, Layout, TransformOptions};
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

    let profile = ColorProfile::new_from_slice(&file_content);
    let new_profile ;
    match profile {
        Ok(profile) => {
            new_profile = profile;
        }
        Err(_) => {
            let img = image::ImageReader::new(BufReader::new(Cursor::new(file_content.as_slice())));
            let Ok(decoded_img) = img.with_guessed_format().and_then(|reader| reader.decode()) else {
                return;
            };
            return;
        }
    }


    println!("Checking file: {path}");
    let profile = ColorProfile::new_from_slice(&file_content);
    match profile {
        Ok(profile) => {
            let new_srgb = ColorProfile::new_srgb();
            for &src_layout in ALL_LAYOUTS {
                for &dst_layout in ALL_LAYOUTS {
                    _ = profile.create_transform_8bit(
                        src_layout,
                        &profile,
                        dst_layout,
                        TransformOptions::default(),
                    );
                    _ = new_srgb.create_transform_8bit(
                        src_layout,
                        &profile,
                        dst_layout,
                        TransformOptions::default(),
                    );
                }
            }
        }
        Err(_) => {}
    }
}