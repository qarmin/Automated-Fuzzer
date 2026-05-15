use std::path::PathBuf;

use font_kit::handle::Handle;
use font_kit::source::Source;
use font_kit::sources::mem::MemSource;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let handle = Handle::Path {
        path: PathBuf::from(path),
        font_index: 0,
    };
    match MemSource::from_fonts([handle.clone()].into_iter()) {
        Ok(mut source) => {
            let _ = source.all_families();
            let _ = source.select_best_match(&[], &font_kit::properties::Properties::new());
            let _ = source.select_by_postscript_name("");
            let _ = source.as_any();
            let _ = source.add_font(handle);
            let _ = source.all_families();
            let _ = source.select_best_match(&[], &font_kit::properties::Properties::new());
            let _ = source.select_by_postscript_name("");
            let _ = source.as_any();
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}
