#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use zune_core::bytestream::ZCursor;
use zune_core::options::DecoderOptions;
use zune_image::image::Image;

fuzz_target!(|data: &[u8]| -> Corpus {
    if check_file(data) {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});


fn check_file(file_content: &[u8]) -> bool {
    let mut success = false;
    for options in [DecoderOptions::new_fast(), DecoderOptions::new_cmd(), DecoderOptions::new_safe()] {
        let img = Image::read(ZCursor::new(&file_content), options);
        match img {
            Ok(_) => {
                success = true;
            }
            Err(_) => {            }
        }
    }
    success
}