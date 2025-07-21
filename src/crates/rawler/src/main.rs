use std::env::args;
use std::fs;
use std::fs::File;
use std::path::Path;

use rawler::decoders::{ RawDecodeParams, RawMetadata};
use rawler::imgop::develop::{Intermediate, RawDevelop};
use rawler::rawsource::RawSource;
use rawler::{Orientation};
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    let save_path = args().nth(2);
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
    }

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            check_file(&path, save_path.as_deref());
        }
    } else {
        check_file(&path, save_path.as_deref());
    }
}
fn check_file(file_path: &str, save_path: Option<&str>) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    println!("Checking file: {file_path}");
    match get_raw_file(&content) {
        Ok((int, met)) => {
            if let Some(save_path) = save_path {
                to_image_rs(int, met, &format!("{}/converted_{}.jpg", save_path, Path::new(file_path).file_stem().unwrap().to_string_lossy()));
            }
        }
        Err(_e) => {
            // println!("Error processing file {}: {}", file_path, e)
        },
    }
}

fn get_raw_file(content: &[u8]) -> Result<(Intermediate, RawMetadata), String> {
    let raw_source = RawSource::new_from_slice(content);
    let decoder = rawler::get_decoder(&raw_source).map_err(|e| e.to_string())?;
    let metadata = decoder
        .raw_metadata(&raw_source, &RawDecodeParams::default())
        .map_err(|e| e.to_string())?;
    let raw_image = decoder
        .raw_image(&raw_source, &RawDecodeParams::default(), false)
        .map_err(|e| e.to_string())?;
    let developer = RawDevelop::default();
    let developed_image = developer.develop_intermediate(&raw_image).map_err(|e| e.to_string())?;
    Ok((developed_image, metadata))
}

fn to_image_rs(developed_image: Intermediate, metadata: RawMetadata, save_path: &str) {
    let orientation = metadata
        .exif
        .orientation
        .map(Orientation::from_u16)
        .unwrap_or(Orientation::Normal);

    let dynamic_image = developed_image.to_dynamic_image().unwrap();

    let rotated_image = match orientation {
        Orientation::Normal => dynamic_image,
        Orientation::HorizontalFlip => dynamic_image.fliph(),
        Orientation::Rotate180 => dynamic_image.rotate180(),
        Orientation::VerticalFlip => dynamic_image.flipv(),
        Orientation::Transpose => dynamic_image.rotate90().flipv(),
        Orientation::Rotate90 => dynamic_image.rotate90(),
        Orientation::Transverse => dynamic_image.rotate90().fliph(),
        Orientation::Rotate270 => dynamic_image.rotate270(),
        Orientation::Unknown => dynamic_image,
    };

    let image_rgb8 = rotated_image.to_rgb8();

    let mut output_file = File::create(save_path).unwrap_or_else(|_| panic!("Failed to create file: {}", save_path));
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output_file, 85);
    encoder.encode_image(&image_rgb8).unwrap();
    println!("File saved to: {}", save_path);
}
