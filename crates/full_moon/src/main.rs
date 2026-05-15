use std::fs;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read_to_string(path) else {
        return;
    };

    let r = full_moon::parse_fallible(&data, full_moon::LuaVersion::new());
    if !r.errors().is_empty() {
        println!("Error: {:?}", r.errors());
    }
}
