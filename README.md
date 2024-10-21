## Automated Fuzzer

This repo contains simple tool to create broken files and checking them with special apps(biome, ruff, mypy and many
more already are implemented, but it is easy to add support for any different app).

This small tool I created mainly for my own use without much vision, so you can easily compile app without changing
source code
if you want to use already implemented fuzzers, but if you want to test your own app

This tool is designed for fast iterations, so it works really great if your app can test/lint several files per second(
e.g. ruff on my pc can test even 50 middle size files per second in one core). If you use slower tool(I had this problem
mypy), you may want to manually generate broken files
via [create_broken_files](https://crates.io/crates/create_broken_files) and test this files in chunks manually.

## How to use it?

- Install tool to create broken files and minimizer(rust and cargo can be installed directly from https://rustup.rs/ via simple
  command)

```
cargo install create_broken_files minimizer
```

- Create file inside `apps` folder and customize class name
- Customize run command, broken messages or created files
- Add to MODE enum your app and point at new file in `main.rs` in match statement
- Create setting inside `fuzz_settings.toml`
- Create required folders used inside `fuzz_settings.toml`
- Find "valid" files - you can find a lot of files in github by cloning big repos and checking its
  files - https://github.com/search?q=stars%3A%3E50++language%3ARust+size%3A%3E1000&type=repositories
- Run app via `cargo run --release`

## How this works

- At start app take n valid files from folder
- Depending on settings invalid files are created
- In loop, different app(`ruff`, `biome`, `mypy` etc.) check this file
- Basing on output messages like `RUST_BACKTRACE`, `crashed`, `error`, `internal bug`, file is checked if caused some
  bugs(this allow to find not only crashes).
- If it found it, then this file is copied to special folder
- If minimization is enabled, app tries to minimize output to produce bug(this may take some time, but output files are
  usually smaller 2x-100x times)

Video, how output should look:

https://user-images.githubusercontent.com/41945903/227783281-a73112ee-b564-41f3-9d6a-f63b294abbce.mp4

## How it is different from other fuzzers?

Compared to cargo fuzz:

- runs applications through the CLI, rather than using its API
- automatically minimizes output files(if of course you have checked this in the settings) - cargo fuzz requires running a separate tool
- automatic running of tested application on multiple threads
- not using advanced input modification techniques
- worse performance, due to overhead associated with running applications via CLI
- automatic generation of a report that can be uploaded to github as an issue
- possibility to collect multiple results at one time, cargo fuzz aborts after the first error found

## So when to use it?
- you have a tool that can be run from the command line (if you have a library, you can create a simple CLI wrapper)
- tool uses file content as input, without needing to setup a complex environment

I is very useful, especially when starting fuzzing a new project.  
I recommend to use two tools at the same time - cargo fuzz and this tool to get best results.  
Automated fuzzer is good to find and group a lot of simpler bugs, while cargo fuzz is good to find more complex bugs one by one.

If you are using rust applications, remember to compile them with release flag, debug symbols enabled, overflow checks and address sanitizer support(you can find in github ci how to do it).

## Modes

Currently, app only search for specific messages in output or checks for specific exit codes.

I plan to add also mode to compare stability of output, sorted output and file content after 2 or more iterations.

## It really works?

Yes, it found thousands of crashes in several projects(some are checked daily in CI):

- Selene - https://github.com/Kampfkarren/selene/issues/505 (1375 files)
- Rome - https://github.com/rome/tools/issues/4323 (>2000 files)
- Ruff - https://github.com/charliermarsh/ruff/issues/3721 (>2000 files)
- Symphonia - https://github.com/pdeljanov/Symphonia/issues/201, https://github.com/pdeljanov/Symphonia/issues/200 (30 files)
- Lofty - https://github.com/Serial-ATA/lofty-rs/issues/174 - (1 file)
- Deno lint - https://github.com/denoland/deno_lint/issues/1145 - (873 files)
- Oxc - https://github.com/Boshen/oxc/issues/232 - (>300 files)
- Static Check Go Tools - https://github.com/dominikh/go-tools/issues/1393 - (10 files)
- Quick Lint js - https://github.com/quick-lint/quick-lint-js/issues/974 - (81 files)

it found a lot of more bugs, but I'm lazy to add them all here.