#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(data) = String::from_utf8(data.to_vec()) else {
        return;
    };

    let _r = full_moon::parse_fallible(&data, full_moon::LuaVersion::new());
});
