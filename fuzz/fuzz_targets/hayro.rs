#![no_main]

use hayro_render::render_png;
use hayro_syntax::pdf::Pdf;
use libfuzzer_sys::{Corpus, fuzz_target};
use std::io::Read;
use std::sync::Arc;

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Some(pdf) = Pdf::new(Arc::new(data.to_vec())) {
        let _pixmaps = render_png(&pdf, 1.0, None);
    }
    Corpus::Keep
});
