# Code Audit Findings — `automated_fuzzer`

**Date:** 2026-05-14
**Scope:** Full project review (`src/` — ~1800 lines of Rust)

---

## Confirmed Bugs

### 1. [HIGH] `zip_file` silently ignores write errors

`remove_non_crashing_files.rs:296-297` — Both `start_file` and `write_all` return `Result`, but errors are silently discarded with `let _ =`. If the zip write fails (disk full, permissions), the output zip will be corrupt with no indication of failure.

```rust
let _ = zip_writer.start_file(file_name, options);
let _ = zip_writer.write_all(file_code);
```

**Fix:** Propagate or at least log errors. At minimum use `.unwrap()` to fail loudly, or return `Result` from the function.

---

### 2. [HIGH] `set_mode(0o777)` on collected files — overly permissive and potentially ineffective

`common.rs:329` — `metadata.permissions().set_mode(0o777)` creates a *new* `Permissions` value but never calls `fs::set_permissions()` to apply it. This is a no-op — the permissions are never actually changed on the filesystem.

```rust
let Ok(metadata) = i.metadata() else { continue; };
metadata.permissions().set_mode(0o777);
```

**Fix:** If the intent is to actually change permissions, use `fs::set_permissions(path, metadata.permissions())`. If it's intentionally a no-op, remove the dead code. Also consider if `0o777` (world-writable) is actually desired — `0o644` or `0o755` would be safer.

---

### 3. [HIGH] `create_new_file_name` panics on files without extensions

`common.rs:61` — `.extension().unwrap()` will panic if the file has no extension (e.g., `Makefile`, `LICENSE`).

```rust
let extension = pat.extension().unwrap().to_str().unwrap().to_string();
let file_name = pat.file_stem().unwrap().to_str().unwrap().to_string();
```

Same issue exists in `create_new_file_name_for_minimization` at line 74 and in multiple other places (`remove_non_crashing_files.rs:163`, `finding_text_status.rs:129`).

**Fix:** Handle `None` from `.extension()` with a default like `"bin"` or `"dat"`.

---

### 4. [MEDIUM] `remove_non_crashing_in_group` is dead code that immediately returns

`remove_non_crashing_files.rs:56` — The function has an unconditional `return broken_files;` as its first statement. All code below is unreachable. The `#[allow(dead_code)]` and `#[allow(unused)]` annotations mask this.

```rust
fn remove_non_crashing_in_group(...) -> Vec<String> {
    // TODO this check may be broken - test it
    return broken_files;
    // ... ~50 lines of unreachable code
```

**Fix:** Either remove the function entirely or fix/re-enable it. Keeping dead code with suppressions adds confusion.

---

### 5. [MEDIUM] `get_group_command` joins filenames with spaces — breaks paths with spaces

`apps/custom.rs:70-71` — File paths are joined with a literal space and substituted into a single command argument. Paths containing spaces will be incorrectly split.

```rust
.map(|e| e.replace("FILE_PATHS_TO_PROVIDE", &full_name.join(" ")));
```

**Fix:** This is a fundamental design issue with the placeholder approach. For group commands, each file should be a separate argument rather than concatenated into one string.

---

### 6. [MEDIUM] `content_before` read then unconditionally written back — race condition with parallel execution

`common.rs:225,250` — The file is read, the command is executed, then the original content is written back. In parallel execution (rayon), if two threads process the same file, they'll corrupt each other.

```rust
let content_before = fs::read(full_name).unwrap();
// ... execute command ...
let res = fs::write(full_name, &content_before);
```

The TODO comment on line 250 acknowledges this is only needed for "unsafe mode" tools that modify input files. In practice, most tools are read-only, making this a redundant I/O penalty and a potential source of race conditions.

**Fix:** Make this read-back-write conditional on a setting flag (e.g., `unsafe_mode`).

---

## Potential Bugs / Needs Verification

### 7. [MEDIUM] `Ordering::Release` used without a corresponding `Acquire` — potential issue

`finding_text_status.rs:196`, `finding_different_output.rs:95` — `fetch_add` uses `Ordering::Release`, but the values are read with `Ordering::Relaxed`. For a simple progress counter this is harmless, but it's semantically wrong — `Relaxed` is sufficient for both.

```rust
let number = atomic.fetch_add(1, Ordering::Release);
```

**Fix:** Use `Ordering::Relaxed` consistently for progress counters.

---

### 8. [MEDIUM] `collect_files` asserts after `dbg!` — exposes settings in production

`common.rs:344-346` — Uses `dbg!` (which prints to stderr) before panicking. In a production/CI context this leaks internal configuration.

```rust
if files.is_empty() {
    dbg!(&settings);
    assert!(!files.is_empty());
}
```

**Fix:** Use `panic!("No files found in {}", settings.temp_possible_broken_files_dir)` with a targeted error message.

---

