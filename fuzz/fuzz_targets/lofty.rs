#![no_main]

use libfuzzer_sys::{Corpus, fuzz_target};
use lofty::file::{FileType, TaggedFileExt};
use lofty::file::AudioFile;
use lofty::probe::Probe;

const ALL_FILE_TYPES: &[FileType] = &[FileType::Aac, FileType::Aiff, FileType::Ape, FileType::Flac, FileType::Mpeg, FileType::Mp4, FileType::Mpc, FileType::Opus, FileType::Vorbis, FileType::Speex, FileType::Wav, FileType::WavPack, FileType::Custom("custom")];

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
