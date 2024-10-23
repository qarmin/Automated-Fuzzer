#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use lofty::file::AudioFile;
use lofty::file::{FileType, TaggedFileExt};
use lofty::probe::Probe;

const ALL_FILE_TYPES: &[FileType] = &[
    FileType::Aac,
    FileType::Aiff,
    FileType::Ape,
    FileType::Flac,
    FileType::Mpeg,
    FileType::Mp4,
    // FileType::Mpc,  // https://github.com/Serial-ATA/lofty-rs/issues/470
    FileType::Opus,
    FileType::Vorbis,
    FileType::Speex,
    // FileType::Wav, // TODO
    // FileType::WavPack, // TODO
];

fuzz_target!(|data: &[u8]| -> Corpus {
    let mut corpus = Corpus::Reject;
    for i in ALL_FILE_TYPES {
        let s = std::io::Cursor::new(data);
        let tagged_file = match Probe::with_file_type(s, *i).read() {
            Ok(t) => t,
            Err(_e) => {
                continue;
            }
        };
        corpus = Corpus::Keep;
        tagged_file.properties();
        tagged_file.tags();
        tagged_file.primary_tag();
    }

    corpus
});
