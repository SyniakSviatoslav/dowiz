#!/bin/sh
# G1 — THE RED LINE, COUNTED.
#
# `crates/dowiz-core/src/money.rs:1` says "Integer money — RED LINE: zero float
# arithmetic on monetary values", `CLAUDE.md:98` says "zero floats, ever", and
# `MANIFESTO.md:18` C5 says "Integer-only money (`i64` minor units, no
# `From<f64>`)". Three documents, no instrument. This is the instrument.
#
# WHAT IT DEFENDS, and it is not an abstraction. `MANIFESTO` C2 requires the
# decision path to replay identically on every node, offline; IEEE-754 results
# vary by compiler, by FPU mode and by target, which is the same reason
# deterministic-lockstep games ban floats. A tax rate is also a DECIMAL
# fraction: it belongs in an integer decimal scale (parts per million,
# `RatePpm`), not in a binary one. Measured, and this is the whole argument in
# one number: 20 % of 100 000 000 lek in Q16 binary fixed point is 305 lek
# short.
#
# WHAT IT COUNTS. Lines mentioning `f64` at all — a parameter, a return type, a
# generic argument, a cast, an `as_f64` accessor — in the files that decide what
# a customer is charged:
#
#     crates/dowiz-core/src/money.rs      the law
#     crates/dowiz-core/src/tax.rs        the rate type and the summariser
#     crates/dowiz-core/src/domain.rs     the aggregate that calls the law
#     workers/api/src/services/ordering/  the pricer, the fees, the promos
#     workers/api/src/command/            where the order is decided and appended
#
# It counts the WHOLE LINE and not a token class, and that is deliberate.
# `BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md` §5 proposed the class
# `as f64|f64::from|: f64|_f64(`; measured against the tree, that class misses
# `-> f64` (`domain.rs:387,392`), `Option<f64>` (`domain.rs:403,427`) and
# `.and_then(Value::as_f64)` (`rates.rs:82`) — five of the eleven real sites,
# including the only one on the Worker's own path. A gate that cannot see the
# float in the pricer is one of the five instruments that measured nothing.
#
# COMMENTS ARE STRIPPED FIRST, which `clock.sh:60-66` learned the hard way: a
# gate that counts the note explaining a float teaches people to delete the
# note. It matters here immediately — `domain.rs:53,55` are two comment lines
# whose content is "the optional geocode is INTEGER micro-degrees, not `f64`".
# Counting them would have punished the comment that says the red line was
# HELD.
#
# TESTS DO NOT COUNT: every `#[cfg(test)]` region is removed and `tests.rs` is
# skipped whole. A test that pins the f64 adapter's behaviour is the thing that
# makes deleting the adapter safe.
#
# THE REGIONS ARE REMOVED BY BRACE MATCHING, NOT BY CUTTING AT THE FIRST ONE,
# and this gate was RED-proved wrong once before it was believed. The first
# version did `awk '/^#\[cfg\(test\)\]/{exit}'`; a float appended BELOW
# `money.rs`'s test module was then invisible and the gate returned 0 with the
# float sitting in the file. That is `unreached.py`'s documented 11 -> 6 bug
# (`hubdo.rs` has a mid-file test module) reproduced in a second instrument.
# `#[cfg(test)] mod tests;` — a declaration, no braces — ends at its semicolon,
# which is how `ordering/mod.rs:13` and `command/mod.rs:43` keep the code below
# them counted.
#
# LINE NUMBERS ARE THE REAL ONES. The filter carries `NR` through, so the
# refusal names `domain.rs:379` and not "line 379 of what is left after the
# tests were deleted" — a number nobody can open.
#
# THE RATCHET MAY ONLY FALL. Same mechanism as `file-size.sh` and `clock.sh`.
# The target is 0 and the route is written down (blueprint §6 items 1, 8 and
# 4-against-8): the `tax_rate: f64` parameters go when their last callers do,
# `convert_all_to_eur_cents(_, f64)` takes a ppm argument, `rates.rs:82` reads
# the integer ppm `rates.rs` already ships, and `domain.rs`'s Kalman-filter
# `f64`s (`:379-427`) are not money and leave with the aggregate work.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/float-money.baseline

FILES="crates/dowiz-core/src/money.rs crates/dowiz-core/src/tax.rs crates/dowiz-core/src/domain.rs"

hits() {
  for f in $FILES $(find workers/api/src/services/ordering workers/api/src/command \
                      -name '*.rs' ! -name 'tests.rs' | sort); do
    [ -f "$f" ] || continue
    # Remove every `#[cfg(test)]` region by brace matching, keep real line
    # numbers, strip comments, then count.
    awk '
      /^[[:space:]]*#\[cfg\(test\)\]/ { if (!skip) { skip=1; started=0; depth=0; next } }
      {
        if (skip) {
          if (!started) {
            ob = index($0, "{"); sc = index($0, ";")
            if (ob > 0 && (sc == 0 || ob < sc)) { started = 1 }
            else if (sc > 0) { skip = 0; next }   # `#[cfg(test)] mod tests;`
            else { next }                          # attribute continuation
          }
          line = $0
          n = gsub(/\{/, "{", line); m = gsub(/\}/, "}", line)
          depth += n - m
          if (depth <= 0) { skip = 0; started = 0 }
          next
        }
        printf "%d:%s\n", NR, $0
      }' "$f" \
      | sed 's,//.*,,' \
      | grep -E 'f64' \
      | sed "s|^|$f:|"
  done
}

n=$(hits | wc -l | tr -d ' ')
echo "float-money: $n float(s) on the money path"

if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "float-money: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")

if [ "$n" -gt "$baseline" ]; then
  echo "float-money: REFUSED — $((n - baseline)) float(s) ADDED to the money path (baseline $baseline)."
  echo "Money is exact integer arithmetic. A rate is a RatePpm. The sites now:"
  hits | sed 's|^|  |'
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "float-money: the ratchet has fallen $baseline -> $n. Lower $BASELINE_FILE in this commit."
fi
echo "float-money: $n (baseline $baseline)"
[ "$n" -eq 0 ] && echo "float-money: ZERO. The red line in money.rs:1 is now a fact, not a header."
exit 0
