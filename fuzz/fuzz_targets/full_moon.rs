#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    let Ok(data) = String::from_utf8(data.to_vec()) else {
        return Corpus::Reject;
    };

    full_moon::parse_fallible(&data, full_moon::LuaVersion::new());
    Corpus::Keep
});
