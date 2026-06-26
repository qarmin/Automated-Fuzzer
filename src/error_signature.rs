use std::sync::LazyLock;

use regex::Regex;

// Pre-compiled regexes for hot-path performance
static RE_PANIC_OLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"panicked at '.*?',\s*(\S+?\.rs):(\d+)").unwrap());
static RE_PANIC_NEW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"panicked at (\S+?\.rs):(\d+)").unwrap());
static RE_SOURCE_LOC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Source Location:\s*(\S+?\.rs):(\d+)").unwrap());
static RE_BT_SRC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bat ((?:\S*?/)?src/\S+?\.rs):(\d+)").unwrap());
static RE_BT_ANY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bat (\S+?\.rs):(\d+)").unwrap());
static RE_ASSERT_FAILED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"assertion failed:\s*(.+)").unwrap());
static RE_ASSERT_BACKTICK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"assertion `([^`]+)` failed").unwrap());
static RE_PANIC_MSG_OLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"panicked at '([^']+)'").unwrap());
static RE_PANIC_MSG_NEW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"panicked at \S+\.rs:\d+:\d+:\s*\n\s*(.+)").unwrap());
static RE_ASAN_TYPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"AddressSanitizer:\s*(\S+)").unwrap());
static RE_NORMALIZE_NUMS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{2,}\b").unwrap());

/// Represents a parsed error signature for grouping crashes
#[derive(Debug, Clone)]
pub struct ErrorSignature {
    /// High-level error type (e.g., "overflow_subtract", "assertion_eq", "index_out_of_bounds")
    pub error_type: String,
    /// Source file where the error occurred (without line number)
    pub source_file: Option<String>,
    /// Line number in source file (for folder naming, not for issue titles)
    pub source_line: Option<u32>,
    /// Detailed condition/message (e.g., "a > 25" for assertion)
    pub condition: Option<String>,
    /// Short description for issue title
    pub short_description: String,
}

impl ErrorSignature {
    /// Returns the full signature string used for grouping/deduplication
    pub fn signature(&self) -> String {
        let mut sig = self.error_type.clone();
        if let Some(ref src) = self.source_file {
            sig.push_str("::");
            sig.push_str(src);
        }
        if let Some(ref cond) = self.condition {
            sig.push_str("::");
            sig.push_str(cond);
        }
        sig
    }

    /// Returns issue title appropriate for the error type.
    ///
    /// Examples:
    ///   Panic `attempt to subtract with overflow` in `src/foo.rs`
    ///   Timeout when processing file
    ///   AddressSanitizer `heap-buffer-overflow` in `src/bar.rs`
    pub fn issue_title(&self) -> String {
        let desc = &self.short_description;

        // Types where the prefix alone is the full description (no backtick-quoting needed)
        let standalone = matches!(
            self.error_type.as_str(),
            "timeout" | "memory_failure" | "out_of_memory" | "segmentation_fault" | "aborted"
        );

        let prefix = match self.error_type.as_str() {
            "timeout" => "Timeout",
            "memory_failure" => "Memory allocation failure",
            "out_of_memory" => "Out of memory",
            "stack_overflow" => "Stack overflow",
            "segmentation_fault" => "Segmentation fault",
            "heap_use_after_free" => "Heap use after free",
            "address_sanitizer" => "AddressSanitizer",
            "thread_sanitizer" => "ThreadSanitizer",
            "leak_sanitizer" => "LeakSanitizer",
            "aborted" => "Aborted",
            "syntax_error" => "Syntax error",
            _ => "Panic",
        };

        // Backticks inside the message/source would close our own backtick-quoting early and
        // garble the title (e.g. "Panic `assertion `left == right` failed` in ...").
        let desc = strip_backticks(desc);

        match (standalone, &self.source_file) {
            // "Timeout when processing file", "Segmentation fault when processing file"
            (true, None) => format!("{prefix} when processing file"),
            // "Timeout when processing file in `src/foo.rs`"  (rare but possible with ASAN)
            (true, Some(src)) => format!("{prefix} in `{}`", strip_backticks(src)),
            // "Panic `msg` in `src/foo.rs`"
            (false, Some(src)) => format!("{prefix} `{desc}` in `{}`", strip_backticks(src)),
            // "Panic `msg`"
            (false, None) => format!("{prefix} `{desc}`"),
        }
    }
}

