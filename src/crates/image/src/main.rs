use std::env::args;
use std::fs;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::options::DecoderOptions;

fn main() {
    let path = args().nth(1).unwrap();
    // let output = image::open(&path);

    let options = DecoderOptions::default()
        .set_strict_mode(false)
        .set_max_width(usize::MAX)
        .set_max_height(usize::MAX);
    let Ok(data) = fs::read(&path) else {
        return;
    };
    let cursor = ZCursor::new(data);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(cursor, options);
    let output = decoder.decode();

    if let Err(e) = output {
        println!("{e}");
    }
}