### 9. [LOW] `allowed_error_statuses` parsing panics on empty string

`settings.rs:212-214` — If `allowed_error_statuses` is an empty string `""`, `.split(',')` produces `[""]`, and `"".parse::<i32>().unwrap()` will panic.

```rust
allowed_error_statuses: general["allowed_error_statuses"]
    .split(',')
    .map(|e| e.parse().unwrap())
    .collect(),
```

Compare with `allowed_signal_statuses` (lines 227-231) which correctly filters empty strings. The inconsistency suggests the error statuses parsing is buggy.

**Fix:** Add `.filter(|e| !e.trim().is_empty())` before `.map(|e| e.parse().unwrap())`, matching the `allowed_signal_statuses` pattern.

---

### 10. [LOW] First CLI argument `.parse().unwrap()` panics on non-numeric input

`main.rs:33-34` — If a user passes a non-numeric first argument, the program panics with an unhelpful message.

```rust
let first_arg: u64 = std::env::args()
    .nth(1)
    .map_or(999_999_999_999_999, |x| x.parse().unwrap());
```

**Fix:** Use `.parse().unwrap_or(999_999_999_999_999)` or provide a proper error message.

---

## Performance Issues

### 11. [MEDIUM] `calculate_number_of_files` and `check_files_number` duplicate work

`common.rs:276-296` — `check_files_number` and `calculate_number_of_files` both do full directory walks, and are called back-to-back in `main.rs` for `valid_input_files_dir`. The directory is walked at least twice at startup.

```rust
check_files_number("Valid input dir", &settings.valid_input_files_dir);
// ... later ...
info!("Found {} files in valid input dir", calculate_number_of_files(&settings.valid_input_files_dir));
```

**Fix:** Merge into a single function or cache the count.

---

### 12. [LOW] `max_depth(999)` used everywhere instead of unbounded

