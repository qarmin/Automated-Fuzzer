//! Fuzzing utilities — common file-processing boilerplate + structured byte input.
//!
//! # File-based fuzzing (most crates):
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
//! # Structured fuzzing (API generators like pdf-writer):
//! ```ignore
//! fn main() {
//!     fuzz_utils::run(|path| {
//!         let mut input = fuzz_utils::ByteInput::from_file(path).unwrap();
//!         let count = input.u8_range("count", 1, 10);
//!         // ...
//!     });
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

/// Run `f` on every file from argv (file or directory, recursive).
///
/// Replaces the 15-line boilerplate in every crate's main.rs:
/// ```ignore
/// fn main() {
///     fuzz_utils::run(check_file);
/// }
///
/// fn check_file(path: &str) {
///     let data = std::fs::read(path).unwrap();
///     // ...
/// }
/// ```
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

    /// Generate a reproducer from the actual source file of a structured fuzzer.
    ///
    /// Reads `source_path`, finds the target function (e.g. `run_pdf_writer`),
    /// and replaces all `input.xxx("name", ...)` calls with hardcoded values.
    /// Also removes the `input: &mut ByteInput` parameter and `?` suffixes.
    ///
    /// Returns `None` if the source can't be read.
    pub fn generate_source_reproducer(&self, source_path: &str) -> Option<String> {
        let source = std::fs::read_to_string(source_path).ok()?;

        // Build a map: name -> rust_expr
        let mut replacements: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for entry in &self.log {
            replacements.insert(&entry.name, &entry.rust_expr);
        }

        let mut output = String::new();
        let mut in_target_fn = false;
        let mut brace_depth = 0u32;
        let mut skip_use_fuzz_utils = false;

        for line in source.lines() {
            let trimmed = line.trim();

            // Skip fuzz_utils and ByteInput imports
            if trimmed.starts_with("use fuzz_utils") || trimmed.contains("ByteInput") {
                skip_use_fuzz_utils = true;
                if trimmed.ends_with(';') {
                    skip_use_fuzz_utils = false;
                }
                continue;
            }
            if skip_use_fuzz_utils {
                if trimmed.ends_with(';') {
                    skip_use_fuzz_utils = false;
                }
                continue;
            }

            // Skip main(), check_file, and fuzz_utils::run lines
            if trimmed.starts_with("fn main()") || trimmed.contains("fuzz_utils::run")
                || trimmed.starts_with("fn check_file(") || trimmed.contains("save_params")
            {
                continue;
            }

            // Detect the target function (any fn with ByteInput param)
            if trimmed.starts_with("fn ") && trimmed.contains("ByteInput") {
                in_target_fn = true;
                // Rewrite signature: remove input param, change return to ()
                let fn_name = trimmed.split('(').next().unwrap_or("fn run");
                output.push_str(fn_name);
                output.push_str("() {\n");
                brace_depth = 1;
                continue;
            }

            if in_target_fn {
                // Track braces
                for ch in trimmed.chars() {
                    if ch == '{' { brace_depth += 1; }
                    if ch == '}' { brace_depth = brace_depth.saturating_sub(1); }
                }

                if brace_depth == 0 {
                    output.push_str("}\n");
                    in_target_fn = false;
                    continue;
                }

                let mut modified = line.to_string();

                // Replace input.xxx("name", ...) with the value
                // Pattern: input.u8_range("name", min, max)?
                // → value (e.g. 12u8)
                for (name, expr) in &replacements {
                    // Match: input.<method>("<name>"...)  optionally with ?
                    let patterns = [
                        format!("input.u8_range(\"{name}\""),
                        format!("input.u8(\"{name}\""),
                        format!("input.u16(\"{name}\""),
                        format!("input.u32(\"{name}\""),
                        format!("input.u64(\"{name}\""),
                        format!("input.i32(\"{name}\""),
                        format!("input.f32_range(\"{name}\""),
                        format!("input.f64_range(\"{name}\""),
                        format!("input.bool(\"{name}\""),
                        format!("input.index(\"{name}\""),
                        format!("input.bytes(\"{name}\""),
                        format!("input.string(\"{name}\""),
                        format!("input.pick(\"{name}\""),
                    ];

                    for pat in &patterns {
                        if let Some(start) = modified.find(pat.as_str()) {
                            // Find the closing )? or )
                            if let Some(close_paren) = modified[start..].find(')') {
                                let end = start + close_paren + 1;
                                // Check for trailing ?
                                let end_with_q = if modified.get(end..end+1) == Some("?") {
                                    end + 1
                                } else {
                                    end
                                };
                                modified = format!(
                                    "{}{}{}",
                                    &modified[..start],
                                    expr,
                                    &modified[end_with_q..]
                                );
                            }
                            break;
                        }
                    }
                }

                // Replace `input.is_empty()` with `false`
                modified = modified.replace("input.is_empty()", "false");

                // Remove lines that only reference `input` (like let mut input = ...)
                let mod_trimmed = modified.trim();
                if mod_trimmed.starts_with("let mut input") || mod_trimmed.starts_with("let input") {
                    continue;
                }
                // Remove `let _ = input.save_params(...)` lines
                if mod_trimmed.contains("input.save_params") {
                    continue;
                }

                // Replace `Some(())` at end with nothing (or keep as is)
                output.push_str(&modified);
                output.push('\n');
            } else {
                // Outside target fn — keep use statements etc.
                output.push_str(line);
                output.push('\n');
            }
        }

        // Add a simple main
        output.push_str("\nfn main() {\n");
        // Find the target function name
        for line in source.lines() {
            if line.trim().starts_with("fn ") && line.contains("ByteInput") {
                let fn_name = line.trim().split('(').next().unwrap_or("fn run").trim_start_matches("fn ").trim();
                output.push_str(&format!("    {fn_name}();\n"));
                break;
            }
        }
        output.push_str("}\n");

        Some(output)
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
