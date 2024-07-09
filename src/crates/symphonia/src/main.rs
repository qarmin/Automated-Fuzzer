use std::env::args;
use std::fs::File;
use std::io;

use walkdir::WalkDir;
use std::path::Path;
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::errors::Error;
use symphonia::core::errors::Error::IoError;
use symphonia::core::io::MediaSourceStream;

fn main() {
    let path = args().nth(1).unwrap().clone();

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            check_file(&path);
        }
    } else {
        check_file(&path);
    }
}

fn check_file(path: &str) {
    let file = match File::open(&path) {
        Ok(t) => t,
        Err(e) => {
            println!("{e}");
            return;
        }
    };

    let _ = parse_audio_file(file);
}

pub fn parse_audio_file(file_handler: File) -> Result<(), Error> {
    let mss = MediaSourceStream::new(Box::new(file_handler), Default::default());

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
