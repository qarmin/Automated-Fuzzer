#!/usr/bin/env python3
"""Update and/or check compilation of all crates + fuzz targets.
Usage:
    python3 check_crates.py              # check only
    python3 check_crates.py --upgrade    # update deps then check
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
CRATES_DIR = os.path.join(ROOT, "crates")
FUZZ_DIR = os.path.join(ROOT, "fuzz")


def run(cmd, cwd, timeout=600):
    """Run a command, return (success, output)."""
    try:
        r = subprocess.run(
            cmd, cwd=cwd, capture_output=True, text=True,
            timeout=timeout, shell=isinstance(cmd, str),
        )
        return r.returncode == 0, r.stdout + r.stderr
    except subprocess.TimeoutExpired:
        return False, f"TIMEOUT after {timeout}s"
    except Exception as e:
        return False, str(e)


def get_fuzz_features():
    """Parse fuzz/Cargo.toml to get list of feature names (ending with _f)."""
    cargo_path = os.path.join(FUZZ_DIR, "Cargo.toml")
    if not os.path.isfile(cargo_path):
        return []
    with open(cargo_path) as f:
        content = f.read()
    # Match lines like: lofty_f = ["lofty"]
    return re.findall(r'^(\w+_f)\s*=', content, re.MULTILINE)


def main():
    upgrade = "--upgrade" in sys.argv

    crates = sorted(
        d for d in os.listdir(CRATES_DIR)
        if os.path.isfile(os.path.join(CRATES_DIR, d, "Cargo.toml"))
    )

    if not crates:
        print("No crates found.")
        return

    # Optionally update
    if upgrade:
        print("=== Updating main project ===")
        run("cargo +nightly -Z unstable-options update --breaking", ROOT)
        run("cargo update", ROOT)
        if os.path.isdir(FUZZ_DIR):
            run("cargo update", FUZZ_DIR)
        print()

    #  Check crates 
    total = 0
    passed = []
    failed = {}  # name -> log

    print(f"=== {'Updating and checking' if upgrade else 'Checking'} {len(crates)} crates ===")

    for name in crates:
        crate_dir = os.path.join(CRATES_DIR, name)
        sys.stdout.write(f"  {name} ... ")
        sys.stdout.flush()
        total += 1

        log = ""
        if upgrade:
            _, out = run("cargo +nightly -Z unstable-options update --breaking", crate_dir)
            log += out
            _, out = run("cargo update", crate_dir)
            log += out

        ok, out = run("cargo check", crate_dir)
        log += out

        if ok:
            print("OK")
            passed.append(name)
        else:
            print("FAILED")
            failed[name] = log

    #  Check fuzz targets 
    fuzz_features = get_fuzz_features()
    if fuzz_features and os.path.isdir(FUZZ_DIR):
        if upgrade:
            sys.stdout.write("\n=== Updating fuzz deps === ")
            sys.stdout.flush()
            run("cargo update", FUZZ_DIR)
            print("done")

        sorted_features = sorted(fuzz_features)
        print(f"\n=== Checking {len(sorted_features)} fuzz targets ===")

        # First try all at once with all features
        all_feats = ",".join(sorted_features)
        sys.stdout.write(f"  all-at-once ({len(sorted_features)} targets) ... ")
        sys.stdout.flush()
        ok, out = run(
            f"cargo +nightly check --features {all_feats}",
            FUZZ_DIR,
            timeout=900,
        )
        if ok:
            print("OK")
            for feat in sorted_features:
                target = feat.removesuffix("_f")
                passed.append(f"fuzz/{target}")
            total += len(sorted_features)
        else:
            print("FAILED (checking individually)")
            # Fallback: check each target separately to find which ones fail
            for feat in sorted_features:
                target = feat.removesuffix("_f")
                sys.stdout.write(f"  fuzz/{target} ... ")
                sys.stdout.flush()
                total += 1

                ok, out2 = run(
                    f"cargo +nightly check --bin {target} --features {feat}",
                    FUZZ_DIR,
                )

                label = f"fuzz/{target}"
                if ok:
                    print("OK")
                    passed.append(label)
                else:
                    print("FAILED")
                    failed[label] = out2

    #  Check main project 
    print("\n=== Checking main project ===")
    sys.stdout.write("  auto_fuzzer ... ")
    sys.stdout.flush()
    total += 1
    ok, out = run("cargo check", ROOT)
    if ok:
        print("OK")
        passed.append("auto_fuzzer")
    else:
        print("FAILED")
        failed["auto_fuzzer"] = out

    #  Summary 
    print(f"\n=== Results ===")
    print(f"Passed: {len(passed)}/{total}")

    if failed:
        print(f"Failed: {', '.join(failed.keys())}")
        print()
        for name, log in failed.items():
            print(f"--- {name} ---")
            lines = log.strip().splitlines()
            for line in lines[-30:]:
                print(f"  {line}")
            print()
        sys.exit(1)
    else:
        print("All crates and fuzz targets compile!")


if __name__ == "__main__":
    main()
