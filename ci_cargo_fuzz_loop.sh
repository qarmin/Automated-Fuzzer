#!/bin/bash
# Run cargo-fuzz in a loop until total timeout expires.
# After each crash, restart with remaining time.
# Usage: ci_cargo_fuzz_loop.sh <target> <features> <corpus_dir> <total_timeout> <jobs>
set -u

TARGET="$1"
FEATURES="$2"
CORPUS_DIR="$3"
TOTAL_TIMEOUT="$4"
JOBS="${5:-4}"

export RUST_BACKTRACE=1
export ASAN_SYMBOLIZER_PATH=$(which llvm-symbolizer-18 2>/dev/null || which llvm-symbolizer)
export ASAN_OPTIONS="symbolize=1:allocator_may_return_null=1"
export RUSTFLAGS="-Zsanitizer=address"

START_TIME=$(date +%s)
RUN=0

while true; do
    NOW=$(date +%s)
    ELAPSED=$((NOW - START_TIME))
    REMAINING=$((TOTAL_TIMEOUT - ELAPSED))

    if [ "$REMAINING" -le 10 ]; then
        echo "=== Time expired after $RUN runs ==="
        break
    fi

    RUN=$((RUN + 1))
    echo ""
    echo "=== Run #$RUN, ${REMAINING}s remaining ==="

    cargo fuzz run "$TARGET" "$CORPUS_DIR" \
        "-j${JOBS}" --release --features "$FEATURES" \
        -- -max_len=99999 -max_total_time="$REMAINING" -rss_limit_mb=20000 2>&1

    EXIT_CODE=$?

    # Remove slow-unit files immediately
    find fuzz/artifacts/ -type f -name 'slow*' -exec rm {} + 2>/dev/null || true

    if [ $EXIT_CODE -eq 0 ]; then
        echo "=== Fuzzer exited cleanly (time expired or no more work) ==="
        break
    fi

    CRASH_COUNT=$(find "fuzz/artifacts/$TARGET" -type f 2>/dev/null | wc -l)
    echo "=== Fuzzer found crash (exit $EXIT_CODE), $CRASH_COUNT total artifacts. Restarting... ==="
done

TOTAL_CRASHES=$(find "fuzz/artifacts/$TARGET" -type f 2>/dev/null | wc -l)
echo ""
echo "=== Finished: $RUN runs, $TOTAL_CRASHES crash files ==="
