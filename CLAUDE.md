# CLAUDE.md

Operational guide for working in this repo: adding fuzz targets, verifying they compile,
updating dependencies, and running locally. For the high-level overview of the two fuzzing
modes and the CI flow, see `README.md` and `ci_process.md`.

## Layout (where things live)

```
src/                        auto_fuzzer binary (CLI orchestrator)
crates/<name>/              local wrapper crate (one per fuzzed library)
  src/main.rs               check_file that exercises the library API
  Cargo.toml                git = dependency on the library
  fuzz_settings_ci.toml     fuzz config (search/ignore patterns, extensions...)
crates/fuzz_utils/          shared lib: fuzz_utils::run + ByteInput
fuzz/fuzz_targets/<name>.rs cargo-fuzz (libfuzzer) target
fuzz/Cargo.toml             cargo-fuzz workspace, one optional feature `<name>_f` per target
configs/external/           fuzz configs for tools WITHOUT a local crate (biome, swc, oxc...)
.github/workflows/fuzz.yml  CI: one job per matrix.include entry
justfile                    every recipe below
check_crates.py             compile-check all crates + fuzz targets (+ optional dep update)
```

There are two kinds of target. Pick by whether the thing being fuzzed is a **Rust library**
(call its API from our own binary) or an **external CLI tool** (clone its repo, build its
binary, run it as a subprocess).

---

## A. Adding a local-crate target (fuzz a Rust library)

Touch 4 places. Use `crates/zune/` as the template for file parsers, `crates/pdf_writer/`
for structured fuzzing (libraries that generate output from bytes instead of parsing a file).

### 1. `crates/<name>/Cargo.toml`

The library dependency MUST be a `git =` source (never `path =`, never a crates.io version -
we fuzz upstream HEAD). Profile must keep overflow/debug assertions on:

```toml
[package]
name = "<name>"
version = "0.1.0"
edition = "2021"

[dependencies]
<library> = { git = "https://github.com/OWNER/REPO.git" }
fuzz_utils = { path = "../fuzz_utils" }

[profile.release]
overflow-checks = true
panic = "abort"
debug = true
debug-assertions = true
```

### 2. `crates/<name>/src/main.rs`

Use `fuzz_utils::run` (it handles file/dir walking). Catch and ignore `Err` - the fuzzer
detects crashes from panics/ASAN/signals, NOT from `Result::Err`.

```rust
use std::fs;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else { return };
    let _ = library::parse(&data); // ignore Err; a panic/abort is the signal
}
```

For structured fuzzing, drive the API from deterministic bytes via `fuzz_utils::ByteInput`
and write a standalone reproducer (see `crates/pdf_writer/src/main.rs`). Reproducers MUST be
paste-runnable with zero `fuzz_utils` deps.

### 3. `crates/<name>/fuzz_settings_ci.toml`

Copy from a neighbour and edit the `[custom]` section. See the config reference below.

### 4. (Optional) cargo-fuzz target

Add `fuzz/fuzz_targets/<name>.rs` (libfuzzer style, see `fuzz/fuzz_targets/zune.rs`), then in
`fuzz/Cargo.toml` add the optional dependency, a `<name>_f = ["<dep>"]` feature, and a `[[bin]]`
entry. `check_crates.py` discovers targets by scanning for `*_f` features.

### 5. Wire into CI (`.github/workflows/fuzz.yml`)

Add a `matrix.include` entry. Custom mode:

```yaml
- name: <NAME_UPPER>
  mode: custom
  crate: <name>
  binary: <name>
  files: "<CORPUS_ARCHIVE>.7z"
```

Cargo-fuzz mode appends `_CF` to the name (stripped later so both modes share one crash
archive) and adds `fuzz_target` + `fuzz_features`. Structured fuzzing sets
`generate_corpus: true` + `corpus_ext`/`corpus_count`/`corpus_max_size` instead of `files`.

---

## B. Adding an external-tool target (fuzz a CLI binary)

For tools with their own repo and CLI (biome, swc, ruff, oxc). No local crate; the config
lives in `configs/external/fuzz_<name>_settings_ci.toml` and CI resolves it from the lowercased
matrix `name`.

