use std::fs;

use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, TagExt};

fn main() {
    fuzz_utils::run(check_file);
}

const ALL_FILE_TYPES: &[FileType] = &[
    FileType::Aac,
    FileType::Aiff,
    FileType::Ape,
    FileType::Flac,
    FileType::Mpeg,
    FileType::Mp4,
    FileType::Mpc,
    FileType::Opus,
    FileType::Vorbis,
    FileType::Speex,
    FileType::Wav,
    FileType::WavPack,
];

fn check_file(path: &str) {
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(e) => {
            println!("{e}");
            return;
        }
    };

    // Try each file type explicitly (like the upstream fuzz target)
    for file_type in ALL_FILE_TYPES {
        let cursor = std::io::Cursor::new(&content);
        let tagged_file = match Probe::with_file_type(cursor, *file_type).read() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };

        // Read properties
        let props = tagged_file.properties();
        let _ = props.duration();
        let _ = props.overall_bitrate();
        let _ = props.audio_bitrate();
        let _ = props.sample_rate();
        let _ = props.bit_depth();
        let _ = props.channels();
        let _ = props.channel_mask();

        // Read all tags
        for tag in tagged_file.tags() {
            let _ = tag.tag_type();
            let _ = tag.len();
            let _ = tag.is_empty();

            // Iterate all items in each tag
            for item in tag.items() {
                let _ = item.key().clone();
                let _ = item.value().clone();
            }

            // Try to read common items via Accessor trait
            let _ = tag.title();
            let _ = tag.artist();
            let _ = tag.album();
            let _ = tag.genre();
            let _ = tag.comment();
            let _ = tag.track();
            let _ = tag.track_total();
            let _ = tag.disk();
            let _ = tag.disk_total();

            // Read pictures
            for pic in tag.pictures() {
                let _ = pic.pic_type();
                let _ = pic.mime_type();
                let _ = pic.data();
                let _ = pic.description();
            }
        }

        // Read primary tag specifically
        if let Some(primary) = tagged_file.primary_tag() {
            let _ = primary.tag_type();
            let _ = primary.len();
            let _ = primary.title();
            let _ = primary.artist();
        }
    }

    // Also try auto-detection via Probe (without specifying file type)
    let cursor = std::io::Cursor::new(&content);
    if let Ok(tagged_file) = Probe::new(cursor).read() {
        let _ = tagged_file.file_type();
        let _ = tagged_file.properties();
        let _ = tagged_file.tags();
        let _ = tagged_file.primary_tag();
    }
}
