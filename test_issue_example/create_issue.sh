#!/bin/bash
# Issue: Test issue - auto_fuzzer create_issue.sh verification
# Repo:  qarmin/Automated-Fuzzer
# Review the issue_body.md before running!

DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Creating issue: Test issue - auto_fuzzer create_issue.sh verification"
echo ""

ISSUE_URL=$(gh issue create \
    --repo "qarmin/Automated-Fuzzer" \
    --title "$(cat "$DIR/issue_title.txt")" \
    --body-file "$DIR/issue_body.md" 2>&1)

echo ""
echo "Issue created:"
echo "$ISSUE_URL"
echo ""
echo "Opening in browser to attach compressed.zip..."
xdg-open "$ISSUE_URL" 2>/dev/null || open "$ISSUE_URL" 2>/dev/null || echo "Open manually: $ISSUE_URL"
echo ""
echo "Attach this file: $DIR/compressed.zip"
