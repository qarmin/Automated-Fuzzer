use std::fs;
use std::io::Cursor;

use image::{GenericImageView, ImageFormat};
use image::math::Rect;

const IMAGE_FORMATS_READ: &[ImageFormat] = &[
    ImageFormat::Png,
    ImageFormat::Jpeg,
    ImageFormat::Gif,
    ImageFormat::WebP,
    ImageFormat::Pnm,
    ImageFormat::Tiff,
    ImageFormat::Tga,
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
    ImageFormat::Bmp,
    ImageFormat::Ico,
    ImageFormat::Hdr,
    ImageFormat::OpenExr,
    ImageFormat::Farbfeld,
    // ImageFormat::Avif, // Don't use, it is really slow
    ImageFormat::Qoi,
];

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };

    // Try guessing format from content
    if let Ok(guessed) = image::guess_format(&content) {
        eprintln!("Guessed format: {guessed:?}");
    }

    // Try loading with each format
    let mut last_img = None;
    for format in IMAGE_FORMATS_READ.iter() {
        match image::load_from_memory_with_format(&content, *format) {
            Ok(img) => {
                exercise_image(&img);
                last_img = Some(img);
            }
            Err(e) => {
                eprintln!("Read {format:?}: {e}");
            }
        }
    }

    // Also try auto-detect loading
    match image::load_from_memory(&content) {
        Ok(img) => {
            exercise_image(&img);
            last_img = Some(img);
        }
        Err(e) => {
            eprintln!("Auto-detect load: {e}");
        }
    }

    let Some(img) = last_img else {
        return;
    };

    println!("Image: {file_path}, {:?}", img.dimensions());

    // Try writing to each format
    for format in IMAGE_FORMATS_WRITE.iter() {
        let buffer: Vec<u8> = Vec::new();
        if let Err(e) = img.write_to(&mut Cursor::new(buffer), *format) {
            eprintln!("Write {format:?}: {e}");
        }
    }
}

fn exercise_image(img: &image::DynamicImage) {
    let (w, h) = img.dimensions();
    let _ = img.color();

    // Sample a pixel if the image is non-empty
    if w > 0 && h > 0 {
        let _ = img.get_pixel(0, 0);
        let _ = img.get_pixel(w / 2, h / 2);
        let _ = img.get_pixel(w - 1, h - 1);
    }

    // Try color conversions
    let _ = img.to_rgb8();
    let _ = img.to_rgba8();
    let _ = img.to_luma8();
    let _ = img.to_luma_alpha8();
    let _ = img.to_rgb16();
    let _ = img.to_rgba16();
    let _ = img.to_luma16();

    // Try basic transformations
    let _ = img.fliph();
    let _ = img.flipv();
    let _ = img.rotate90();
    let _ = img.rotate180();
    let _ = img.rotate270();

    // Try cropping (small region to avoid huge allocations)
    if w > 2 && h > 2 {
        let crop_w = w.min(16);
        let crop_h = h.min(16);
        let _ = img.crop(Rect { x: 0, y: 0, width: crop_w, height: crop_h });
    }

    // Try resizing (small target to avoid huge allocations)
    if w > 0 && h > 0 {
        let _ = img.thumbnail(8, 8);
    }

    // Try grayscale conversion
    let _ = img.grayscale();

    // Try blur with small sigma
    let _ = img.blur(1.0);

    // Try adjusting brightness/contrast
    let _ = img.brighten(10);
    let _ = img.adjust_contrast(10.0);
}
