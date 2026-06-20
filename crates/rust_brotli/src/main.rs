use std::fs;
use std::io::{self, Write};

const MAX_OUTPUT: usize = 64 * 1024 * 1024;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else {
        return;
    };

    let mut input = data.as_slice();
    let mut output = BoundedWriter::default();
    let _ = brotli::BrotliDecompress(&mut input, &mut output);
}

#[derive(Default)]
struct BoundedWriter {
    written: usize,
}

impl Write for BoundedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written += buf.len();
        if self.written > MAX_OUTPUT {
            return Err(io::Error::other("output limit exceeded"));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
