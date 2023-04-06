use std::env::args;

fn main() {
    let output = image::open(&args().nth(1).unwrap());
    if let Err(e) = output {
        println!("{e}");
    }
}
