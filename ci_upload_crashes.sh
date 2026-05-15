#!/bin/bash
# Merge local crashes with remote, dedup, upload with race-condition retry.
# Usage: ci_upload_crashes.sh <project_name> <local_crashes_dir> [max_retries]
set -u

PROJECT="$1"
LOCAL_DIR="$2"
MAX_RETRIES="${3:-10}"
ARCHIVE="crashes_${PROJECT}.7z"

LOCAL_COUNT=$(find "$LOCAL_DIR" -type f 2>/dev/null | wc -l)
if [ "$LOCAL_COUNT" -eq 0 ]; then
    echo "No local crashes to upload."
    # Clean up empty remote archive if exists
    gh release delete-asset Nightly "$ARCHIVE" 2>/dev/null || true
    echo "CRASH_COUNT=0"
    exit 0
fi

for ATTEMPT in $(seq 1 "$MAX_RETRIES"); do
    echo ""
    echo "=== Upload attempt $ATTEMPT/$MAX_RETRIES ==="

    # Step 1: Download current remote archive + checksum
    rm -rf /tmp/upload_work
    mkdir -p /tmp/upload_work

    gh release download Nightly \
        --pattern "$ARCHIVE" \
        --dir /tmp/upload_work \
        --clobber 2>/dev/null

    CHECKSUM_BEFORE="none"
    if [ -f "/tmp/upload_work/$ARCHIVE" ]; then
        CHECKSUM_BEFORE=$(md5sum "/tmp/upload_work/$ARCHIVE" | awk '{print $1}')

        # Extract and merge remote into local
        mkdir -p /tmp/upload_work/remote
        cd /tmp/upload_work && 7z x "$ARCHIVE" -oremote 2>/dev/null || true
        cd "$OLDPWD"
        find /tmp/upload_work/remote -type f -exec cp -n {} "$LOCAL_DIR/" \; 2>/dev/null || true
    fi

    echo "Remote checksum: $CHECKSUM_BEFORE"

    # Step 2: Dedup
    python3 "$(dirname "$0")/dedup_files.py" "$LOCAL_DIR"

    MERGED_COUNT=$(find "$LOCAL_DIR" -type f | wc -l)
    echo "Files to upload: $MERGED_COUNT"

    # Step 3: Pack
    rm -f "/tmp/upload_work/$ARCHIVE"
    cd "$LOCAL_DIR"
    7z a "/tmp/upload_work/$ARCHIVE" . >/dev/null
    cd "$OLDPWD"

    # Step 4: Re-check remote didn't change
    rm -rf /tmp/upload_recheck
    mkdir -p /tmp/upload_recheck

    gh release download Nightly \
        --pattern "$ARCHIVE" \
        --dir /tmp/upload_recheck \
        --clobber 2>/dev/null

    CHECKSUM_NOW="none"
    if [ -f "/tmp/upload_recheck/$ARCHIVE" ]; then
        CHECKSUM_NOW=$(md5sum "/tmp/upload_recheck/$ARCHIVE" | awk '{print $1}')
    fi

    echo "Remote checksum now: $CHECKSUM_NOW"

    if [ "$CHECKSUM_BEFORE" = "$CHECKSUM_NOW" ]; then
        # No race condition — safe to upload
        gh release upload Nightly "/tmp/upload_work/$ARCHIVE" --clobber
        echo "✓ Uploaded $MERGED_COUNT crash files (attempt $ATTEMPT)"
        echo "CRASH_COUNT=$MERGED_COUNT"
        exit 0
    else
        echo "⚠️  Remote changed during merge! Retrying..."
        # Merge the new remote data into local for next attempt
        mkdir -p /tmp/upload_recheck/remote
        cd /tmp/upload_recheck && 7z x "$ARCHIVE" -oremote 2>/dev/null || true
        cd "$OLDPWD"
        find /tmp/upload_recheck/remote -type f -exec cp -n {} "$LOCAL_DIR/" \; 2>/dev/null || true
    fi
done

echo "ERROR: Failed to upload after $MAX_RETRIES attempts (persistent race condition)"
echo "Forcing upload..."
gh release upload Nightly "/tmp/upload_work/$ARCHIVE" --clobber
FINAL_COUNT=$(find "$LOCAL_DIR" -type f | wc -l)
echo "CRASH_COUNT=$FINAL_COUNT"
