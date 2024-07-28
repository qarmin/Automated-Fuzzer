#![no_main]

use std::io::Cursor;

use image::ImageFormat;
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    let res = match image::load_from_memory(&data) {
        Ok(res) => res,
        Err(_e) => {
            return Corpus::Reject;
        }
    };

    for format in [
        ImageFormat::Bmp,
        ImageFormat::Farbfeld,
        ImageFormat::Ico,
        ImageFormat::Jpeg,
        ImageFormat::Png,
        ImageFormat::Pnm,
        ImageFormat::Tiff,
        ImageFormat::WebP,
        ImageFormat::Tga,
        ImageFormat::Dds,
        ImageFormat::Hdr,
        ImageFormat::OpenExr,
        // ImageFormat::Avif, // Don't use, it is really slow https://github.com/image-rs/image/issues/2282
        ImageFormat::Qoi,
    ]
        .into_iter()
    {
        let buffer: Vec<u8> = Vec::new();
        let _ = res.write_to(&mut Cursor::new(buffer), format);
    }
    Corpus::Keep
});
