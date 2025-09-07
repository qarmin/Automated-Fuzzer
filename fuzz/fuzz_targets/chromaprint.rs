#![no_main]

// TODO - for now it fails due several crashes in symphonia

use libfuzzer_sys::{fuzz_target, Corpus};

use rusty_chromaprint::Configuration;

// Use a static for DEFAULT_CONFIG, or just call the function directly in the fuzzer closure
// static DEFAULT_CONFIG: Configuration = Configuration::preset_test1();

fuzz_target!(|data: &[u8]| -> Corpus {
    let config = Configuration::preset_test1();
    if let Ok(()) = calc_fingerprint_helper(data.to_vec(), &config) {
        return Corpus::Keep;
    }
    Corpus::Reject
});

fn calc_fingerprint_helper(data: Vec<u8>, config: &Configuration) -> Result<(), String> {
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use std::io;

    let cursor = io::Cursor::new(data);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let hint = Hint::new();
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };

    let probed = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .map_err(|e| e.to_string())?;

    decode_only(probed, config)
}

fn decode_only(mut reader: Box<dyn symphonia::core::formats::FormatReader>, _config: &Configuration) -> Result<(), String> {
    use symphonia::core::formats::TrackType;
    use symphonia::core::codecs::CodecParameters;
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error;

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
        .make_audio_decoder(codec_params, &AudioDecoderOptions::default())
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
            Err(Error::DecodeError(_)) => (),
            Err(_) => break,
        }
    }

    Ok(())
}