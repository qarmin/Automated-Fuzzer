#![no_main]

use std::fmt::Debug;
use libfuzzer_sys::{fuzz_target, Corpus};
use nom_exif::{ExifIter, MediaParser, MediaSource, TrackInfo};
use std::io::Cursor;

fuzz_target!(|data: &[u8]| -> Corpus {
    let mut parser = MediaParser::new();

    // Parse unseekable
    let reader = Cursor::new(data);
    let Ok(ms) = MediaSource::unseekable(reader) else {
        return Corpus::Reject;
    };
    let iter: Result<ExifIter, _> = parser.parse(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps_info();
        for i in iter {
            let s = i;
            s.tag_code();
            s.get_value();
            let _ = s.get_result();
            s.tag();
            s.ifd_index();
            s.has_value();
        }
    }

    let reader = Cursor::new(data);
    let Ok(ms) = MediaSource::unseekable(reader) else {
        return Corpus::Reject;
    };
    let _: Result<TrackInfo, _> = parser.parse(ms);

    // Parse seekable
    let reader = Cursor::new(data);
    let Ok(ms) = MediaSource::seekable(reader) else {
        return Corpus::Reject;
    };
    let iter: Result<ExifIter, _> = parser.parse(ms);
    if let Ok(iter) = iter {
        let _ = iter.parse_gps_info();
        for i in iter {
            let s = i;
            s.tag_code();
            s.get_value();
            let _ = s.get_result();
            s.tag();
            s.ifd_index();
            s.has_value();
        }
    }

    let reader = Cursor::new(data);
    let Ok(ms) = MediaSource::seekable(reader) else {
        return Corpus::Reject;
    };
    let _: Result<TrackInfo, _> = parser.parse(ms);

    Corpus::Keep
});