**Always compile the tool locally in `/tmp` first** to learn (a) the binary name, (b) the exact
CLI invocation for linting/formatting a file, (c) the exit codes on good/bad/broken input, and
(d) whether any non-default features are needed. Replicate CI conditions: strip the pinned
toolchain and build on nightly release.

```bash
cd /tmp && wget -q "https://github.com/OWNER/REPO/archive/refs/heads/main.zip" \
  && unzip -q main.zip && cd REPO-main
rm -f rust-toolchain.toml                 # CI strips this; build on current nightly
cargo +nightly build --release --bin <binary>
./target/release/<binary> <args> some.file ; echo "exit=$?"
```

Note the exit codes: anything in the config's `allowed_error_statuses` (typically `0,1,2,101,124`)
is treated as "ran fine", everything else as a potential crash. A tool that exits 1/2 on a normal
parse error is fine; a tool that panics on every input is not (that one needs a feature flag - see
the oxfmt note below).

### Config: `configs/external/fuzz_<name>_settings_ci.toml`

Copy `fuzz_swc_settings_ci.toml` (JS/TS) or another close match. The actual fuzzed command comes
from the `command` line, NOT from the matrix - so each distinct invocation needs its own config:

```toml
[custom]
name = "<name>"
command = "<binary>_normal|<subcmd-or-args>|FILE_PATHS_TO_PROVIDE"  # pipe-separated argv
extensions = "ts,js,mjs,mts"
```

`FILE_PATHS_TO_PROVIDE` is where the mutated file paths get spliced in. `<binary>_normal` is the
non-ASAN build CI installs to `/usr/local/bin`.

### CI matrix entry

```yaml
- name: <NAME_UPPER>
  mode: custom
  external: true
  clone_url: "https://github.com/OWNER/REPO/archive/refs/heads/main.zip"
  install_path: "REPO-main/path/to/binary/crate"   # dir passed to `cargo install --path`
  cargo_toml_dir: "REPO-main"                       # repo root; CI rewrites its [profile.release]
  binary: <binary>
  run_args: "<args>"            # only used by the "Print crash details" step; omit if none
  install_args: "--no-default-features"   # OPTIONAL extra flags for cargo install (see below)
  files: "<CORPUS_ARCHIVE>.7z"
```

CI clones the zip, removes any `rust-toolchain.toml`/`.cargo`, rewrites the repo's
`[profile.release]` (overflow-checks + debug-assertions + thin LTO), then runs
`cargo +nightly install --path <install_path> $install_args` for both a normal and an ASAN build.

