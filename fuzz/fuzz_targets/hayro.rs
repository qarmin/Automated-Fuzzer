#![no_main]

use hayro_render::{InterpreterSettings, render_png};
use hayro_syntax::pdf::Pdf;
use libfuzzer_sys::{Corpus, fuzz_target};
use std::sync::Arc;

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Some(pdf) = Pdf::new(Arc::new(data.to_vec())) {
        let pages = pdf.pages();
        let pixmaps = render_png(&pdf, 1.0, InterpreterSettings::default(), None);
        return Corpus::Keep;
    }
    Corpus::Reject
});