`common.rs:44,284,290` and `remove_non_crashing_files.rs:268` — `max_depth(999)` is used as a proxy for "unlimited". This is functionally equivalent but less clear than omitting the constraint entirely (jwalk's default is unlimited).

```rust
WalkDir::new(dir).max_depth(999).into_iter()
```

**Fix:** Remove `.max_depth(999)` to use the default unlimited depth.

---

### 13. [LOW] `collect_files` size tracking includes truncated files

`common.rs:335-340` — `size_all` accumulates sizes of all matching files, but then `files.truncate(max_collected_files)` discards some. The reported `files_size` is therefore inaccurate (too high).

```rust
files.push(s.to_string());
size_all += metadata.len();
// ... later ...
files.truncate(settings.max_collected_files);
```

**Fix:** Calculate `size_all` after truncation, or track it alongside the files that survive.

---

## Code Quality / Maintainability

### 14. [MEDIUM] Massive code duplication between `execute_command_and_connect_output` and `execute_command_on_pack_of_files`

`common.rs:113-160` and `common.rs:224-274` — These two functions share ~80% identical logic (signal checking, status code checking, timeout detection, output formatting). Only the command invocation differs.

**Fix:** Extract the shared output analysis logic into a helper function.

---

### 15. [MEDIUM] Large blocks of commented-out code

Multiple files contain significant blocks of commented-out code:
- `main.rs:38-41` (rayon thread pool)
- `remove_non_crashing_files.rs:32-40` (second pass logic)
- `remove_non_crashing_files.rs:108-112` (group processing)
- `finding_text_status.rs:147-165` (debug/save logic)
- `obj.rs:136-147` (multiple unused methods)

This makes the code harder to navigate. If it's needed for reference, it belongs in git history.

**Fix:** Remove commented-out code. Use version control for historical reference.

---

### 16. [MEDIUM] `JS_VUE_SVELTE` is a byte-for-byte duplicate of `JAVASCRIPT_ARGS`

`broken_files.rs:57-66` vs `broken_files.rs:46-55` — These two constants are identical. If they diverge in the future, it would be intentional; but currently it's confusing.

```rust
const JAVASCRIPT_ARGS: &[&str] = &[ ... ];
const JS_VUE_SVELTE: &[&str] = &[ ... ];  // same content
```

**Fix:** Either reference the same constant or add a comment explaining why they're separate.

---

### 17. [LOW] `&Box<dyn ProgramConfig>` used everywhere instead of `&dyn ProgramConfig`

Throughout the codebase, functions accept `&Box<dyn ProgramConfig>`. The `Box` indirection is unnecessary when you already have a reference.

```rust
pub(crate) fn execute_command_and_connect_output(obj: &Box<dyn ProgramConfig>, ...) -> OutputResult {
```

The `#![allow(clippy::borrowed_box)]` at the top of `main.rs` suppresses this Clippy lint globally.

**Fix:** Change to `&dyn ProgramConfig`. This removes an unnecessary indirection and is the idiomatic Rust pattern.

---

### 18. [LOW] Duplicated ASAN environment setup

`obj.rs:62-67` and `apps/custom.rs:54-59` — The exact same ASAN environment variables are set in two places. `CustomStruct::get_full_command` overrides the trait default, duplicating the ASAN logic.

```rust
command.envs([
    ("RUST_BACKTRACE", "1"),
    ("ASAN_SYMBOLIZER_PATH", "/usr/bin/llvm-symbolizer"),
    ("ASAN_OPTIONS", "symbolize=1"),
]);
```

**Fix:** Let `CustomStruct` call the default trait implementation via `self.get_full_command_default()` or extract the ASAN setup to a shared helper.

---

### 19. [LOW] Settings loaded as `HashMap<String, HashMap<String, String>>` — no validation

`settings.rs:167-233` — All settings are deserialized as raw string hashmaps and manually parsed. Missing keys panic with unhelpful `HashMap` "key not found" messages. A typo in the config file produces an opaque crash.

**Fix:** Define a proper serde struct for the config and deserialize directly, or at minimum use `.get()` with descriptive error messages instead of indexing.

---

### 20. [LOW] `PYTHON_ARGS` contains duplicates

`broken_files.rs:36-44` — Several entries appear multiple times: `"None"` (2x), `"False"` (2x), `"True"` (2x), `"is not None"` (2x), `"is False"` (2x), `"is True"` (2x), `"is not True"` (2x), `"\""` (2x).

```rust
const PYTHON_ARGS: &[&str] = &[
    // ...
    "None", // appears at both line 36 and 41
    "False", "True", // appear at both lines 36-37 and 41-42
    // ...
];
```

Similar duplicates exist in `GDSCRIPT_ARGS` (e.g., `"and"`, `"or"`, `"not"`, `"is"`, `"in"`, `"static"`, `"const"`, `"enum"`, `"signal"`, `"func"`, `"class_name"`, `"extends"`, `"tool"`, `"var"`, `"pass"`, `"match"`).

**Fix:** Remove duplicate entries.

---

### 21. [LOW] `collect_command_to_string` quotes args containing `/` — unnecessarily noisy

`common.rs:356` — Any argument containing `/` (i.e., every file path) gets quoted. This makes logged commands harder to read and copy-paste.

```rust
if [" ", "\"", "\\", "/"].iter().any(|e| tmp_string.contains(e)) {
    format!("\"{}\"", tmp_string.replace('"', "\\\""))
```

**Fix:** Remove `/` from the quoting trigger list. Paths with slashes don't need quoting in shell commands.

---

## Suspicious Logic

### 22. [MEDIUM] `save_results_to_file` replaces `"\n\n```"` with `"\n```"` — may corrupt report content

`remove_non_crashing_files.rs:255` — A blanket string replacement removes blank lines before triple-backtick fences. If the error output itself contains `\n\n````, this will corrupt it.

```rust
.replace("\n\n```", "\n```");
```

**Fix:** Only apply formatting fixes to the template structure, not the entire output string including user data.

---

### 23. [LOW] `MAX_FILES` constant is `999_999_999_999` — not a realistic limit

`remove_non_crashing_files.rs:21` — `.take(MAX_FILES)` with a ~1 trillion limit is effectively a no-op.

```rust
pub const MAX_FILES: usize = 999_999_999_999;
// ...
let broken_files: Vec<String> = collect_broken_files(settings).into_iter().take(MAX_FILES).collect();
```

**Fix:** Remove the `.take(MAX_FILES)` or set a meaningful limit.

---

### 24. [LOW] `is_status_code_broken` logic inverted from what the name suggests

`common.rs:132-136` — The condition checks that the status code is **not** in `allowed_error_statuses`. The variable name `is_status_code_broken` combined with `allowed_error_statuses` is confusing — "allowed errors" that are "not broken" is a double negative.

```rust
let is_status_code_broken = !obj.get_settings().allowed_error_statuses.is_empty()
    && output.status.code()
        .is_some_and(|code| !obj.get_settings().allowed_error_statuses.contains(&code));
```

This means: "if we have a whitelist, and the code is NOT on it, mark as broken." The semantics are correct but the naming (`allowed_error_statuses`) is misleading — these are actually the *only allowed* statuses, not specifically error statuses.

**Fix:** Rename to `allowed_exit_codes` or add a clarifying comment.

---

## Summary

| Severity | Count |
|----------|-------|
| High     | 3     |
| Medium   | 7     |
| Low      | 14    |

**Key takeaways:**
- The zip writer silently discards errors (finding #1) — fix immediately.
- The `set_mode` call is a no-op (finding #2) — either fix or remove.
- Extension `.unwrap()` calls will panic on extensionless files (finding #3).
- `allowed_error_statuses` parsing is inconsistent with `allowed_signal_statuses` (finding #9) — likely a latent bug.
- Significant code duplication and commented-out code reduce maintainability.
