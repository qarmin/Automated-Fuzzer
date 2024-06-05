use dicom_object::open_file;
use std::env::args;

fn main() {
    let path = args().nth(1).unwrap().clone();
    let res = open_file(path);
    if let Err(e) = res {
        // eprintln!("Error: {}", e);
    } else {
        // println!("Result: {:?}", res);
    }
    // dbg!(res);
}
