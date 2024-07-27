#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::BufReader;
use lofty::file::TaggedFileExt;
use lofty::file::AudioFile;
use lofty::probe::Probe;

fuzz_target!(|data: &[u8]| {
    let s = std::io::Cursor::new(data);
    let tagged_file = match Probe::new(BufReader::new(s)).read() {
        Ok(t) => t,
        Err(_e) => {
            return;
        }
    };
    tagged_file.properties();
    tagged_file.tags();
    tagged_file.primary_tag();
});
