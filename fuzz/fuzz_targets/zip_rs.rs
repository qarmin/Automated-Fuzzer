#![no_main]

use std::io::Read;
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]|  -> Corpus{
    let cursor = std::io::Cursor::new(data);
    let mut zip = match zip::ZipArchive::new(cursor) {
        Ok(t) => t,
        Err(_e) => {
            return Corpus::Reject;
        }
    };

    for i in 0..zip.len() {
        match zip.by_index(i) {
            Ok(mut file) => {
                let mut buf = Vec::new();
                let _ = file.read(&mut buf);
            }
            Err(_e) => {
            }
        }
    }
    Corpus::Keep
});
