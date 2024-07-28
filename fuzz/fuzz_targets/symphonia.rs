#![no_main]

use std::{fs, io};
use std::env::args;
use std::path::Path;

use libfuzzer_sys::{Corpus, fuzz_target};
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::errors::Error;
use symphonia::core::errors::Error::IoError;
use symphonia::core::io::MediaSourceStream;

fuzz_target!(|data: &[u8]| -> Corpus {
     if parse_audio_file(data.to_vec()).is_ok() {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});

pub fn parse_audio_file(content: Vec<u8>) -> Result<(), Error> {
    let cursor = io::Cursor::new(content);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let probed = match symphonia::default::get_probe().format(
        &Default::default(),
        mss,
        &Default::default(),
        &Default::default(),
    ) {
        Ok(t) => t,
        Err(_) => {
            return Err(Error::Unsupported(
                "probe info not available/file not recognized",
            ))
        }
    };

    let mut format = probed.format;

    let track = match format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
    {
        Some(k) => k,
        None => return Err(Error::Unsupported("not supported audio track")),
    };

    let mut decoder =
        match symphonia::default::get_codecs().make(&track.codec_params, &Default::default()) {
            Ok(k) => k,
            Err(_) => return Err(Error::Unsupported("not supported codec")),
        };

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                return Err(Error::ResetRequired);
            }
            Err(err) => {
                if let IoError(ref er) = err {
                    // Catch eof, not sure how to do it properly
                    if er.kind() == io::ErrorKind::UnexpectedEof {
                        return Ok(());
                    }
                }
                return Err(err);
            }
        };

        decoder.decode(&packet)?;
    }
}
