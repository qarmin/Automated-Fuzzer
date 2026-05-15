#![no_main]

use dicom_dump::DumpOptions;
use dicom_object::from_reader;
use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    let cursor = std::io::Cursor::new(data);
    let Ok(res) = from_reader(cursor) else {
        return Corpus::Reject;
    };
    let mut item_to_dump = Vec::new();
    let _ = DumpOptions::new().dump_object_to(&mut item_to_dump, &res);
    let _ = dicom_json::to_string(&res);
    let mut item_to_dump = Vec::new();
    if res.write_all(&mut item_to_dump).is_ok() {
        // if item_to_dump != data {
        //     panic!("DIFFERENT CONTENT, expected: {}, got: {}", data.len(), item_to_dump.len());
        // }
    }
    Corpus::Keep
});
