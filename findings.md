# Code Audit Findings - auto_fuzzer v0.2.0

Date: 2026-05-15

---

## HIGH Severity

### 1. `load_settings_from()` silently overwrites `fuzz_settings.toml` in the working directory

When the user passes `--config some_other.toml`, the function copies its content over any existing `fuzz_settings.toml` in the current directory. This is destructive and surprising — the user's original `fuzz_settings.toml` is lost without warning.

```rust
// src/main.rs:428-437
fn load_settings_from(config_path: &str) -> settings::Setting {
    if config_path != "fuzz_settings.toml" && Path::new(config_path).exists() {
        let content = fs::read_to_string(config_path).expect("Failed to read config file");
        fs::write("fuzz_settings.toml", &content).expect("Failed to write temp config"); // BUG: overwrites existing file
    }
    load_settings()
}
```

**Fix:** Modify `load_settings()` to accept a path parameter, or use `config::File::from` with the user-supplied path instead of copying files around.

---

### 2. `TIMEOUT_SECS` double-set panics in CI mode

`TIMEOUT_SECS` is a `OnceCell` that can only be set once. In `ci::run_ci()`, it calls `TIMEOUT_SECS.set(timeout).unwrap()`. But if the main function or any other code path already set it (e.g., `Commands::Minimize` sets it to `999_999_999_999`), the `.unwrap()` will panic.

Similarly, `verify_regressions` sets it to `999_999_999_999` — if called after `run_ci` in the same process (unlikely but possible in tests or library use), this panics.

```rust
// src/ci.rs:13
TIMEOUT_SECS.set(timeout).unwrap(); // panics if already set

// src/ci.rs:84
TIMEOUT_SECS.set(999_999_999_999).unwrap(); // panics if already set
```

**Fix:** Use `TIMEOUT_SECS.get_or_init(|| timeout)` or switch to `AtomicU64` which can be updated multiple times.

---

### 3. `truncate_str` panics on multi-byte UTF-8 characters

If the string contains multi-byte UTF-8 (common in crash outputs, non-ASCII paths), slicing at byte position `max_len - 3` may land in the middle of a character, causing a panic.

```rust
// src/error_signature.rs:521-527
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3]) // panics on multi-byte char boundary
    }
}
```

**Fix:** Use `s.char_indices()` to find the last valid char boundary before `max_len - 3`, or use a crate-level helper that respects char boundaries.

---

### 4. TOML injection in metadata file via unsanitized crash output

The `save_results_to_file` function writes error signature fields directly into a TOML file using `format!`. If any field (especially `short_description` or `error_signature`) contains a double-quote `"` or newline, the resulting TOML is malformed and may silently corrupt data for downstream report tools.

```rust
// src/remove_non_crashing_files.rs:153-171
let metadata = format!(
    r#"error_type = "{}"
error_signature = "{}"
short_description = "{}"
..."#,
    signature.error_type,       // may contain quotes
    signature.signature(),      // may contain quotes
    signature.short_description, // very likely to contain quotes from assertion messages
    ...
);
```

**Fix:** Use `toml::to_string()` with a proper struct, or at minimum escape double-quotes in the values. Example: `signature.short_description.replace('"', "\\\"")`.

---

### 5. `zip_file` silently discards write errors — can produce corrupt zip files

Both `start_file` and `write_all` return `Result`, but errors are silently ignored with `let _ =`. If the disk is full or the zip entry fails, a corrupt empty zip is produced with no indication.

```rust
// src/remove_non_crashing_files.rs:209-218
pub fn zip_file(zip_filename: &str, file_name: &str, file_code: &[u8]) {
    let zip_file = File::create(zip_filename).unwrap();
    let mut zip_writer = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()...;
    let _ = zip_writer.start_file(file_name, options); // error silently discarded
    let _ = zip_writer.write_all(file_code);           // error silently discarded
    // ZipWriter::finish() is never called — relies on Drop
}
```

**Fix:** Propagate or at least log errors. Call `zip_writer.finish().unwrap()` to ensure the zip central directory is written.

---

### 6. `set_mode(0o777)` on collected files is a no-op on read-only filesystems and a security concern

`collect_files` sets all files to world-readable/writable/executable (0o777). This is overly permissive and won't work on read-only or special filesystems (CI tmpfs, etc.). The return value of `set_mode` is not checked.

```rust
// src/common.rs:322
metadata.permissions().set_mode(0o777);
```

Note: `Permissions::set_mode()` only modifies the in-memory `Permissions` struct — it does NOT apply it to the filesystem. This call is completely a no-op. You would need `fs::set_permissions(path, permissions)` to actually change anything.

**Fix:** Use `fs::set_permissions(&path, metadata.permissions())` after the set_mode, or remove the line entirely since it does nothing.

---

## MEDIUM Severity

### 7. Regex compiled on every call in `error_signature.rs` — performance issue

Every call to `parse_error_signature`, `extract_source_file`, etc. recompiles multiple regular expressions from scratch. In the `remove_non_crashing` hot path (called per-file in parallel), this adds significant overhead.

