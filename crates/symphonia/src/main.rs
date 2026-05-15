use std::env::args;
use std::path::Path;
use std::{fs, io};

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
    }

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
    let Ok(content) = fs::read(path) else {
        return;
    };

    println!("Checking file: {:?}", path);
    if let Err(e) = parse_audio_file(content) {
        eprintln!("{e}");
    };
}

pub fn parse_audio_file(content: Vec<u8>) -> Result<(), String> {
    let cursor = io::Cursor::new(content);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let fmt_opts = FormatOptions::default();
    let meta_opts: MetadataOptions = Default::default();
    let hint = Hint::new();

    let mut probed = match symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts) {
        Ok(format) => format,
        Err(e) => {
            return Err(e.to_string());
        }
    };

    // Exercise metadata reading
    read_metadata(&mut *probed);

    let decoder_opts = AudioDecoderOptions::default().verify(true);

    // Collect all track IDs and their codec params for audio tracks
    let mut audio_tracks: Vec<(u32, _)> = Vec::new();
    for track in probed.tracks() {
        let track_id = track.id;
        if let Some(CodecParameters::Audio(audio_params)) = track.codec_params.as_ref() {
            audio_tracks.push((track_id, audio_params.clone()));
        }
    }

    // Create decoders for all audio tracks
    let mut decoders: Vec<(u32, Box<dyn symphonia::core::codecs::audio::AudioDecoder>)> = Vec::new();
    for (track_id, audio_params) in &audio_tracks {
        match symphonia::default::get_codecs().make_audio_decoder(audio_params, &decoder_opts) {
            Ok(decoder) => {
                decoders.push((*track_id, decoder));
            }
            Err(e) => {
                eprintln!("Failed to create decoder for track {track_id}: {e}");
            }
        }
    }

    // Decode all packets, dispatching to the correct decoder
    loop {
        let packet = match probed.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                eprintln!("next_packet error: {e}");
                break;
            }
        };

        let pkt_track_id = packet.track_id;
        for (track_id, decoder) in decoders.iter_mut() {
            if *track_id == pkt_track_id {
                match decoder.decode(&packet) {
                    Ok(_decoded) => {}
                    Err(Error::DecodeError(err)) => eprintln!("decode error: {err}"),
                    Err(err) => {
                        eprintln!("fatal decode error: {err}");
                        break;
                    }
                }
            }
        }
    }

    // Try seeking to the beginning using Time
    let _ = probed.seek(
        SeekMode::Coarse,
        SeekTo::Time {
            time: Time::ZERO,
            track_id: None,
        },
    );

    // Try seeking past the beginning
    let _ = probed.seek(
        SeekMode::Accurate,
        SeekTo::Time {
            time: Time::ZERO,
            track_id: None,
        },
    );

    Ok(())
}

fn read_metadata(reader: &mut dyn FormatReader) {
    // Read metadata from the format reader
    if let Some(metadata) = reader.metadata().current() {
        for tag in &metadata.media.tags {
            let _ = tag.raw.key.clone();
            let _ = tag.raw.value.clone();
            if let Some(std_tag) = &tag.std {
                let _ = format!("{std_tag:?}");
            }
        }
        for visual in &metadata.media.visuals {
            let _ = visual.media_type.clone();
            let _ = visual.data.len();
            let _ = visual.dimensions;
        }
        for per_track in &metadata.per_track {
            let _ = per_track.track_id;
            for tag in &per_track.metadata.tags {
                let _ = tag.raw.key.clone();
                let _ = tag.raw.value.clone();
            }
            for visual in &per_track.metadata.visuals {
                let _ = visual.media_type.clone();
                let _ = visual.data.len();
            }
        }
    }
}
