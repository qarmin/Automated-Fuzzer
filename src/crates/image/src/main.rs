use std::env::args;
use std::fs;
use std::io::Cursor;
use std::ops::Deref;
use std::path::Path;

use image::{GenericImageView, ImageBuffer, ImageFormat, Pixel};
use image::imageops;
use walkdir::WalkDir;
const IMAGE_FORMATS_READ: &[ImageFormat] = &[
    ImageFormat::Png,
    ImageFormat::Jpeg,
    ImageFormat::Gif,
    ImageFormat::WebP,
    ImageFormat::Pnm,
    ImageFormat::Tiff,
    ImageFormat::Tga,
    ImageFormat::Dds,
    ImageFormat::Bmp,
    ImageFormat::Ico,
    ImageFormat::Hdr,
    ImageFormat::OpenExr,
    ImageFormat::Farbfeld,
    ImageFormat::Avif,
    ImageFormat::Qoi,
];
const IMAGE_FORMATS_WRITE: &[ImageFormat] = &[
    ImageFormat::Png,
    ImageFormat::Jpeg,
    ImageFormat::Gif,
    ImageFormat::WebP,
    ImageFormat::Pnm,
    ImageFormat::Tiff,
    ImageFormat::Tga,
    ImageFormat::Dds,
    ImageFormat::Bmp,
    ImageFormat::Ico,
    ImageFormat::Hdr,
    ImageFormat::OpenExr,
    ImageFormat::Farbfeld,
    // ImageFormat::Avif, // Don't use, it is really slow
    ImageFormat::Qoi,
];

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
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    let mut img = None;

    for format in IMAGE_FORMATS_READ.iter() {
        let res = image::load_from_memory_with_format(&content, *format);
        if let Ok(res) = res {
            img = Some(res);
        }
    }

    let img = match img {
        Some(img) => img,
        None => return,
    };


    // let _ = img.as_rgb8().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_luma8().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_rgba8().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_luma16().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_rgb16().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_rgba16().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_rgb32f().map(|e|enumerate_pixels(e, |_, _, _|{}));
    // let _ = img.as_rgba32f().map(|e|enumerate_pixels(e, |_, _, _|{}));

    println!("Image: {file_path}, {:?}", img.dimensions());
    for format in IMAGE_FORMATS_WRITE
        .iter()
    {
        let buffer: Vec<u8> = Vec::new();
        println!("Before write_to {format:?}");
        if let Err(e) = img.write_to(&mut Cursor::new(buffer), *format) {
            eprintln!("Error: {}", e);
        };
        println!("After write_to {format:?}");
    }
}

// fn enumerate_pixels<P, Container, F>(img: &ImageBuffer<P, Container>, mut f: F)
// where
//     P: Pixel,
//     Container: Deref<Target = [P::Subpixel]>,
//     F: FnMut(u32, u32, &P),
// {
//     for (x, y, pixel) in img.enumerate_pixels() {
//         println!("Image pixel");
//         f(x, y, pixel);
//     }
// }
