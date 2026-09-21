#!/bin/sh
# F26 — THE SQL RATCHET.
#
# The operator's directive is "no SQL: bebop, Rust, wasm and nothing else", and
# 118 prepared statements do not leave the tree in one commit. So this counts
# them and refuses any commit that ADDS one. The number may only fall, which is
# the same mechanism `bebop-lang/tools/arch_check.py` uses for file size and the
# same reason: a target nobody can reach in one step still has to be monotone,
# or it is a wish.
#
# WHAT IT COUNTS, and why it is `.prepare(` rather than the word SELECT: the
# D1 API has exactly one entry point, so a statement that exists has been
# prepared. Counting SQL keywords would match every comment that explains why a
# query was deleted, which is the opposite of what this gate wants to encourage.
#
# When the count reaches zero, the LAST commit removes `[[d1_databases]]` from
# wrangler.toml, deletes workers/api/migrations/, and this gate becomes an
# assertion that the count is 0 rather than a ratchet.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/no-sql.baseline
n=$(grep -o "\.prepare(" workers/api/src/*.rs | wc -l | tr -d ' ')
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-sql: baseline recorded at $n prepared statements"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
echo "no-sql: $n prepared statements (ratchet at $baseline)"
if [ "$n" -gt "$baseline" ]; then
  echo "no-sql: FAILED — $((n - baseline)) statement(s) ADDED. The ratchet only goes down."
  echo "Per file now:"
  for f in workers/api/src/*.rs; do
    c=$(grep -c "\.prepare(" "$f" || true)
    [ "$c" -gt 0 ] && echo "  $c $(basename "$f")"
  done | sort -rn
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-sql: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
[ "$n" -eq 0 ] && echo "no-sql: ZERO. Remove the D1 binding and the migrations directory."
exit 0
