use std::fs;

use zune_core::bytestream::ZCursor;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(file_content) = fs::read(path) else {
        return;
    };

    for options in [
        DecoderOptions::new_fast(),
        DecoderOptions::new_cmd(),
        DecoderOptions::new_safe(),
    ] {
        let img = Image::read(ZCursor::new(&file_content), options);
        if let Ok(_) = img {
            println!("Successfully checked file: {path}");
        }
    }
}
