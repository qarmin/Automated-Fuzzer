//! Fuzzing utilities — shared boilerplate for fuzz-target crates.
//!
//! Currently this crate exposes a single helper, [`run`], that handles the
//! "iterate every file argv[1] points at (file or directory, recursive)" pattern.
//!
//! ```ignore
//! fn main() {
//!     fuzz_utils::run(check_file);
//! }
//!
//! fn check_file(path: &str) {
//!     let data = std::fs::read(path).unwrap();
//!     // call library with data
//! }
//! ```
//!
//! Structured (API-driven) fuzz targets that need to translate raw bytes into
//! typed parameters should keep that logic inside their own crate and emit
//! standalone `.reproducer.rs` files there — see `crates/pdf_writer/src/main.rs`
//! for the reference pattern.

/// Run `f` on every file from argv (file or directory, recursive).
pub fn run(f: impl Fn(&str)) {
    let path = std::env::args().nth(1).expect("Usage: <binary> <file_or_dir>");
    let p = std::path::Path::new(&path);
    if !p.exists() {
        panic!("Missing file: {path:?}");
    }
    if p.is_dir() {
        for entry in walkdir::WalkDir::new(&path).into_iter().flatten() {
            if entry.file_type().is_file() {
                f(&entry.path().to_string_lossy());
            }
        }
    } else {
        f(&path);
    }
}
