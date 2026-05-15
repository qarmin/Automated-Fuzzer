# Config lives in crates/<name>/fuzz_settings_ci.toml for local crates,
# or configs/external/fuzz_<name>_settings_ci.toml for external tools.
# This helper resolves the right path.
[private]
config-path name:
    @if [ -f "crates/{{name}}/fuzz_settings_ci.toml" ]; then echo "crates/{{name}}/fuzz_settings_ci"; \
    elif [ -f "configs/external/fuzz_{{name}}_settings_ci.toml" ]; then echo "configs/external/fuzz_{{name}}_settings_ci"; \
    else echo "fuzz_settings" ; fi

#  Build

# Build the fuzzer binary
build:
    cargo build --release

# Build all wrapper crates
build-crates:
    for dir in crates/*/; do \
        if [ -f "$dir/Cargo.toml" ]; then \
            echo "Building $dir" && (cd "$dir" && cargo build --release) || true; \
        fi; \
    done

# Build a specific crate: just build-crate symphonia
build-crate name:
    cd crates/{{name}} && cargo build --release

# Build a crate with ASAN (nightly required)
build-crate-asan name:
    cd crates/{{name}} && RUSTFLAGS="-Zsanitizer=address" cargo +nightly build --release --target x86_64-unknown-linux-gnu

# Install the fuzzer binary
install:
    cargo install --path .

# Install a wrapper crate globally: just install-crate symphonia
install-crate name:
    cargo install --path crates/{{name}}

# Install external tools (create_broken_files + minimizer)
install-tools:
    cargo install create_broken_files minimizer

#  Fuzzing

# Run custom fuzzer with a config: just fuzz symphonia 3600
fuzz name timeout="0":
    auto_fuzzer fuzz --mode custom --config $(just config-path {{name}}).toml --timeout {{timeout}}

# Run custom fuzzer with the default fuzz_settings.toml
fuzz-default timeout="0":
    auto_fuzzer fuzz --mode custom --timeout {{timeout}}

# Run cargo-fuzz for a target: just cargo-fuzz image 3600
cargo-fuzz name timeout="3600":
    cd fuzz && cargo update && cd .. && \
    auto_fuzzer fuzz --mode cargo-fuzz --target {{name}} --corpus /opt/INPUT_FILES_DIR \
        --features "{{name}}_f" --timeout {{timeout}}

#  Results

# Remove non-crashing files (verification pass with ASAN)
verify name:
    #!/usr/bin/env bash
    CONFIG=$(just config-path {{name}})
    cp "${CONFIG}.toml" fuzz_settings.toml
    sd "remove_non_crashing_items_from_broken_files = false" "remove_non_crashing_items_from_broken_files = true" fuzz_settings.toml
    sd "TMP_FOLDER_TO_REPLACE" "/tmp/tmp_folder/data" fuzz_settings.toml
    export RUST_BACKTRACE=1
    export ASAN_SYMBOLIZER_PATH=$(which llvm-symbolizer-18 2>/dev/null || which llvm-symbolizer)
    export ASAN_OPTIONS="symbolize=1"
    auto_fuzzer fuzz --mode custom

# Minimize all broken files for a project
minimize name:
    auto_fuzzer minimize --config $(just config-path {{name}}).toml

# List crash reports
reports dir="/tmp/tmp_folder/data":
    auto_fuzzer report list --dir {{dir}}

# Generate issue report for a crash directory
report-create dir repo="" version="" variant="cli":
    auto_fuzzer report create --dir {{dir}} \
        {{ if repo != "" { "--repo " + repo } else { "" } }} \
        {{ if version != "" { "--version " + version } else { "" } }} \
        --variant {{variant}}

#  Ignore list

ignore-add project pattern url:
    auto_fuzzer ignore add "{{project}}" "{{pattern}}" "{{url}}"

ignore-list project="":
    auto_fuzzer ignore list {{ if project != "" { "--project " + project } else { "" } }}

# Check if ignored issues have been closed (read-only)
ignore-verify:
    auto_fuzzer ignore verify

# Check and remove entries for closed issues
ignore-clean:
    auto_fuzzer ignore clean

#  CI

ci-run name timeout state_dir="/opt/fuzzer_state" output_dir="/opt/results":
    auto_fuzzer ci run --config $(just config-path {{name}}).toml \
        --timeout {{timeout}} --state-dir {{state_dir}} --output-dir {{output_dir}}

ci-verify name state_dir="/opt/fuzzer_state":
    auto_fuzzer ci verify-regressions --config $(just config-path {{name}}).toml --state-dir {{state_dir}}

#  Maintenance

upgrade:
    cargo +nightly -Z unstable-options update --breaking
    cargo update
    for dir in crates/*/; do if [ -f "$dir/Cargo.toml" ]; then (cd "$dir" && cargo +nightly -Z unstable-options update --breaking && cargo update) || true; fi; done
    cd fuzz && cargo update && cd ..

