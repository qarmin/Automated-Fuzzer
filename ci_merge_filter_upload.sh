#!/bin/bash
# Merge local crashes with the remote archive, filter via the built-in
# remove_non_crashing_files mechanism (which honors search_items / ignored_items
# from the project's fuzz_settings.toml), and upload with race-condition retry.
# Reports produced as a side effect of the filter are packaged separately.
#
# Loop semantics (matches the user-described flow):
#   1. download remote crashes archive + record md5
#   2. cp -n the remote files into BROKEN_FILES_DIR (merge without overwriting)
#   3. dedup by content hash
#   4. run `auto_fuzzer legacy --remove-non-crashing`
#      → drops files that do not reproduce a *real* crash per the TOML config
#      → writes per-signature reports into temp_folder (/tmp/tmp_folder/data)
#   5. pack BROKEN_FILES_DIR into a fresh archive
#   6. re-download remote, recompute md5
#   7. if md5 unchanged: upload, also upload reports archive, exit
#      else: clear merge state by re-syncing on next iter; loop
#
# Usage: ci_merge_filter_upload.sh <project_name> <broken_files_dir> <reports_dir> [max_retries]
set -u

PROJECT="$1"
BROKEN_DIR="$2"
REPORTS_DIR="$3"
MAX_RETRIES="${4:-10}"
SIZE_LIMIT_MB="${5:-500}"   # max total size of crash files per project, in MB
CRASHES_ARCHIVE="crashes_${PROJECT}.7z"
REPORTS_ARCHIVE="reports_${PROJECT}.7z"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

mkdir -p "$BROKEN_DIR" "$REPORTS_DIR"

run_filter() {
    # Run the built-in filter. It reads fuzz_settings.toml from cwd and decides
    # what is a real crash via search_items minus ignored_items in the config.
    # Any file in BROKEN_DIR that does NOT reproduce gets deleted; reports for
    # the ones that do are written under temp_folder.
    auto_fuzzer legacy --remove-non-crashing || true
}

# Trim crash files to stay within SIZE_LIMIT_MB.
# Files are grouped by extension (no-extension files form their own group).
# Within each group we remove the largest files first, but we never remove
# the sole remaining representative of a group — so every unique crash
# category keeps at least one example file.
trim_crashes_to_limit() {
    local dir="$1"
    local limit_bytes=$(( SIZE_LIMIT_MB * 1024 * 1024 ))

    local total_size
    total_size=$(du -sb "$dir" 2>/dev/null | awk '{print $1}')

    if [ "$total_size" -le "$limit_bytes" ]; then
        echo "Crash dir size: $((total_size / 1024 / 1024)) MB — within ${SIZE_LIMIT_MB} MB limit, no trimming needed."
        return
    fi

    echo "Crash dir size: $((total_size / 1024 / 1024)) MB — trimming to ~${SIZE_LIMIT_MB} MB (removing largest from over-represented groups)..."

    local removed=0
    while [ "$total_size" -gt "$limit_bytes" ]; do
        # Build size-sorted list (largest first)
        find "$dir" -maxdepth 1 -type f -printf '%s %p\n' | sort -rn > /tmp/_trim_list.txt

        local candidate=""
        while read -r _size fpath; do
            local fname
            fname=$(basename "$fpath")
            local count
            if [[ "$fname" == *.* ]]; then
                local ext="${fname##*.}"
                count=$(find "$dir" -maxdepth 1 -type f -name "*.$ext" | wc -l)
            else
                count=$(find "$dir" -maxdepth 1 -type f ! -name "*.*" | wc -l)
            fi
            if [ "$count" -gt 1 ]; then
                candidate="$fpath"
                break
            fi
        done < /tmp/_trim_list.txt
        rm -f /tmp/_trim_list.txt

        if [ -z "$candidate" ]; then
            echo "Cannot trim further — every remaining category has only 1 file."
            break
        fi

        rm -f "$candidate"
        removed=$(( removed + 1 ))
        total_size=$(du -sb "$dir" 2>/dev/null | awk '{print $1}')
    done

    echo "Trimmed $removed crash file(s). Final size: $((total_size / 1024 / 1024)) MB ($(find "$dir" -maxdepth 1 -type f | wc -l) files remaining)."
}

