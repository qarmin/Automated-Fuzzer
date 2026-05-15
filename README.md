# auto_fuzzer

Automated fuzzing tool for finding crashes in CLI tools and Rust libraries. Supports two fuzzing modes (custom mutation-based and cargo-fuzz/libfuzzer), crash grouping by error signature, minimization, ignore lists with GitHub issue tracking, and CI integration with state persistence.

## Installation

```bash
cargo install --path .
```

External tools (installed separately):
```bash
cargo install create_broken_files minimizer
```

## Quick start

### 1. Custom fuzzing mode

Requires a `fuzz_settings.toml` config file (see `old/fuzz_settings.toml` for examples).

```bash
# Fuzz for 1 hour
auto_fuzzer fuzz --mode custom --timeout 3600

# Fuzz with a specific config
auto_fuzzer fuzz --mode custom --config fuzz_lofty_settings.toml --timeout 7200

# Fuzz indefinitely (Ctrl+C to stop gracefully, second Ctrl+C to force quit)
auto_fuzzer fuzz --mode custom
```

### 2. Cargo-fuzz mode

Wrapper around `cargo fuzz run` with result collection and graceful shutdown.

```bash
# Fuzz the "image" target for 5 hours
auto_fuzzer fuzz --mode cargo-fuzz --target image --corpus /opt/INPUT_FILES --timeout 18000

# With features and 8 parallel jobs
auto_fuzzer fuzz --mode cargo-fuzz --target symphonia --corpus /opt/MUSIC \
    --features "symphonia_f" --jobs 8 --timeout 3600
```

### 3. Minimize crashes

Runs the external `minimizer` on all crash files, keeping originals untouched.

```bash
# Minimize all files in the configured broken_files_dir
auto_fuzzer minimize

# Minimize files from a specific directory
auto_fuzzer minimize --dir /opt/BROKEN_FILES_DIR
```

### 4. View and create reports

```bash
# List all unique crashes grouped by error signature
auto_fuzzer report list --dir /tmp/tmp_folder/data

# Generate a GitHub issue draft for a specific crash
auto_fuzzer report create --dir /tmp/tmp_folder/data/lofty_overflow_s__42_bytes_-_123456 \
    --repo "Serial-ATA/lofty-rs" \
    --version "abc1234" \
    --variant cli

# Generate issue drafts for all crashes
auto_fuzzer report create-all --dir /tmp/tmp_folder/data --project lofty
```

This creates `issue_title.txt`, `issue_body.md`, and `create_issue.sh` in each crash directory. Review and run:

```bash
bash /tmp/tmp_folder/data/lofty_overflow_s__.../create_issue.sh
# Then manually attach compressed.zip through the GitHub web UI
```

### 5. Manage ignore list

Track known bugs so they don't clutter results.

```bash
# Add a known issue
auto_fuzzer ignore add lofty "src/wavpack/properties.rs" \
    "https://github.com/Serial-ATA/lofty-rs/issues/620"

auto_fuzzer ignore add rawler "src/bits.rs" \
    "https://github.com/dnglab/dnglab/issues/571"

# List all ignored patterns
auto_fuzzer ignore list
auto_fuzzer ignore list --project lofty

# Remove an entry
auto_fuzzer ignore remove lofty "src/wavpack/properties.rs"
```

### 6. Validate issue links

Check if ignored issues have been closed upstream (requires `gh` CLI authenticated).

```bash
# Check all links
auto_fuzzer validate links

# Automatically remove entries for closed issues
auto_fuzzer validate links --auto-remove
```

Example output:
```
[FIXED] lofty "src/wavpack/properties.rs" - https://github.com/.../issues/620 is CLOSED
[OPEN]  rawler "src/bits.rs" - https://github.com/.../issues/571 still open
```

### 7. CI mode

Run in GitHub Actions with persistent state between runs (corpus, known crashes, history).

```bash
# Run fuzzer with state persistence
auto_fuzzer ci run \
    --config fuzz_lofty_settings.toml \
    --timeout 16000 \
    --state-dir /opt/fuzzer_state \
    --output-dir /opt/results

# Check if previously found crashes are still reproducible
auto_fuzzer ci verify-regressions \
    --config fuzz_lofty_settings.toml \
    --state-dir /opt/fuzzer_state
```

