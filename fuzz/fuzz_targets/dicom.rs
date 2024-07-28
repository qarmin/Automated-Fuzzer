#![no_main]

use dicom_dump::DumpOptions;
use dicom_object::from_reader;
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    let cursor = std::io::Cursor::new(data);
    let Ok(res) = from_reader(cursor) else {
        return Corpus::Reject;
    };
    let mut item_to_dump = Vec::new();
    let _ = DumpOptions::new().dump_object_to(&mut item_to_dump, &res);
    let _ = dicom_json::to_string(&res);
    let mut item_to_dump = Vec::new();
    let _ = res.write_dataset(&mut item_to_dump);
    Corpus::Keep
});
