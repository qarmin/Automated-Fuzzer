use std::fs;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(file_path: &str) {
    match fs::read(file_path) {
        Ok(bytes) => {
            if let Err(e) = pdf_extract::extract_text_from_mem(&bytes) {
                println!("Error {}", e);
            }
        }
        Err(e) => println!("Error {}", e),
    }
}