Example output of `verify-regressions`:
```
[STILL BROKEN] /opt/fuzzer_state/known_crashes/file1.mp3 - attempt to subtract with overflow
[FIXED] /opt/fuzzer_state/known_crashes/file2.mp3

Regression check: 1 still broken, 1 fixed
```

### 8. Legacy mode

Backward-compatible with the old `automated_fuzzer` CLI.

```bash
# Same as old: automated_fuzzer 3600
auto_fuzzer legacy 3600

# Remove non-crashing files (verification pass)
auto_fuzzer legacy --remove-non-crashing
```

## Configuration

The tool reads `fuzz_settings.toml` (or a custom path via `--config`). Minimal example:

```toml
[general]
loop_number = 100
broken_files_for_each_file = 10
minimize_output = true
temp_possible_broken_files_dir = "/tmp/AA_BROKEN_INPUT_FILES"
minimization_attempts = 200
minimization_repeat = true
minimization_time = 1800
debug_print_results = false
debug_executed_commands = false
debug_print_broken_files_creator = false
remove_non_crashing_items_from_broken_files = false
check_for_stability = false
stability_runs = 3
temp_folder = "/tmp/tmp_folder/data"
timeout_group = 400
timeout = 100
allowed_signal_statuses = ""
allowed_error_statuses = "0,1,2,101"
max_file_size_limit = 5000000
max_collected_files = 999999999999999
ignore_file_if_contains_searched_items = true
check_if_file_is_parsable = false
grouping = 100
custom_folder_path = "/opt/CUSTOM"
minimization_attempts_with_signal_timeout = 10
current_mode = "custom"

[custom]
name = "my_tool"
command = "my_tool|lint|FILE_PATHS_TO_PROVIDE"
extensions = "rs"
valid_input_files_dir = "/opt/VALID_FILES_DIR"
broken_files_dir = "/opt/BROKEN_FILES_DIR"
group_mode = "none"
search_item_1 = "RUST_BACKTRACE"
search_item_2 = "panicked at"
search_item_100 = "AddressSanitizer"
file_type = "rust"
stability_mode = "none"
```

Key fields:
- `command` - pipe-separated binary and args, with `FILE_PATHS_TO_PROVIDE` as the placeholder
- `search_item_N` - patterns that indicate a crash (matched against stdout+stderr)
- `ignored_item_N` - patterns to skip (known issues, too-big-to-minimize, etc.)
- `file_type` - mutation strategy: `text`, `binary`, `rust`, `python`, `js`, `go`, `lua`, `slint`, `jsvuesvelte`, `svg`, `gdscript`

## Error grouping

Crashes are grouped by a two-level signature: `{error_type}::{source_file}`. Examples:

| Crash output | Signature | Issue title |
|---|---|---|
| `attempt to subtract with overflow` in `src/wavpack/properties.rs` | `overflow_subtract::src/wavpack/properties.rs` | Panic attempt to subtract with overflow in src/wavpack/properties.rs |
| `assertion failed: a > 25` in `src/foo.rs` | `assertion::src/foo.rs::a > N` | Panic assertion failed: a > N in src/foo.rs |
| `assertion failed: b > 25` in `src/foo.rs` | `assertion::src/foo.rs::b > N` | Panic assertion failed: b > N in src/foo.rs |
| `index out of bounds` in `src/wavpack/properties.rs` | `index_out_of_bounds::src/wavpack/properties.rs` | Panic index out of bounds in src/wavpack/properties.rs |
| `entered unreachable code: Bad BOM` | `unreachable_code::src/id3/v2/read.rs::Bad BOM [0, 0]` | Panic internal error: entered unreachable code: Bad BOM in src/id3/v2/read.rs |
| timeout | `timeout` | Panic timeout when processing file |

Line numbers are stripped (they change between versions). Numeric values >= 10 in assertions are replaced with `N`.

## Graceful shutdown

Press `Ctrl+C` once to finish the current iteration and save all results found so far. Press again to force quit. Works in both custom and cargo-fuzz modes.

## License

MIT
