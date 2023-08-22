use rustpython_parser::mode::Mode;
use rustpython_parser::{ast, parser};
use std::env::args;

fn main() {
    let path = args().nth(1).unwrap().clone();

    let Ok(content) = std::fs::read_to_string(&path) else {
        return
    };

    let _ = match parser::parse(content.as_str(), Mode::Module, "") {
        Ok(t) => t,
        Err(_) => return,
    };
}
