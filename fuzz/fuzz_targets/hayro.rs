#![no_main]

use hayro::{render, InterpreterSettings, Pdf, RenderSettings};
use libfuzzer_sys::{Corpus, fuzz_target};
use std::sync::Arc;

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Ok(pdf) = Pdf::new(Arc::new(data.to_vec())) {
        for page in pdf.pages().iter() {
            let _pixmap = render(page, &InterpreterSettings::default(), &RenderSettings::default());
        }

        return Corpus::Keep;
    }
    Corpus::Reject
});
