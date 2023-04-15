use std::env::args;
use pdf::file::FileOptions;

fn main() {
    let path = args().nth(1).unwrap().clone();
    // let parser_options = ParseOptions {
    //     allow_error_in_option: true,
    //     allow_xref_error: true,
    //     allow_invalid_ops: true,
    //     allow_missing_endobj: true,
    // };
    // TODO re-enable
    if let Err(e) = FileOptions::cached()
        // .parse_options(parser_options)
        .open(&path)
    {
        println!("{}    -     {:?}", path, e);
    } else {
        //     // println!("VALID   {}", path);
    }
}
