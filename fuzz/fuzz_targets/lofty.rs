#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::BufReader;
use lofty::file::TaggedFileExt;
use lofty::file::AudioFile;
use lofty::probe::Probe;

fuzz_target!(|data: &[u8]| {
    // unsafe {
    //     let null_ptr: *const i32 = std::ptr::null();
    //     let _ = *null_ptr;
    // }
    let s = std::io::Cursor::new(data.to_vec());
    let tagged_file = match Probe::new(BufReader::new(s)).read() {
        Ok(t) => t,
        Err(_e) => {
            return;
        }
    };
    // Null pointer dereference
    tagged_file.properties();
    tagged_file.tags();
    tagged_file.primary_tag();
});
