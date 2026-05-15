use std::fs;
use std::io::Read;

fn main() {
    fuzz_utils::run(check_file);
}
fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    let cursor = std::io::Cursor::new(content);
    let mut zip = match zip::ZipArchive::new(cursor) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    for i in 0..zip.len() {
        match zip.by_index(i) {
            Ok(mut file) => {
                let mut buf = Vec::new();
                let _ = file.read(&mut buf);
            }
            Err(e) => {
                eprintln!("{e}");
            }
        }
    }
}