```rust
// src/error_signature.rs:440-441 (and 10+ other locations)
fn extract_source_file(output: &str) -> Option<String> {
    let re1 = Regex::new(r"panicked at '.*?',\s*(\S+?\.rs):\d+").ok()?;
    // ... 4 more Regex::new calls
}
```

**Fix:** Use `lazy_static!` or `std::sync::LazyLock` (Rust 1.80+) to compile each regex once:
```rust
static RE_PANIC_OLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"...").unwrap());
```

---

### 8. `close_app_if_timeouts()` calls `process::exit(0)` — skips cleanup

This is called inside `test_files_in_group` after `while_some()` completes. `process::exit(0)` terminates immediately without running destructors, potentially leaving temp files, partial writes, or locked resources.

```rust
// src/common.rs:38-43
pub(crate) fn close_app_if_timeouts() {
    if check_if_app_ends() {
        info!("Timeout reached, closing app");
        std::process::exit(0); // no cleanup, no destructors
    }
}
```

**Fix:** Return a `bool` or `Result` instead and let the caller handle graceful shutdown via the existing `SHOULD_STOP` mechanism.

---

### 9. `report::extract_toml_value` is a fragile ad-hoc TOML parser

It does line-by-line key matching without understanding TOML structure. A key like `error_signature` matches a line starting with `error_signature_v2` too. Values containing `=` are truncated.

```rust
// src/report.rs:223-231
fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with(key) {       // "error_signature" matches "error_signature_v2 = ..."
            if let Some(val) = line.split('=').nth(1) {  // "a = b = c" yields "b " not "b = c"
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}
```

**Fix:** Use `toml::from_str::<toml::Table>()` and index by key, since `toml` is already a dependency.

---

### 10. `fuzz_cargo.rs` — double `unsafe` block nesting

The `libc_signal_child` function is marked `unsafe fn` and also contains an inner `unsafe` block, which is redundant and triggers a future-incompatible warning.

```rust
// src/fuzz_cargo.rs:112-117
unsafe fn libc_signal_child(child: &std::process::Child) {
    let pid = child.id() as i32;
    unsafe {          // redundant inner unsafe
        libc::kill(pid, libc::SIGTERM);
    }
}
```

**Fix:** Remove either the `unsafe fn` annotation (make the function safe with an internal `unsafe` block) or the inner `unsafe` block.

---

### 11. `cargo fuzz run` signal handling sends SIGTERM to wrong process group

`libc::kill(pid, SIGTERM)` only sends the signal to the direct `cargo` process, not to the libfuzzer child process tree. The actual fuzzer process may continue running orphaned.

```rust
// src/fuzz_cargo.rs:53-55
unsafe {
    libc_signal_child(&child); // kills cargo, not the fuzzer subprocess
}
```

**Fix:** Use `libc::kill(-pid, libc::SIGTERM)` (negative PID) to signal the entire process group, or set up a new process group with `pre_exec` + `setsid`.

---

### 12. Ignore list is not consulted during fuzzing or report generation

The `IgnoreList` struct and its `is_ignored`/`find_matching` methods exist but are never called anywhere during the actual fuzzing pipeline or in `remove_non_crashing_files`. The fuzzer uses `ignored_item_N` from `fuzz_settings.toml` exclusively. The ignore list is purely a standalone CLI data store.

```rust
// src/ignore_list.rs:86-91 — never called
pub fn is_ignored(&self, project: &str, output: &str) -> bool { ... }
pub fn find_matching(&self, project: &str, output: &str) -> Vec<&IgnoreEntry> { ... }
```

**Fix (needs verification):** Decide on the integration point. Either hook `is_ignored()` into `save_results_to_file` to skip writing reports for ignored patterns, or into `is_broken()` in the custom struct. Currently the two ignore systems (settings-based `ignored_item_N` and `ignore_list.toml`) are completely disconnected.

---

### 13. `ci::run_ci()` hardcodes "default" target for cargo-fuzz

When CI mode runs with `--mode cargo-fuzz`, it always passes `"default"` as the target name instead of reading it from the config or a CLI parameter.

```rust
// src/ci.rs:33
run_cargo_fuzz("default", &corpus_dir, timeout, None, 4);
```

**Fix:** Accept `--target` in `CiAction::Run` and pass it through, or read it from the config file.

---

## LOW Severity

### 14. `fuzz_cargo.rs` uses `format!` where a string literal suffices

```rust
// src/fuzz_cargo.rs:28-29
cmd.arg(format!("-max_len=99999"))    // should be just "-max_len=99999"
    .arg(format!("-max_total_time={timeout}"))  // this one is fine, uses interpolation
```

**Fix:** `cmd.arg("-max_len=99999")`.

---

### 15. `collect_command_to_string` quotes all paths containing `/`

Any argument with a `/` (i.e., virtually every file path on Unix) gets double-quoted. This may produce confusing output in reports where simple paths like `/opt/BROKEN_FILES_DIR/file.bin` appear quoted.

