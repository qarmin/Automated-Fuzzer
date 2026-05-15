//! Structured fuzzing utilities.
//!
//! Reads a file of random bytes and uses them as deterministic "randomness"
//! to drive API calls. Logs every decision for reproducible crash reports.
//!
//! # Usage in a crate wrapper:
//! ```ignore
//! use fuzz_utils::ByteInput;
//!
//! fn check_data(input: &mut ByteInput) {
//!     let page_count = input.u8_range("page_count", 1, 10);
//!     let width = input.f32_range("width", 0.0, 1000.0);
//!     // ... call library APIs with these values
//! }
//! ```
//!
//! # How it works:
//! - Input file = raw bytes, consumed sequentially
//! - Each `input.xxx("name", ...)` call consumes bytes and logs the decision
//! - When bytes run out, `is_empty()` returns true → stop making calls
//! - On crash: the input file is the reproducer (deterministic)
//! - The log (`.params()`) shows human-readable parameter values
//! - `.generate_reproducer()` produces standalone Rust code with hardcoded values

use std::fmt::Write as FmtWrite;

/// A deterministic byte-driven input source that logs all consumed values.
pub struct ByteInput {
    data: Vec<u8>,
    pos: usize,
    log: Vec<ParamEntry>,
}

/// One logged parameter decision.
#[derive(Clone, Debug)]
pub struct ParamEntry {
    pub name: String,
    pub value: String,
    pub rust_expr: String,
}

