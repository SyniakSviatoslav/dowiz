#!/usr/bin/env bash
# Mechanical preflight against the silent-artifact class (empty/stale .bin,
# journal 1788288248). Usage: guard_artifact.sh <file> [expected_md5]
set -u
f=${1:?usage: guard_artifact.sh <file> [md5]}
if [ ! -f "$f" ]; then echo "GUARD: missing artifact $f"; exit 1; fi
sz=$(wc -c < "$f" 2>/dev/null || echo 0)
if [ "$sz" -le 0 ]; then echo "GUARD: empty artifact $f (zero bytes)"; exit 1; fi
if [ $# -ge 2 ]; then
  got=$(md5sum "$f" | awk '{print $1}')
  if [ "$got" != "$2" ]; then echo "GUARD: checksum mismatch $f: got $got want $2"; exit 1; fi
fi
exit 0