```rust
// src/common.rs:349
if [" ", "\"", "\\", "/"].iter().any(|e| tmp_string.contains(e)) {
```

**Fix (potential, needs verification):** Consider removing `"/"` from the quoting triggers, or only quoting when shell-special characters are present.

---

### 16. `USE_ASAN_ENVS` uses `state::InitCell<RwLock<bool>>` — overcomplicated

A simple `AtomicBool` would suffice for a thread-safe boolean flag. The current approach requires a `RwLock` acquisition (with potential poisoning on panic) for each read.

```rust
// src/obj.rs:12
pub static USE_ASAN_ENVS: state::InitCell<RwLock<bool>> = state::InitCell::new();
```

**Fix:** Replace with `static USE_ASAN_ENVS: AtomicBool = AtomicBool::new(false);`.

---

### 17. `common.rs::create_new_file_name` panics on files without extension

If a file has no extension (e.g., `Makefile`, `LICENSE`), `.extension().unwrap()` panics.

```rust
// src/common.rs:61
let extension = pat.extension().unwrap().to_str().unwrap().to_string();
```

Same issue in `create_new_file_name_for_minimization` at line 75.

**Fix:** Use `.extension().and_then(|e| e.to_str()).unwrap_or("")`.

---

### 18. `finding_text_status.rs` calls `close_app_if_timeouts()` which exits the whole process

After the group test phase, `close_app_if_timeouts()` is called which does `process::exit(0)`. This exits without saving any results found during the grouping phase. Since `SHOULD_STOP` is already checked in the loop, this call is both redundant and harmful.

```rust
// src/finding_text_status.rs:177
close_app_if_timeouts();
```

**Fix:** Remove the call, or replace with a `check_if_app_ends()` that returns to the caller.

---

### 19. `report::create_report` generates a shell script vulnerable to argument injection

If the issue title contains shell metacharacters (backticks, `$()`, etc.), the `$(cat ...)` expansion in the generated script is safe, but the comment line `# Title: {title}` is not executed. However, the repo argument `--repo "{r}"` could be exploited if the repo string comes from untrusted input (unlikely in practice).

```rust
// src/report.rs:139-140
let repo_arg = if let Some(r) = repo {
    format!(" \\\n     --repo \"{r}\"")  // no escaping of r
};
```

**Fix (potential issue):** Escape double-quotes in `r` if needed, though in practice repo names are user-supplied CLI args.

---

### 20. `collect_files` does `dbg!` in production code then asserts

The `dbg!` macro is meant for debugging and prints to stderr. Having it in production code before an assert is messy.

```rust
// src/common.rs:336-338
if files.is_empty() {
    dbg!(&settings);
    assert!(!files.is_empty());
}
```

**Fix:** Replace with `panic!("No valid files found in {}", settings.temp_possible_broken_files_dir)` or similar.

---

### 21. `save_results_to_file` template uses `$` placeholders with `.replace()` — fragile

The template string uses `$CNT_TEXT`, `$COMMAND`, `$ERROR` as placeholders. If the file content or error output contains these exact strings, they get replaced too, corrupting the report.

```rust
// src/remove_non_crashing_files.rs:145-148
.replace("$CNT_TEXT", &cnt_text)
.replace("$COMMAND", &command_str_with_extension)
.replace("$ERROR", &result)
```

**Fix (potential issue, low probability):** Use a more unique placeholder pattern like `{{CNT_TEXT}}` or use a proper template engine. In practice, crash outputs are very unlikely to contain `$CNT_TEXT`, but `$ERROR` is plausible in shell-like outputs.

---

### 22. `ci::verify_regressions` may overwrite files during archive with name collisions

When archiving fixed files, it uses just the filename (not the full path). If two files from different subdirectories have the same name, `fs::rename` to the same destination silently overwrites the first.

```rust
// src/ci.rs:133-134
let file_name = Path::new(file).file_name().unwrap().to_string_lossy().to_string();
let _ = fs::rename(file, format!("{archive_dir}/{file_name}"));
```

**Fix:** Include a hash or counter in the archived filename to prevent collisions.

---

### 23. No integration between `Fuzz` subcommand and `report list/create` results directory

The `Fuzz` command writes results to `settings.temp_folder` and `settings.broken_files_dir`, but the `Report` command looks in a `results` directory by default. There's no obvious path connecting the two workflows — a user must manually know to point `--dir` at the right place.

**Fix (needs verification):** Document the workflow or add a `--results-dir` option to `Fuzz` that defaults to the same directory `Report` uses.

---

### 24. `normalize_assertion_condition` only replaces numbers with 2+ digits

Single-digit numbers like `0`, `1`, `5` are preserved. This means `assertion failed: x > 0` and `assertion failed: x > 5` would be grouped differently, even though they're likely the same assertion with different runtime values.

```rust
// src/error_signature.rs:491
let re = Regex::new(r"\b\d{2,}\b").unwrap();
```

**Fix (needs verification):** This may be intentional — single-digit numbers are more likely to be actual constants in the code rather than runtime values. But it's worth reconsidering, especially for bounds-checking assertions.