/// Remove backtick characters so an interpolated value cannot break the title's own
/// backtick-quoting. Surrounding whitespace left by the removal is collapsed and trimmed.
fn strip_backticks(s: &str) -> String {
    s.replace('`', "").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse crash output into a structured ErrorSignature
pub fn parse_error_signature(output: &str) -> ErrorSignature {
    // Try each pattern in order of specificity

    // Assertion `left == right` failed
    if let Some(sig) = try_parse_assertion_eq(output) {
        return sig;
    }

    // Assertion failed: CONDITION
    if let Some(sig) = try_parse_assertion_custom(output) {
        return sig;
    }

    // Specific overflow types
    if let Some(sig) = try_parse_overflow(output) {
        return sig;
    }

    // Index out of bounds
    if let Some(sig) = try_parse_index_out_of_bounds(output) {
        return sig;
    }

    // Unreachable code with message
    if let Some(sig) = try_parse_unreachable(output) {
        return sig;
    }

    // Not implemented
    if let Some(sig) = try_parse_not_implemented(output) {
        return sig;
    }

    // Char boundary
    if output.contains("is not a char boundary") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "char_boundary".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "is not a char boundary".to_string(),
        };
    }

    // Divide by zero
    if output.contains("divide by zero") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "divide_by_zero".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "attempt to divide by zero".to_string(),
        };
    }

    // Option::unwrap()
    if output.contains("Option::unwrap()") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "option_unwrap".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "called `Option::unwrap()` on a `None` value".to_string(),
        };
    }

    // Result::unwrap()
    if output.contains("Result::unwrap()") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "result_unwrap".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "called `Result::unwrap()` on an `Err` value".to_string(),
        };
    }

    // Slicing error
    if output.contains("when slicing `") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "slicing".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "range error when slicing".to_string(),
        };
    }

    // Out of range
    if output.contains("out of range for") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "out_of_range".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "out of range".to_string(),
        };
    }

    // Memory allocation failure
    if output.contains("memory allocation of") {
        return ErrorSignature {
            error_type: "memory_failure".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "memory allocation failure".to_string(),
        };
    }

    // Stack overflow
    if output.contains("stack overflow") || output.contains("stack-overflow") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "stack_overflow".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "stack overflow".to_string(),
        };
    }

    // Heap use after free (ASAN)
    if output.contains("heap-use-after-free") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "heap_use_after_free".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "heap-use-after-free".to_string(),
        };
    }

    // AddressSanitizer (generic)
    if output.contains("AddressSanitizer") {
        let (src, src_line) = extract_source_file_and_line(output);
        let detail = extract_asan_type(output);
        return ErrorSignature {
            error_type: "address_sanitizer".to_string(),
            source_file: src,
            source_line: src_line,
            condition: detail.clone(),
            short_description: detail.unwrap_or_else(|| "AddressSanitizer error".to_string()),
        };
    }

    // ThreadSanitizer
    if output.contains("ThreadSanitizer") {
        return ErrorSignature {
            error_type: "thread_sanitizer".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "ThreadSanitizer error".to_string(),
        };
    }

    // LeakSanitizer
    if output.contains("LeakSanitizer") {
        return ErrorSignature {
            error_type: "leak_sanitizer".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "LeakSanitizer: memory leak detected".to_string(),
        };
    }

    // Segmentation fault
    if output.contains("segmentation fault") || output.contains("output signal \"Some(11)\"") {
        return ErrorSignature {
            error_type: "segmentation_fault".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "segmentation fault".to_string(),
        };
    }

    // Killed / OOM
    if output.contains("Killed") || output.contains("output signal \"Some(15)\"") {
        return ErrorSignature {
            error_type: "out_of_memory".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "killed (likely out of memory)".to_string(),
        };
    }

    // Timeout
    if output.contains("output status \"Some(124)\"") || output.contains("timeout: sending signal") {
        return ErrorSignature {
            error_type: "timeout".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "timeout when processing file".to_string(),
        };
    }

    // Aborted
    if output.contains("Aborted") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "aborted".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "aborted".to_string(),
        };
    }

    // Generic panic with panicked at message
    if output.contains("panicked at") {
        let (src, src_line) = extract_source_file_and_line(output);
        let msg = extract_panic_message(output);
        let short = msg.clone().unwrap_or_else(|| "panicked".to_string());
        return ErrorSignature {
            error_type: "panic".to_string(),
            source_file: src,
            source_line: src_line,
            condition: msg,
            short_description: truncate_str(&short, 80),
        };
    }

    // Generic RUST_BACKTRACE
    if output.contains("RUST_BACKTRACE") {
        let (src, src_line) = extract_source_file_and_line(output);
        return ErrorSignature {
            error_type: "panic".to_string(),
            source_file: src,
            source_line: src_line,
            condition: None,
            short_description: "panic".to_string(),
        };
    }

    // Syntax error (from formatters/linters)
    if output.contains("Fix introduced a syntax error") {
        return ErrorSignature {
            error_type: "syntax_error".to_string(),
            source_file: None,
            source_line: None,
            condition: None,
            short_description: "fix introduced a syntax error".to_string(),
        };
    }

    // Fallback
    ErrorSignature {
        error_type: "unknown".to_string(),
        source_file: None,
        source_line: None,
        condition: None,
        short_description: "unknown error".to_string(),
    }
}

