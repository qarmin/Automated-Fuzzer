#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use nom_exif::{ExifIter, MediaParser, MediaSource};
use std::io::Cursor;

fuzz_target!(|data: &[u8]| -> Corpus {
    let mut parser = MediaParser::new();

    // Parse unseekable
    let reader = Cursor::new(data);
    let Ok(ms) = MediaSource::unseekable(reader) else {
        return Corpus::Reject;
    };
    let iter: Result<ExifIter, _> = parser.parse_exif(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps();
        for s in iter {
            let _ = s.value();
            let _ = s.result();
            s.tag();
        }
    }

    // Parse seekable
    let reader = Cursor::new(data);
    let Ok(ms) = MediaSource::seekable(reader) else {
        return Corpus::Reject;
    };
    let iter: Result<ExifIter, _> = parser.parse_exif(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps();
        for s in iter {
            let _ = s.value();
            let _ = s.result();
            s.tag();
        }
    }

    Corpus::Keep
});
