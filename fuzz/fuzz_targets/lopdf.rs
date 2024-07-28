#![no_main]

use std::io::Cursor;
use libfuzzer_sys::{Corpus, fuzz_target};
use lopdf::Document;

fuzz_target!(|data: &[u8]| -> Corpus {
    let cursor = Cursor::new(data);
    match Document::load_from(cursor) {
        Ok(mut document) => {
            let pages = document.get_pages();

            let mut doc_clone = document.clone();
            doc_clone.decompress();

            for (i, _) in pages.iter().enumerate() {
                let page_number = (i + 1) as u32;
                let _text = document.extract_text(&[page_number]);
            }

            let _ = document.save_to(&mut Cursor::new(Vec::new()));
            Corpus::Keep
        }
        Err(_err) => {
            Corpus::Reject
        }
    }
});