fn try_parse_assertion_eq(output: &str) -> Option<ErrorSignature> {
    // Pattern: "assertion `left == right` failed"
    if !output.contains("assertion `left == right` failed") && !output.contains("assertion `left != right` failed") {
        return None;
    }
    let op = if output.contains("assertion `left == right` failed") {
        "eq"
    } else {
        "ne"
    };
    let (src, src_line) = extract_source_file_and_line(output);
    Some(ErrorSignature {
        error_type: format!("assertion_{op}"),
        source_file: src,
        source_line: src_line,
        condition: None,
        short_description: format!("assertion `left {op} right` failed"),
    })
}

fn try_parse_assertion_custom(output: &str) -> Option<ErrorSignature> {
    // Pattern: "assertion failed: CONDITION" or "assertion `CONDITION` failed"
    let condition = if let Some(cap) = RE_ASSERT_FAILED.captures(output) {
        Some(normalize_assertion_condition(cap.get(1)?.as_str()))
    } else {
        RE_ASSERT_BACKTICK
            .captures(output)
            .and_then(|cap| cap.get(1))
            .map(|m| normalize_assertion_condition(m.as_str()))
    };

    let condition = condition?;

    // Skip the left == right case (handled above)
    if condition.contains("left == right") || condition.contains("left != right") {
        return None;
    }

    let (src, src_line) = extract_source_file_and_line(output);
    let short = format!("assertion failed: {condition}");
    Some(ErrorSignature {
        error_type: "assertion".to_string(),
        source_file: src,
        source_line: src_line,
        condition: Some(condition),
        short_description: truncate_str(&short, 80),
    })
}

fn try_parse_overflow(output: &str) -> Option<ErrorSignature> {
    let patterns = [
        ("attempt to subtract with overflow", "overflow_subtract"),
        ("attempt to multiply with overflow", "overflow_multiply"),
        ("attempt to add with overflow", "overflow_add"),
        ("attempt to shift right with overflow", "overflow_shift_right"),
        ("attempt to shift left with overflow", "overflow_shift_left"),
        ("attempt to negate with overflow", "overflow_negate"),
        ("attempt to divide with overflow", "overflow_divide"),
    ];

    for (pattern, error_type) in patterns {
        if output.contains(pattern) {
            let (src, src_line) = extract_source_file_and_line(output);
            return Some(ErrorSignature {
                error_type: error_type.to_string(),
                source_file: src,
                source_line: src_line,
                condition: None,
                short_description: pattern.to_string(),
            });
        }
    }
    None
}

