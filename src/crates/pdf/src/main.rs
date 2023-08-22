use pdf::file::FileOptions;
use pdf::object::Resolve;
use std::env::args;

fn main() {
    let path = args().nth(1).unwrap().clone();
    // let parser_options = ParseOptions {
    //     allow_error_in_option: true,
    //     allow_xref_error: true,
    //     allow_invalid_ops: true,
    //     allow_missing_endobj: true,
    // };
    // TODO re-enable
    match FileOptions::cached().open(&path) {
        Ok(file) => {
            for idx in 0..file.num_pages() {
                if let Ok(page) = file.get_page(idx) {
                    let _ = page.media_box();
                    let _ = page.crop_box();
                    let _ = page.resources();
                }
                let _ = file.options();
                let _ = file.get_root();
            }
        }
        Err(e) => println!("{}    -     {:?}", path, e),
    }
}
