use std::fs;

use rusty_chromaprint::Configuration;
use symphonia::core::formats::FormatReader;
fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(content) = fs::read(path) else {
        return;
    };

    println!("Checking file: {:?}", path);
    for config in &[
        Configuration::preset_test1(),
        Configuration::preset_test2(),
        Configuration::preset_test3(),
        Configuration::preset_test4(),
        Configuration::preset_test5(),
    ] {
        if let Err(e) = calc_fingerprint_helper(content.clone(), config) {
            eprintln!("Error with config for file {path}: {e}");
        }
    }
}

fn calc_fingerprint_helper(data: Vec<u8>, config: &Configuration) -> Result<(), String> {
    use std::io;

    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let cursor = io::Cursor::new(data);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let hint = Hint::new();
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts = FormatOptions::default();

    let probed = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .map_err(|e| e.to_string())?;

    decode_only(probed, config)
}

fn decode_only(
    mut reader: Box<dyn symphonia::core::formats::FormatReader>,
    _config: &Configuration,
) -> Result<(), String> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::codecs::CodecParameters;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::TrackType;

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

        if packet.track_id != track_id {
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
