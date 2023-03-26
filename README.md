## Automated Fuzzer

This repo contains simple tool to create broken files and checking them with special apps(rome, ruff, mypy already are implemented, but it is easy to add support for any different app).

This small tool I created mainly for my own use without much vision, and cannot be used directly via `cargo install --git`, but you need to modify source files to be able to run it(this is not too hard)

## How to use it?
- Install tool to create broken files
```
cargo install create_broken_files
```
- Add to `commons.rs`(already this is done partially for python/javascript) words that will be added to code(best to add some language keywords)
- Create settings(by copying e.g. Ruff config) in `settings.rs` and setup paths to valid folders(this folders must exists) - changing supported apps works by commenting/uncommenting parts of code
- Copy `ruff.rs` and create file with similar content - one function will create command to run and second will validate output to check if crash/error happened
- Add this functions to `main.rs` inside `choose_validate_output_function`, `choose_run_command` and `choose_broken_files_creator`
- Run app via `cargo run --release`
- Depending on CPU load, consider to enable more/less rayon threads

## How this works
- At start app take n valid files from folder
- Depending on settings invalid files are created
- In loop, different app(`ruff`, `rome`, `mypy` etc.) check this file
- Basing on output messages like `RUST_BACKTRACE`, `crashed`, `error`, `internal bug`, file is checked if caused some bugs(this will find not only crashes).
- If yes, then this file is copied to special folder to be able to verify it


Video, how output should look:  

https://user-images.githubusercontent.com/41945903/227783281-a73112ee-b564-41f3-9d6a-f63b294abbce.mp4
