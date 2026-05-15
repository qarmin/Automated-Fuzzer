#!/usr/bin/env python3
"""Remove duplicate files by MD5 hash from a directory (recursively).
Usage: python3 dedup_files.py /path/to/dir [--dry-run]
"""
import hashlib
import os
import sys

def md5(path):
    h = hashlib.md5()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()

def dedup(directory, dry_run=False):
    seen = {}  # md5 -> first path
    removed = 0
    kept = 0
    for root, _dirs, files in os.walk(directory):
        for name in sorted(files):
            path = os.path.join(root, name)
            if not os.path.isfile(path):
                continue
            h = md5(path)
            if h in seen:
                if dry_run:
                    print(f"[DUP] {path}  (same as {seen[h]})")
                else:
                    os.remove(path)
                    print(f"[REMOVED] {path}  (dup of {seen[h]})")
                removed += 1
            else:
                seen[h] = path
                kept += 1
    print(f"\nKept {kept}, removed {removed} duplicates.")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <directory> [--dry-run]")
        sys.exit(1)
    directory = sys.argv[1]
    dry_run = "--dry-run" in sys.argv
    if not os.path.isdir(directory):
        print(f"Not a directory: {directory}")
        sys.exit(1)
    dedup(directory, dry_run)
