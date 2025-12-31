#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use little_exif::metadata::Metadata;
use little_exif::filetype::FileExtension;

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.is_empty() {
        return Corpus::Reject;
    }

    let data_vec = data.to_vec();

    // Try different file formats
    let formats = [
        FileExtension::JPEG,
        FileExtension::PNG { as_zTXt_chunk: false },
        FileExtension::PNG { as_zTXt_chunk: true },
        FileExtension::WEBP,
        FileExtension::TIFF,
        FileExtension::HEIF,
        FileExtension::JXL,
    ];

    for format in formats.iter() {
        let _ = Metadata::new_from_vec(&data_vec, *format);
    }

    Corpus::Keep
});