**`install_args`**: extra flags forwarded to both `cargo install` invocations. Use it when the
default feature set produces a non-working binary. Example: oxc's `oxfmt` enables a `napi` feature
by default that makes the pure-Rust CLI panic on every input ("External formatter must be set when
`napi` feature is enabled"), so `OXFMT` sets `install_args: "--no-default-features"`. `oxlint` from
the same repo needs nothing. When in doubt, build both ways in `/tmp` and run the binary on a real
file before trusting it.

Two invocations of the same tool (e.g. biome `lint` vs `format`, oxc `oxlint` vs `oxfmt`) = two
configs + two matrix entries with distinct `name`s. Distinct names mean separate crash archives,
so results never collide.

---

## Config reference (`[custom]` section of fuzz_settings_ci.toml)

| Field | Meaning |
|---|---|
| `name` | project name, used in folder/report names |
| `command` | `<binary>_normal\|arg\|...\|FILE_PATHS_TO_PROVIDE`, pipe-separated argv |
| `extensions` | comma list; MUST match the corpus files or nothing gets tested |
| `file_type` | `binary` for images/audio/fonts; a language name (`js`, `python`, `rust`...) for source |
| `search_item_N` | substring in stdout+stderr that marks a crash |
| `ignored_item_N` | substring that suppresses a known crash - ALWAYS add the GitHub issue URL in a comment |
| `allowed_error_statuses` | exit codes treated as non-crash (e.g. `0,1,2,101,124`) |
| `group_mode` / `grouping` | `by_group` batches N files per command run for speed |
| `stability_mode` | re-run check for output stability |

Standard crash patterns to include:

```toml
search_item_1 = "RUST_BACKTRACE"
search_item_2 = "panicked at"
search_item_100 = "AddressSanitizer"
search_item_101 = "LeakSanitizer"
search_item_102 = "ThreadSanitizer"
search_item_103 = "timeout: sending signal"
search_item_125 = "(core dumped)"
search_item_126 = "segmentation fault"
```

Ignore patterns have two homes: `ignored_item_N` here (per project, filters during fuzzing) and
the CLI-managed global `ignore_list.toml` (`auto_fuzzer ignore add/list/verify/clean`). Both must
carry a GitHub issue URL so `ignore clean` can drop entries once the issue is closed.

---

## Checking that things compile

`check_crates.py` is the source of truth: it runs `cargo check` on every crate in `crates/`, every
`*_f` fuzz target in `fuzz/`, and the main binary, then prints a pass/fail summary with the tail of
each failure log.

```bash
just check-crates          # python3 check_crates.py  - check only
python3 check_crates.py     # same thing directly
```

Single crate while iterating:

```bash
just build-crate <name>            # cargo build --release in crates/<name>
just build-crate-asan <name>       # nightly ASAN build (what CI's crash-repro uses)
```

External tools have no local crate, so `check_crates.py` does not cover them - verify those by
building in `/tmp` as shown in section B.

---

## Updating dependencies

Because every target tracks upstream HEAD via `git =`, deps drift constantly. Two recipes:

```bash
just upgrade           # update main + every crate + fuzz, NO compile check (fast, can leave breakage)
just upgrade-check     # python3 check_crates.py --upgrade  - update everything THEN cargo check all
```

`upgrade-check` is the safe one - it updates and immediately tells you which crates stopped
compiling. Prefer it before committing a dependency bump. Both use the nightly breaking-update
flow: `cargo +nightly -Z unstable-options update --breaking` followed by `cargo update`, applied to
the root, each `crates/*`, and `fuzz/`.

When a crate fails to compile after an upgrade, it is almost always an upstream API change in the
fuzzed library - fix `crates/<name>/src/main.rs` to match the new API, not the dependency version
(we intentionally follow HEAD).

---

## Running locally

```bash
just setup-dirs                              # create /opt/*_DIR and /tmp/tmp_folder/data
just prepare <name> "<CORPUS>.7z"            # download corpus + build crate + write fuzz_settings.toml
just prepare <name> --generate               # random seed corpus (structured fuzzing)
just fuzz <name> 3600                        # custom-mode fuzz for 1h (0 = no timeout)
just cargo-fuzz <name> 3600                  # libfuzzer mode
just verify <name>                           # ASAN re-run, drops files that no longer crash
just reports                                 # list grouped crash reports
just pipeline <name> "<CORPUS>.7z" 3600      # prepare + fuzz + verify + reports
just list-crates                             # all local crates
```

Crashes land in `/opt/BROKEN_FILES_DIR`; grouped reports (one representative per error signature)
in `/tmp/tmp_folder/data`. Available corpus archives are listed in the README / fuzzer-project
skill (image, audio, pdf, font, js/ts, python, etc.).

---

## Gotchas

- **`git =` only** in local crate Cargo.toml. A `path =` or crates.io version means you are not
  fuzzing upstream.
- **Extensions must match the corpus.** `.jpg/.png` corpus + `extensions = "webp"` fuzzes nothing.
- **Ignore `Err`, never `unwrap` defensively** in `check_file` - a panic is the thing we want to
  catch, but an `unwrap` you add yourself is a false positive.
- **External tools: confirm the binary actually works on a real file before adding it.** A tool that
  needs `--no-default-features` (or any feature flag) will otherwise "crash" on every input and
  flood the report.
- **Every `ignored_item_N` needs a GitHub issue URL** in a trailing comment, or `ignore clean`
  cannot retire it.
- **`run_args` in the matrix only affects the crash-details print step.** The real command is the
  `command` line in the TOML.
```
