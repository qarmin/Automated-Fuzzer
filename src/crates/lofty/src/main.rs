use std::env::args;
use std::fs::File;

use lofty::TaggedFileExt;
use lofty::{read_from, AudioFile};

fn main() {
    let path = args().nth(1).unwrap().clone();
    let mut file = match File::open(&path) {
        Ok(t) => t,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    let tagged_file = match read_from(&mut file) {
        Ok(t) => t,
        Err(e) => {
            println!("{e}");
            return;
        }
    };

    // let Ok(mut file) = File::open(&path) else { return; };
    // let Ok(tagged_file) = read_from(&mut file) else { return; };

    tagged_file.properties();
    tagged_file.tags();
    tagged_file.primary_tag();
}
