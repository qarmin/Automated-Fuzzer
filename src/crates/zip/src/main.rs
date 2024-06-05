use std::env::args;
use std::fs::File;

fn main() {
    let path = args().nth(1).unwrap().clone();
    match File::open(&path) {
        Ok(file) => {
            if let Err(e) = zip::ZipArchive::new(file) {
                println!("Failed to open zip file {e}");
            }
        }
        Err(_inspected) => (),
    }
}
