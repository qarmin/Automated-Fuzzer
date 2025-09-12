#![no_main]

use std::io::Cursor;

use image::ImageFormat;
use libfuzzer_sys::{fuzz_target, Corpus};

const IMAGE_FORMATS_READ: &[ImageFormat] = &[
    ImageFormat::Png,
    ImageFormat::Jpeg,
    ImageFormat::Gif,
    ImageFormat::WebP,
    ImageFormat::Pnm,
    ImageFormat::Tiff,
    // ImageFormat::Tga, // TODO - https://github.com/image-rs/image/issues/2602
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
    // ImageFormat::Tga, // TODO - https://github.com/image-rs/image/issues/2602
    ImageFormat::Dds,
    // ImageFormat::Bmp,
    ImageFormat::Ico,
    ImageFormat::Hdr,
    ImageFormat::OpenExr,
    ImageFormat::Farbfeld,
    // ImageFormat::Avif, // Don't use, it is really slow
    ImageFormat::Qoi,
];

fuzz_target!(|data: &[u8]| -> Corpus {
    let mut img = None;

    for format in IMAGE_FORMATS_READ.iter() {
        let res = image::load_from_memory_with_format(&data, *format);
        if let Ok(res) = res {
            img = Some(res);
        }
    }

    let img = match img {
        Some(img) => img,
        None => return Corpus::Reject,
    };

    for format in IMAGE_FORMATS_WRITE.iter()
    {
        let buffer: Vec<u8> = Vec::new();
        let _ = img.write_to(&mut Cursor::new(buffer), *format);
    }
    Corpus::Keep
});
