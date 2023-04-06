## Automated Fuzzer

This repo contains simple tool to create broken files and checking them with special apps(rome, ruff, mypy and more already are implemented, but it is easy to add support for any different app).

This small tool I created mainly for my own use without much vision, and cannot be used directly via `cargo install --git`, but you need to modify source files to be able to run it(this is not too hard)

## How to use it?
- Install tool to create broken files
```
cargo install create_broken_files
```
- Create file inside `apps` folder and customize class name
- Customize run command, broken messages or created files
- Add to MODE enum your app and point at new file in `main.rs` in match statement
- Create setting inside `fuzz_settings.toml`
- Create required folders used inside `fuzz_settings.toml`
- Run app via `cargo run --release`

## How this works
- At start app take n valid files from folder
- Depending on settings invalid files are created
- In loop, different app(`ruff`, `rome`, `mypy` etc.) check this file
- Basing on output messages like `RUST_BACKTRACE`, `crashed`, `error`, `internal bug`, file is checked if caused some bugs(this will find not only crashes).
- If yes, then this file is copied to special folder to be able to verify it
- If minimization is enabled, app tries to minimize output to produce bug(this may take some time, but output files are usually smaller 2x-100x times)

Video, how output should look:  

https://user-images.githubusercontent.com/41945903/227783281-a73112ee-b564-41f3-9d6a-f63b294abbce.mp4
