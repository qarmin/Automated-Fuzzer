#![no_main]

use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    let Ok(data) = String::from_utf8(data.to_vec()) else {
        return Corpus::Reject;
    };

     full_moon::parse_fallible(&data, full_moon::LuaVersion::new());
    Corpus::Keep
});
