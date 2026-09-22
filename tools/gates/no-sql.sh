#!/bin/sh
# F26 — THE SQL RATCHET, NOW A ZERO ASSERTION.
#
# The operator's directive is "no SQL: bebop, Rust, wasm and nothing else". The
# D1 binding is out of wrangler.toml, workers/api/migrations/ is deleted,
# migrate.rs is deleted, and no handle remains in the code. So the ratchet that
# counted 118 prepared statements down to zero is now an assertion that both
# stay there.
#
# WHAT IT COUNTS, AND WHY COMMENTS ARE STRIPPED FIRST. The ratchet this replaced
# carried a warning in its own header: "Counting SQL keywords would match every
# comment that explains why a query was deleted, which is the opposite of what
# this gate wants to encourage." Its replacement counted `d1(` over raw files
# and immediately failed on
#
#     // NO `ctx.d1("DB")` here: this route held a D1 handle it never used
#
# -- a comment recording a REMOVAL, refused as though it were an addition. A
# gate that punishes the note explaining a deletion teaches people to delete the
# note. So `//` line comments go before anything is counted, and what is left is
# code: `.prepare(` for a statement, `.d1(` for a handle.
#
# The method is the coarse one on purpose: a `//` inside a string literal would
# truncate that line early and could hide a call written after it on the same
# line. That is not a hole worth closing with a Rust parser here -- `cargo
# check` fails the moment a `.d1(` call exists, because the `d1` feature is off
# in Cargo.toml and `RouteContext::d1` no longer resolves. This gate's job is to
# make the intent legible and to catch the binding coming back in a config, not
# to be the only thing standing between the tree and SQL.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/no-sql.baseline
code() { sed 's,//.*,,' $(find workers/api/src -name '*.rs'); }
prepared=$(code | grep -o "\.prepare(" | wc -l | tr -d ' ')
handles=$(code | grep -o "\.d1(" | wc -l | tr -d ' ')
# THE BINDING ITSELF, because the code can be clean while the deployment still
# attaches a database -- which is how it looked for the whole migration.
binding=$(grep -c '^\[\[d1_databases\]\]' workers/api/wrangler.toml || true)
migrations=$(ls workers/api/migrations 2>/dev/null | wc -l | tr -d ' ')
n=$((prepared + handles + binding + migrations))
echo "no-sql: $prepared .prepare( + $handles .d1( + $binding binding(s) + $migrations migration file(s)"
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-sql: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
if [ "$n" -gt "$baseline" ]; then
  echo "no-sql: FAILED — $baseline -> $n. SQL or a D1 binding came back."
  code | grep -n "\.prepare(\|\.d1(" || true
  grep -n '^\[\[d1_databases\]\]' workers/api/wrangler.toml || true
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-sql: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
[ "$n" -eq 0 ] && echo "no-sql: ZERO. No statements, no handles, no binding, no migrations."
exit 0
