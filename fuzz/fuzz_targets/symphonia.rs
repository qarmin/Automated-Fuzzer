#![no_main]

use std::env::args;
use std::path::Path;
use std::{fs, io};

use libfuzzer_sys::{Corpus, fuzz_target};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

fuzz_target!(|data: &[u8]| -> Corpus {
    if parse_audio_file(data.to_vec()).is_ok() {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});

pub(crate) fn parse_audio_file(content: Vec<u8>) -> Result<(), String> {
    let cursor = io::Cursor::new(content);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let fmt_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let meta_opts: MetadataOptions = Default::default();
    let hint = Hint::new();

    let probed = match symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts) {
        Ok(format) => format,
        Err(e) => {
            return Err(e.to_string());
        }
    };

    let opts = DecodeOptions {
        decoder_opts: AudioDecoderOptions {
            verify: true,
            ..Default::default()
        },
    };
    decode_only(probed, opts)?;

    Ok(())
}

#[derive(Copy, Clone)]
struct DecodeOptions {
    decoder_opts: AudioDecoderOptions,
}

fn decode_only(mut reader: Box<dyn FormatReader>, opts: DecodeOptions) -> Result<(), String> {
    let track = reader.default_track(TrackType::Audio);

    let track = match track {
        Some(track) => track,
        _ => return Ok(()),
    };

    let codec_params = match track.codec_params.as_ref() {
        Some(CodecParameters::Audio(audio)) => audio,
        _ => return Ok(()),
    };

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec_params, &opts.decoder_opts)
        .map_err(|e| e.to_string())?;

    let track_id = track.id;

    loop {
        let Some(packet) = reader.next_packet().map_err(|e| e.to_string())? else {
            break;
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(_decoded) => continue,
            Err(Error::DecodeError(err)) => eprintln!("decode error: {}", err),
            Err(err) => return Err(err.to_string()),
        }
    }

    Ok(())
}