fn try_parse_index_out_of_bounds(output: &str) -> Option<ErrorSignature> {
    if !output.contains("index out of bounds") && !output.contains("is out of bounds") {
        return None;
    }
    let (src, src_line) = extract_source_file_and_line(output);
    Some(ErrorSignature {
        error_type: "index_out_of_bounds".to_string(),
        source_file: src,
        source_line: src_line,
        condition: None,
        short_description: "index out of bounds".to_string(),
    })
}

fn try_parse_unreachable(output: &str) -> Option<ErrorSignature> {
    if !output.contains("entered unreachable code") {
        return None;
    }
    let (src, src_line) = extract_source_file_and_line(output);

    // Extract the message after "entered unreachable code: "
    let condition = if let Some(idx) = output.find("entered unreachable code: ") {
        let msg_start = idx + "entered unreachable code: ".len();
        let msg = &output[msg_start..];
        let end = msg.find('\n').unwrap_or(msg.len());
        let trimmed = msg[..end].trim().trim_end_matches('\'').trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    } else {
        None
    };

    let short = if let Some(ref c) = condition {
        format!("internal error: entered unreachable code: {c}")
    } else {
        "internal error: entered unreachable code".to_string()
    };

    Some(ErrorSignature {
        error_type: "unreachable_code".to_string(),
        source_file: src,
        source_line: src_line,
        condition,
        short_description: truncate_str(&short, 80),
    })
}

fn try_parse_not_implemented(output: &str) -> Option<ErrorSignature> {
    if !output.contains("not implemented:") && !output.contains("not yet implemented") {
        return None;
    }
    let (src, src_line) = extract_source_file_and_line(output);
    let condition = if let Some(idx) = output.find("not implemented: ") {
        let msg_start = idx + "not implemented: ".len();
        let msg = &output[msg_start..];
        let end = msg.find('\n').unwrap_or(msg.len());
        Some(msg[..end].trim().to_string())
    } else {
        None
    };

    let short = if let Some(ref c) = condition {
        format!("not implemented: {c}")
    } else {
        "not yet implemented".to_string()
    };

    Some(ErrorSignature {
        error_type: "not_implemented".to_string(),
        source_file: src,
        source_line: src_line,
        condition,
        short_description: truncate_str(&short, 80),
    })
}

/// Returns true if the path points into rustc internals or a cargo registry checkout,
/// in which case it should be skipped when looking for the crash's true source location.
fn is_external_source(path: &str) -> bool {
    path.contains("rustc/") || path.contains(".cargo/registry") || path.contains("library/")
}

/// Extract source file path and line number from Rust panic/backtrace output.
/// Skips matches inside rustc/std/registry paths so that crashes are attributed
/// to the actual project source rather than to a transitive dependency.
fn extract_source_file_and_line(output: &str) -> (Option<String>, Option<u32>) {
    let regexes: &[&LazyLock<Regex>] = &[&RE_PANIC_OLD, &RE_PANIC_NEW, &RE_SOURCE_LOC, &RE_BT_SRC, &RE_BT_ANY];
    for re in regexes {
        for cap in re.captures_iter(output) {
            let raw = cap.get(1).unwrap().as_str();
            if is_external_source(raw) {
                continue;
            }
            let line = cap.get(2).and_then(|m| m.as_str().parse().ok());
            return (Some(normalize_source_path(raw)), line);
        }
    }

    (None, None)
}

/// Normalize source path - keep from "src/" onwards, strip absolute prefix
fn normalize_source_path(path: &str) -> String {
    if let Some(idx) = path.find("src/") {
        path[idx..].to_string()
    } else if let Some(idx) = path.find("crates/") {
        path[idx..].to_string()
    } else {
        path.to_string()
    }
}

/// Normalize assertion condition - remove specific numeric values
fn normalize_assertion_condition(condition: &str) -> String {
    let trimmed = condition.trim().to_string();
    // Replace concrete numbers with N for dedup (but keep variable names)
    RE_NORMALIZE_NUMS.replace_all(&trimmed, "N").to_string()
}