for ATTEMPT in $(seq 1 "$MAX_RETRIES"); do
    echo ""
    echo "=== Sync attempt $ATTEMPT/$MAX_RETRIES ==="

    # 1. Download remote + checksum
    rm -rf /tmp/sync_work
    mkdir -p /tmp/sync_work/remote_extract

    gh release download Nightly \
        --pattern "$CRASHES_ARCHIVE" \
        --dir /tmp/sync_work \
        --clobber 2>/dev/null || true

    CHECKSUM_BEFORE="none"
    if [ -f "/tmp/sync_work/$CRASHES_ARCHIVE" ]; then
        CHECKSUM_BEFORE=$(md5sum "/tmp/sync_work/$CRASHES_ARCHIVE" | awk '{print $1}')
        7z x "/tmp/sync_work/$CRASHES_ARCHIVE" -o/tmp/sync_work/remote_extract >/dev/null 2>&1 || true
        REMOTE_COUNT=$(find /tmp/sync_work/remote_extract -type f 2>/dev/null | wc -l)
        echo "Remote archive: $REMOTE_COUNT files, md5=$CHECKSUM_BEFORE"
    else
        echo "Remote archive: not present (first upload)"
    fi

    # 2. Merge remote into local — cp -n preserves anything we already filtered out
    if [ -d /tmp/sync_work/remote_extract ]; then
        find /tmp/sync_work/remote_extract -type f -exec cp -n {} "$BROKEN_DIR/" \; 2>/dev/null || true
    fi
    BEFORE_DEDUP=$(find "$BROKEN_DIR" -type f | wc -l)

    # 3. Content dedup
    python3 "$SCRIPT_DIR/dedup_files.py" "$BROKEN_DIR" || true
    AFTER_DEDUP=$(find "$BROKEN_DIR" -type f | wc -l)
    echo "After merge + dedup: $AFTER_DEDUP files (was $BEFORE_DEDUP before dedup)"

    # 4. Filter via built-in mechanism (uses config search/ignored items)
    BEFORE_FILTER=$AFTER_DEDUP
    run_filter
    AFTER_FILTER=$(find "$BROKEN_DIR" -type f | wc -l)
    REPORT_COUNT=$(find "$REPORTS_DIR" -type f 2>/dev/null | wc -l)
    echo "After filter: $AFTER_FILTER files (filter removed $((BEFORE_FILTER - AFTER_FILTER))), $REPORT_COUNT report files"

    # 4b. Enforce size limit — remove the largest files from over-represented
    #     extension groups so total stays within SIZE_LIMIT_MB.
    if [ "$AFTER_FILTER" -gt 0 ]; then
        trim_crashes_to_limit "$BROKEN_DIR"
        AFTER_FILTER=$(find "$BROKEN_DIR" -type f | wc -l)
    fi

    # 5. Pack our filtered set
    rm -f "/tmp/sync_work/$CRASHES_ARCHIVE"
    if [ "$AFTER_FILTER" -gt 0 ]; then
        cd "$BROKEN_DIR"
        7z a "/tmp/sync_work/$CRASHES_ARCHIVE" . >/dev/null
        cd "$OLDPWD"
    fi

    # 6. Re-check remote — did anyone else upload while we worked?
    rm -rf /tmp/sync_recheck
    mkdir -p /tmp/sync_recheck

    gh release download Nightly \
        --pattern "$CRASHES_ARCHIVE" \
        --dir /tmp/sync_recheck \
        --clobber 2>/dev/null || true

    CHECKSUM_NOW="none"
    if [ -f "/tmp/sync_recheck/$CRASHES_ARCHIVE" ]; then
        CHECKSUM_NOW=$(md5sum "/tmp/sync_recheck/$CRASHES_ARCHIVE" | awk '{print $1}')
    fi
    echo "Remote checksum now: $CHECKSUM_NOW (was $CHECKSUM_BEFORE)"

    if [ "$CHECKSUM_BEFORE" = "$CHECKSUM_NOW" ]; then
        # 7a. Stable — upload crashes (or delete archive if empty)
        if [ "$AFTER_FILTER" -gt 0 ]; then
            gh release upload Nightly "/tmp/sync_work/$CRASHES_ARCHIVE" --clobber
            echo "✓ Uploaded $AFTER_FILTER crash files (attempt $ATTEMPT)"
        else
            gh release delete-asset Nightly "$CRASHES_ARCHIVE" -y 2>/dev/null || true
            echo "✓ No crashes survived filter — deleted remote archive"
        fi

        # Reports archive (separate, also cached externally)
        rm -f "/tmp/sync_work/$REPORTS_ARCHIVE"
        if [ "$REPORT_COUNT" -gt 0 ]; then
            cd "$REPORTS_DIR"
            7z a "/tmp/sync_work/$REPORTS_ARCHIVE" . >/dev/null
            cd "$OLDPWD"
            gh release upload Nightly "/tmp/sync_work/$REPORTS_ARCHIVE" --clobber
            echo "✓ Uploaded $REPORT_COUNT report files"
        else
            gh release delete-asset Nightly "$REPORTS_ARCHIVE" -y 2>/dev/null || true
            echo "✓ No reports to upload — deleted remote reports archive"
        fi

        echo "CRASH_COUNT=$AFTER_FILTER"
        echo "REPORT_COUNT=$REPORT_COUNT"
        exit 0
    fi

    echo "⚠️  Remote changed during sync — looping"
done

echo "ERROR: Failed to converge after $MAX_RETRIES attempts (persistent race)."
echo "Forcing final upload."
if [ "$AFTER_FILTER" -gt 0 ] && [ -f "/tmp/sync_work/$CRASHES_ARCHIVE" ]; then
    gh release upload Nightly "/tmp/sync_work/$CRASHES_ARCHIVE" --clobber || true
fi
if [ "$REPORT_COUNT" -gt 0 ]; then
    cd "$REPORTS_DIR" && 7z a "/tmp/sync_work/$REPORTS_ARCHIVE" . >/dev/null && cd "$OLDPWD"
    gh release upload Nightly "/tmp/sync_work/$REPORTS_ARCHIVE" --clobber || true
fi
echo "CRASH_COUNT=$AFTER_FILTER"
echo "REPORT_COUNT=$REPORT_COUNT"
exit 1