impl ByteInput {
    /// Create from raw bytes (read from a file).
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            pos: 0,
            log: Vec::new(),
        }
    }

    /// Create from a file path.
    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self::new(data))
    }

    /// True when no more bytes are available.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Remaining bytes.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Get all logged parameter entries.
    pub fn params(&self) -> &[ParamEntry] {
        &self.log
    }

    /// Save the current parameters to a file (key=value format, human-readable).
    pub fn save_params(&self, path: &str) -> std::io::Result<()> {
        let mut out = String::new();
        for entry in &self.log {
            writeln!(out, "{}={}", entry.name, entry.value).unwrap();
        }
        std::fs::write(path, out)
    }

    /// Generate a standalone Rust reproducer snippet with hardcoded values.
    /// The `body_template` should contain `$NAME` placeholders that get replaced.
    pub fn generate_reproducer(&self, body_template: &str) -> String {
        let mut code = body_template.to_string();
        for entry in &self.log {
            let placeholder = format!("${}", entry.name);
            code = code.replace(&placeholder, &entry.rust_expr);
        }
        code
    }

    // ── Raw byte consumers ──

    fn consume_byte(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

    fn consume_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        if self.pos + n <= self.data.len() {
            let bytes = self.data[self.pos..self.pos + n].to_vec();
            self.pos += n;
            Some(bytes)
        } else {
            None
        }
    }

    // ── Typed consumers (each logs the decision) ──

    /// Consume a u8.
    pub fn u8(&mut self, name: &str) -> Option<u8> {
        let val = self.consume_byte()?;
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}u8"),
        });
        Some(val)
    }

    /// Consume a u8 in a range [min, max] (inclusive).
    pub fn u8_range(&mut self, name: &str, min: u8, max: u8) -> Option<u8> {
        let raw = self.consume_byte()?;
        let range = (max - min) as u16 + 1;
        let val = min + (raw as u16 % range) as u8;
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}u8"),
        });
        Some(val)
    }

    /// Consume a u16.
    pub fn u16(&mut self, name: &str) -> Option<u16> {
        let bytes = self.consume_bytes(2)?;
        let val = u16::from_le_bytes([bytes[0], bytes[1]]);
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}u16"),
        });
        Some(val)
    }

    /// Consume a u32.
    pub fn u32(&mut self, name: &str) -> Option<u32> {
        let bytes = self.consume_bytes(4)?;
        let val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}u32"),
        });
        Some(val)
    }

    /// Consume a u64.
    pub fn u64(&mut self, name: &str) -> Option<u64> {
        let bytes = self.consume_bytes(8)?;
        let val = u64::from_le_bytes(bytes.try_into().unwrap());
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}u64"),
        });
        Some(val)
    }

    /// Consume an i32.
    pub fn i32(&mut self, name: &str) -> Option<i32> {
        let bytes = self.consume_bytes(4)?;
        let val = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}i32"),
        });
        Some(val)
    }

    /// Consume a f32 in range [min, max].
    pub fn f32_range(&mut self, name: &str, min: f32, max: f32) -> Option<f32> {
        let bytes = self.consume_bytes(4)?;
        let raw = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let t = raw as f64 / u32::MAX as f64;
        let val = min as f64 + t * (max as f64 - min as f64);
        let val = val as f32;
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: format!("{val}"),
            rust_expr: format!("{val:.6}f32"),
        });
        Some(val)
    }

    /// Consume a f64 in range [min, max].
    pub fn f64_range(&mut self, name: &str, min: f64, max: f64) -> Option<f64> {
        let bytes = self.consume_bytes(8)?;
        let raw = u64::from_le_bytes(bytes.try_into().unwrap());
        let t = raw as f64 / u64::MAX as f64;
        let val = min + t * (max - min);
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: format!("{val}"),
            rust_expr: format!("{val:.6}f64"),
        });
        Some(val)
    }

    /// Consume a bool.
    pub fn bool(&mut self, name: &str) -> Option<bool> {
        let raw = self.consume_byte()?;
        let val = raw % 2 == 1;
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}"),
        });
        Some(val)
    }

    /// Pick one item from a slice by index.
    pub fn pick<'a, T: std::fmt::Debug>(&mut self, name: &str, options: &'a [T]) -> Option<&'a T> {
        if options.is_empty() {
            return None;
        }
        let raw = self.consume_byte()?;
        let idx = raw as usize % options.len();
        let val = &options[idx];
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: format!("{val:?}"),
            rust_expr: format!("/* {val:?} */"),
        });
        Some(val)
    }

    /// Pick an index from 0..len.
    pub fn index(&mut self, name: &str, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        let raw = if len <= 256 {
            self.consume_byte()? as usize
        } else {
            let bytes = self.consume_bytes(2)?;
            u16::from_le_bytes([bytes[0], bytes[1]]) as usize
        };
        let val = raw % len;
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: val.to_string(),
            rust_expr: format!("{val}usize"),
        });
        Some(val)
    }

    /// Consume up to `max_len` bytes as a raw byte slice.
    pub fn bytes(&mut self, name: &str, max_len: usize) -> Option<Vec<u8>> {
        let len_byte = self.consume_byte()? as usize;
        let len = len_byte.min(max_len).min(self.remaining());
        if len == 0 {
            self.log.push(ParamEntry {
                name: name.to_string(),
                value: "[]".to_string(),
                rust_expr: "vec![]".to_string(),
            });
            return Some(vec![]);
        }
        let bytes = self.consume_bytes(len)?;
        self.log.push(ParamEntry {
            name: name.to_string(),
            value: format!("[{} bytes]", bytes.len()),
            rust_expr: format!("vec!{:?}", bytes),
        });
        Some(bytes)
    }

    /// Consume up to `max_len` bytes as a UTF-8 string (lossy).
    pub fn string(&mut self, name: &str, max_len: usize) -> Option<String> {
        let raw = self.bytes(&format!("{name}_raw"), max_len)?;
        let val = String::from_utf8_lossy(&raw).to_string();
        // Overwrite the last log entry with string representation
        if let Some(last) = self.log.last_mut() {
            last.name = name.to_string();
            last.value = format!("{val:?}");
            last.rust_expr = format!("{val:?}.to_string()");
        }
        Some(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_consumption() {
        let mut input = ByteInput::new(vec![42, 0, 1, 10, 20, 30, 40]);
        assert_eq!(input.u8("a"), Some(42));
        assert_eq!(input.bool("b"), Some(false)); // 0 % 2 == 0
        assert_eq!(input.u8_range("c", 5, 15), Some(6)); // 1 % 11 + 5 = 6
        assert!(!input.is_empty());
        assert_eq!(input.params().len(), 3);
        assert_eq!(input.params()[0].rust_expr, "42u8");
    }

    #[test]
    fn test_exhaustion() {
        let mut input = ByteInput::new(vec![1, 2]);
        assert_eq!(input.u8("a"), Some(1));
        assert_eq!(input.u8("b"), Some(2));
        assert_eq!(input.u8("c"), None);
        assert!(input.is_empty());
    }

    #[test]
    fn test_reproducer() {
        let mut input = ByteInput::new(vec![3, 128, 0, 0, 0]);
        let _ = input.u8_range("count", 1, 5);
        let _ = input.f32_range("width", 0.0, 100.0);

        let template = "let count = $count;\nlet width = $width;\n";
        let code = input.generate_reproducer(template);
        assert!(code.contains("let count = "));
        assert!(code.contains("let width = "));
    }

    #[test]
    fn test_save_params() {
        let mut input = ByteInput::new(vec![10, 1]);
        input.u8("x");
        input.bool("flag");
        let params_str: String = input
            .params()
            .iter()
            .map(|e| format!("{}={}\n", e.name, e.value))
            .collect();
        assert!(params_str.contains("x=10"));
        assert!(params_str.contains("flag=true"));
    }
}
