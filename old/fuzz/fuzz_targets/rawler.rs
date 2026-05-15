#![no_main]

use rawler::decoders::{ RawDecodeParams, RawMetadata};
use rawler::imgop::develop::{Intermediate, RawDevelop};
use rawler::rawsource::RawSource;
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    match get_raw_file(data) {
        Ok((_int, _met)) => {
            Corpus::Keep
        }
        Err(_e) => {
            Corpus::Reject
        },
    }

});

fn get_raw_file(content: &[u8]) -> Result<(Intermediate, RawMetadata), String> {
    let raw_source = RawSource::new_from_slice(content);
    let decoder = rawler::get_decoder(&raw_source).map_err(|e| e.to_string())?;
    let metadata = decoder
        .raw_metadata(&raw_source, &RawDecodeParams::default())
        .map_err(|e| e.to_string())?;
    let raw_image = decoder
        .raw_image(&raw_source, &RawDecodeParams::default(), false)
        .map_err(|e| e.to_string())?;
    let developer = RawDevelop::default();
    let developed_image = developer.develop_intermediate(&raw_image).map_err(|e| e.to_string())?;
    Ok((developed_image, metadata))
}