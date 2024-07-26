use std::io::BufReader;
use afl::fuzz;
use lofty::file::TaggedFileExt;
use lofty::file::AudioFile;
use lofty::probe::Probe;

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(s) = std::str::from_utf8(data) {
            let s = std::io::Cursor::new(s);
            let tagged_file = match Probe::new(BufReader::new(s)).read() {
                Ok(t) => t,
                Err(_e) => {
                    return;
                }
            };
            tagged_file.properties();
            tagged_file.tags();
            tagged_file.primary_tag();
        }
    });
}
