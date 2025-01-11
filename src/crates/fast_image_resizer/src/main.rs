use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg};
use image::*;
use std::env::args;
use std::path::Path;
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

const FILTER_TYPES: &[FilterType] = &[
    FilterType::Box,
    FilterType::Bilinear,
    FilterType::Hamming,
    FilterType::CatmullRom,
    FilterType::Mitchell,
    FilterType::Gaussian,
    FilterType::Lanczos3,
];

const PIXEL_TYPES: &[PixelType] = &[
    PixelType::U8,
    PixelType::U8x2,
    PixelType::U8x3,
    PixelType::U8x4,
    PixelType::U16,
    PixelType::U16x2,
    PixelType::U16x3,
    PixelType::U16x4,
    PixelType::I32,
    PixelType::F32,
    PixelType::F32x2,
    PixelType::F32x3,
    PixelType::F32x4,
];

fn check_file(file_path: &str) {
    let Ok(src_image) = ImageReader::open(file_path) else {
        eprintln!("Error while reading image: {:?}", file_path);
        return;
    };
    let Ok(decoded) = src_image.decode() else {
        eprintln!("Error while decoding image: {:?} (probably not image)", file_path);
        return;
    };

    let mut all = vec![];
    all.push(ResizeAlg::Nearest);
    for filter_type in FILTER_TYPES {
        all.push(ResizeAlg::Interpolation(*filter_type));
        all.push(ResizeAlg::Convolution(*filter_type));
        for num in [2,15] {
            all.push(ResizeAlg::SuperSampling(*filter_type, num));
        }
    }


    for pixel_type in PIXEL_TYPES {
        let w = decoded.width() % 300;
        let h = decoded.height() % 300;

        for (width, height) in [(0, 0), (1, 1), (300, 1), (1, 300), (500, 50), (50, 500), (w, h), (w * 2, h * 2), (w / 2, h / 2)] {
            for resize_alg in &all {
                println!(
                    "Checking file: {:?}, width: {:?}, height: {:?}, pixel_type: {:?}, resize_alg: {:?}",
                    file_path, width, height, pixel_type, resize_alg
                );
                let mut dst_image = Image::new(width, height, *pixel_type);
                let resize_options = fast_image_resize::ResizeOptions::new().resize_alg(resize_alg.clone());
                if let Err(e) =
                    fast_image_resize::Resizer::new().resize(&decoded, &mut dst_image, Some(&resize_options))
                {
                    eprintln!("Error while resizing image: {:?}", e);
                };
            }
        }
    }
}
