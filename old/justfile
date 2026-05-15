upgrade:
    cargo +nightly -Z unstable-options update --breaking
    cargo update
    for dir in */; do (cd "$dir" && cargo +nightly -Z unstable-options update --breaking); done
    for dir in */; do (cd "$dir" && cargo update); done
    for dir in src/crates/*/; do if [ -f "$dir/Cargo.toml" ]; then (cd "$dir" && cargo +nightly -Z unstable-options update --breaking); fi; done
    for dir in src/crates/*/; do if [ -f "$dir/Cargo.toml" ]; then (cd "$dir" && cargo update); fi; done

fix:
    cargo +nightly fmt
    cargo clippy --fix --allow-dirty --allow-staged
    cargo +nightly fmt
    cargo fmt

fixn:
    cargo +nightly fmt
    cargo +nightly clippy --fix --allow-dirty --allow-staged
    cargo +nightly fmt
    cargo fmt

fixa:
    for dir in src/crates/*/; do if [ "$dir" = "src/crates/fuzz/" ] || [ "$dir" = "src/crates/./" ]; then continue; fi; if [ -f "$dir/Cargo.toml" ]; then (cd "$dir" && cargo +nightly fmt && cargo clippy --fix --allow-dirty --allow-staged); fi; done

build_all:
    cargo build --workspace;
    for dir in src/crates/*/; do if [ "$dir" = "src/crates/fuzz/" ] || [ "$dir" = "src/crates/./" ]; then continue; fi; if [ -f "$dir/Cargo.toml" ]; then (cd "$dir" && cargo build); fi; done
