# auto_fuzzer

Automated fuzzing tool for Rust libraries and CLI tools. Finds panics, overflows, OOM, timeouts, ASAN errors, and other crashes.

Two fuzzing modes:
- **Custom** — mutates valid files with `create_broken_files`, runs a wrapper binary, detects crashes from output patterns
- **Cargo-fuzz** — uses libfuzzer via `cargo fuzz run`, restarts in a loop until timeout

Both modes share crash archives, reports, and ignore lists. Results are grouped by error signature (type + source file + line).

## Project structure

```
src/                     auto_fuzzer binary (CLI orchestrator)
crates/<name>/           wrapper binaries that exercise a library's API
  src/main.rs            uses fuzz_utils::run(check_file)
  fuzz_settings_ci.toml  fuzzing config (search/ignore patterns, extensions, etc.)
crates/fuzz_utils/       shared lib: file walking boilerplate + structured ByteInput
fuzz/fuzz_targets/       cargo-fuzz (libfuzzer) targets
fuzz/Cargo.toml          cargo-fuzz workspace with optional features per target
configs/external/        fuzz configs for tools without a local crate (biome, ruff, swc...)
.github/workflows/       CI (single fuzz.yml with mode: custom or cargo-fuzz in matrix)
```

## Installation

```bash
cargo install --path .
cargo install create_broken_files minimizer  # external tools
```

## Quick start with justfile

```bash
just list-crates                              # show all 27 crates
just check-crates                             # verify all compile (crates + fuzz targets)
just upgrade-check                            # update deps + verify

# Prepare + fuzz a project
just prepare symphonia "AA_MUSIC_VALID_FILES.7z"
just fuzz symphonia 3600

# Structured fuzzing (no file corpus needed)
just prepare pdf_writer --generate
just fuzz pdf_writer 3600

# Full pipeline: prepare → fuzz → verify (ASAN) → reports
just pipeline zune "AA_IMAGE_VALID_FILES.7z" 3600

# Verify results, list reports
just verify symphonia
just reports
```

## Fuzzing modes

### Custom mode

```bash
auto_fuzzer fuzz --mode custom --timeout 3600
auto_fuzzer fuzz --mode custom --config crates/symphonia/fuzz_settings_ci.toml --timeout 7200
```

### Cargo-fuzz mode

```bash
auto_fuzzer fuzz --mode cargo-fuzz --target image --corpus /opt/INPUT_FILES \
    --features "image_f" --timeout 3600
```

## Minimize crashes

Runs the external `minimizer` on crash files, keeping originals.

```bash
auto_fuzzer minimize
auto_fuzzer minimize --dir /opt/BROKEN_FILES_DIR
```

## Reports

Each crash generates a folder like `zune__assertion_eq__src_image.rs_482/397_bytes_12345/` containing:
- `to_report.txt` — full crash report with reproducer code
- `to_report_metadata.toml` — structured metadata (type, signature, source file, line)
- `crash_output.txt` — raw stdout+stderr
- `compressed.zip` — crash file
- `issue_title.txt` — ready-made title with backticks
- `issue_body.md` — ready-made body
- `create_issue.sh` — one-click: creates GitHub issue, opens browser to attach zip

```bash
auto_fuzzer report list --dir /tmp/tmp_folder/data
bash /tmp/tmp_folder/data/zune__assertion_eq__src_image.rs_482/397_bytes_12345/create_issue.sh
```

The repo is auto-detected from `crates/<name>/Cargo.toml` git dependency. For library crates, the report includes a simplified reproducer with `check_file` code. For structured fuzzing crates (pdf_writer), the reproducer has all `input.xxx()` calls replaced with hardcoded values.

## Ignore list

Two mechanisms work together:

### Config-level (`ignored_item_N` in fuzz_settings_ci.toml)

Filters crashes **during fuzzing** — matched patterns are never saved. Always add a comment with the issue URL:

```toml
ignored_item_1 = "stack-overflow"          # https://github.com/boa-dev/boa/issues/1402
ignored_item_2 = "memory allocation of"    # https://github.com/boa-dev/boa/issues/5367
ignored_item_3 = "src/bits.rs"             # https://github.com/dnglab/dnglab/issues/571
```

### Global (`ignore_list.toml`)

Managed via CLI, independent of fuzzing:

```bash
auto_fuzzer ignore add lofty "src/wavpack/properties.rs" "https://github.com/Serial-ATA/lofty-rs/issues/620"
auto_fuzzer ignore remove lofty "src/wavpack/properties.rs"
```

### List, verify, clean

```bash
# Show all ignored patterns (both ignore_list.toml and all fuzz configs)
auto_fuzzer ignore list
auto_fuzzer ignore list --project hayro

# Check if issues have been closed (read-only)
auto_fuzzer ignore verify

# Check and remove entries for closed issues
auto_fuzzer ignore clean
```

`verify` and `clean` check both `ignore_list.toml` and every `crates/*/fuzz_settings_ci.toml`. Entries without a GitHub URL are left untouched.

## Error grouping

Crashes are grouped by signature: `{error_type}::{source_file}`. Folders include the line number for disambiguation.

| Crash | Folder | Issue title |
|---|---|---|
| overflow in `src/wavpack/properties.rs:123` | `symphonia__overflow_s__src_wavpack_properties.rs_123/` | Panic `attempt to subtract with overflow` in `src/wavpack/properties.rs` |
| assertion_eq in `src/image.rs:482` | `zune__assertion_eq__src_image.rs_482/` | Panic `assertion \`left == right\` failed` in `src/image.rs` |
| timeout (no source) | `boa__timeout__unknown/` | Timeout when processing file |
| OOM (no source) | `boa__memory_failure__unknown/` | Memory allocation failure when processing file |
| ASAN heap-use-after-free | `foo__heap_use_after_free__src_buf.rs_10/` | Heap use after free `heap-use-after-free` in `src/buf.rs` |

## Structured fuzzing (fuzz_utils)

For libraries that **generate** output (not parse input), like `pdf-writer`:

```rust
use fuzz_utils::ByteInput;

fn main() {
    fuzz_utils::run(|path| {
        let mut input = ByteInput::from_file(path).unwrap();
        let count = input.u8_range("page_count", 1, 10).unwrap();
        let width = input.f32_range("width", 0.0, 1000.0).unwrap();
        // ... call library API with these values
    });
}
```

- Input file = random bytes, consumed sequentially
- Each call logs the decision: `page_count=3`, `width=595.2`
- Same file = same decisions = deterministic
- Minimizer shrinks the file = fewer API calls
- On crash: `.params` file + `.reproducer.rs` with hardcoded values

## CI

Single workflow `.github/workflows/fuzz.yml` with matrix entries for both modes. Default timeout is `DEFAULT_TIMEOUT` (env), overridable per-job with `custom_timeout`.

Flow:
1. Build binary (normal + ASAN), upload to Nightly release
2. Download/generate test corpus
3. Fuzz for N seconds
4. Verify new crashes (ASAN for custom, minimizer for cargo-fuzz)
5. Download previous crashes (post-fuzz = small race window)
6. Regression check (re-run old crashes, remove fixed ones)
7. Merge + dedup (MD5), upload with retry (race condition safe)
8. Print crash details in logs (exit code per file)
9. Generate reports, upload artifacts

Crashes persist across runs in `crashes_<NAME>.7z` on the Nightly release. Both `custom` and `cargo-fuzz` modes write to the same archive (suffix `_CF` is stripped).

## Graceful shutdown

`Ctrl+C` once = finish current iteration, save results. Again = force quit. Works in both modes.

## License

MIT