# Update all deps and verify every crate compiles
upgrade-check:
    python3 check_crates.py --upgrade

# Check all crates compile (without updating)
check-crates:
    python3 check_crates.py

fix:
    cargo +nightly fmt
    cargo clippy --fix --allow-dirty --allow-staged
    cargo +nightly fmt

fix-crates:
    for dir in crates/*/; do \
        if [ -f "$dir/Cargo.toml" ]; then \
            (cd "$dir" && cargo +nightly fmt 2>/dev/null && cargo clippy --fix --allow-dirty --allow-staged 2>/dev/null) || true; \
        fi; \
    done

test:
    cargo test

#  Setup helpers

setup-dirs:
    mkdir -p /opt/VALID_FILES_DIR /opt/BROKEN_FILES_DIR /opt/POSSIBLY_BROKEN_FILES_DIR /tmp/tmp_folder/data /opt/CUSTOM /opt/INPUT_FILES_DIR

download files:
    #!/usr/bin/env bash
    CURR_DIR=$(pwd)
    cd /opt/VALID_FILES_DIR
    python3 "$CURR_DIR/download_helper.py" "{{files}}"
    cd "$CURR_DIR"

# Prepare a project for local fuzzing: just prepare symphonia "AA_MUSIC_VALID_FILES.7z"
# For structured fuzzing (no corpus): just prepare pdf_writer --generate
prepare name files="":
    #!/usr/bin/env bash
    set -e
    echo "=== Setting up {{name}} ==="
    mkdir -p /opt/VALID_FILES_DIR /opt/BROKEN_FILES_DIR /opt/POSSIBLY_BROKEN_FILES_DIR /tmp/tmp_folder/data /opt/CUSTOM

    if [ -z "$(ls -A /opt/VALID_FILES_DIR 2>/dev/null)" ]; then
        if [ "{{files}}" = "--generate" ]; then
            echo "Generating 500 random seed files..."
            python3 -c "
    import os, random
    for i in range(500):
        size = random.randint(8, 2048)
        with open(f'/opt/VALID_FILES_DIR/seed_{i}.bin', 'wb') as f:
            f.write(random.randbytes(size))
    "
        elif [ -n "{{files}}" ]; then
            echo "Downloading test files..."
            CURR_DIR=$(pwd)
            cd /opt/VALID_FILES_DIR
            python3 "$CURR_DIR/download_helper.py" "{{files}}"
            cd "$CURR_DIR"
        else
            echo "ERROR: Specify corpus files or --generate"
            echo "  just prepare symphonia \"AA_MUSIC_VALID_FILES.7z\""
            echo "  just prepare pdf_writer --generate"
            exit 1
        fi
    else
        echo "Valid files already exist, skipping"
    fi

    echo "Building {{name}} crate..."
    cd crates/{{name}} && cargo update && cargo build --release
    BINARY=$(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c "import sys,json; t=json.load(sys.stdin)['packages'][0]['targets']; print([x['name'] for x in t if 'bin' in x['kind']][0])" 2>/dev/null || echo "{{name}}")
    sudo cp "target/release/$BINARY" "/usr/local/bin/{{name}}_normal" 2>/dev/null || cp "target/release/$BINARY" "$HOME/.cargo/bin/{{name}}_normal"
    cd ../..

    CONFIG=$(just config-path {{name}})
    cp "${CONFIG}.toml" fuzz_settings.toml
    sd "TMP_FOLDER_TO_REPLACE" "/tmp/tmp_folder/data" fuzz_settings.toml

    echo ""
    echo "=== Ready! Run: just fuzz {{name}} 3600 ==="

# Full pipeline: prepare + fuzz + verify + report
pipeline name files timeout="3600":
    just prepare {{name}} "{{files}}"
    just fuzz {{name}} {{timeout}}
    just verify {{name}}
    just reports

list-crates:
    @ls -1 crates/
