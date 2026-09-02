#!/usr/bin/env bash
# Extract an immutable baseline from git by explicit rev+path and verify it
# is non-empty before any use. Usage: fetch_golden.sh <rev> <repo-path> <dst>
set -u
rev=${1:?}; path=${2:?}; dst=${3:?}
git show "$rev:$path" > "$dst" || exit 1
sz=$(wc -c < "$dst")
if [ "$sz" -le 0 ]; then echo "fetch_golden: empty $dst (bad rev/path? repo root prefix!)"; rm -f "$dst"; exit 1; fi
echo "$dst: $sz bytes md5=$(md5sum "$dst" | awk '{print $1}')"