/// Extract panic message from output
fn extract_panic_message(output: &str) -> Option<String> {
    // "panicked at 'MESSAGE'"
    if let Some(cap) = RE_PANIC_MSG_OLD.captures(output) {
        return Some(cap.get(1)?.as_str().to_string());
    }

    // "panicked at src/file.rs:123:\nMESSAGE"
    if let Some(cap) = RE_PANIC_MSG_NEW.captures(output) {
        return Some(cap.get(1)?.as_str().trim().to_string());
    }

    None
}

/// Extract ASAN error type
fn extract_asan_type(output: &str) -> Option<String> {
    RE_ASAN_TYPE
        .captures(output)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    // Find the last valid char boundary at or before max_len - 3
    let target = max_len.saturating_sub(3);
    let boundary = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= target)
        .last()
        .unwrap_or(0);
    format!("{}...", &s[..boundary])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overflow_subtract() {
        let output = "thread 'main' panicked at src/wavpack/properties.rs:123:5:\nattempt to subtract with overflow\nnote: run with `RUST_BACKTRACE=1`";
        let sig = parse_error_signature(output);
        assert_eq!(sig.error_type, "overflow_subtract");
        assert_eq!(sig.source_file.as_deref(), Some("src/wavpack/properties.rs"));
        assert_eq!(
            sig.issue_title(),
            "Panic `attempt to subtract with overflow` in `src/wavpack/properties.rs`"
        );
    }

    #[test]
    fn test_issue_title_strips_backticks_from_message() {
        let sig = ErrorSignature {
            error_type: "assertion".to_string(),
            source_file: Some("src/foo.rs".to_string()),
            source_line: Some(1),
            condition: None,
            short_description: "assertion `left == right` failed".to_string(),
        };
        assert_eq!(
            sig.issue_title(),
            "Panic `assertion left == right failed` in `src/foo.rs`"
        );
    }

    #[test]
    fn test_assertion_eq() {
        let output =
            "thread 'main' panicked at src/state.rs:280:9:\nassertion `left == right` failed\n  left: 2\n right: 1";
        let sig = parse_error_signature(output);
        assert_eq!(sig.error_type, "assertion_eq");
        assert_eq!(sig.source_file.as_deref(), Some("src/state.rs"));
    }

    #[test]
    fn test_assertion_custom_different_conditions() {
        let output1 = "thread 'main' panicked at src/foo.rs:10:1:\nassertion failed: a > 25";
        let output2 = "thread 'main' panicked at src/foo.rs:20:1:\nassertion failed: b > 25";
        let sig1 = parse_error_signature(output1);
        let sig2 = parse_error_signature(output2);
        // Different conditions should produce different signatures
        assert_ne!(sig1.signature(), sig2.signature());
        assert_eq!(sig1.error_type, "assertion");
        assert_eq!(sig2.error_type, "assertion");
    }

    #[test]
    fn test_unreachable_with_message() {
        let output = "thread 'main' panicked at src/id3/v2/read.rs:50:5:\ninternal error: entered unreachable code: Bad BOM [0, 0]";
        let sig = parse_error_signature(output);
        assert_eq!(sig.error_type, "unreachable_code");
        assert!(sig.condition.as_ref().unwrap().contains("Bad BOM"));
    }

    #[test]
    fn test_index_out_of_bounds() {
        let output = "thread 'main' panicked at src/wavpack/properties.rs:99:5:\nindex out of bounds: the len is 488 but the index is 488";
        let sig = parse_error_signature(output);
        assert_eq!(sig.error_type, "index_out_of_bounds");
        assert_eq!(sig.source_file.as_deref(), Some("src/wavpack/properties.rs"));
    }

    #[test]
    fn test_timeout() {
        let output =
            "timeout: sending signal\n##### Automatic Fuzzer note, output status \"Some(124)\", output signal \"None\"";
        let sig = parse_error_signature(output);
        assert_eq!(sig.error_type, "timeout");
    }
}
