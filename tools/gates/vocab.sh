#!/bin/sh
# F5 — THE CLIENT VOCABULARY IS GENERATED, AND THIS IS WHAT KEEPS IT THAT WAY.
#
# `tools/gates/vocabulary.sh` is the cheap half of blueprint P4: it refuses a
# tree where the kernel's `OrderStatus` has a member that has no word in
# `admin/i18n.js` and no colour in the three stylesheets. It closed the hole it
# was written for — `REFUNDING` and `COMPENSATED_REFUND` were missing from every
# surface — but it cannot stop the NEXT hand copy, because until now there was
# nothing for a browser surface to copy FROM.
#
# `tools/gen-vocab` is the thing to copy from, and it is not a regex over
# `order_machine.rs`: it is a Rust binary that path-depends on `dowiz-core` and
# CALLS `OrderStatus::as_str`, `is_terminal`, `took_money`, `is_active`,
# `assert_transition` and `Currency::from_code`. Its output is
# `workers/api/public/lib/vocab.js`, which is committed (the edge serves this
# tree directly, exactly as `public/tsconfig.json` explains for the compiled
# TypeScript) and therefore CAN go stale. This gate is the thing that notices.
#
# THREE REFUSALS, in the order they can bite:
#
#   1. THE GENERATOR REFUSED. It cross-checks its own derivation against
#      `FSM_GOLDEN_SIGNATURE` (vertex and edge counts reached two independent
#      ways) and exits 1 on disagreement, printing nothing on stdout. A gate
#      that only diffed would then compare the committed file against an empty
#      one and call the kernel's own drift "the file is stale".
#   2. THE COMMITTED FILE IS STALE. A member added to the FSM, an edge moved, a
#      currency added — and the browser keeps shipping yesterday's list.
#   3. A HAND COPY CAME BACK. The specific fingerprint of the defect P4 closes:
#      a literal set naming both `REJECTED` and `CANCELLED`, which is what the
#      owner console, the storefront tracker, the sea and the kit each held, and
#      which was short by `COMPENSATED_REFUND` in all four. `//` comments go
#      FIRST and newlines are squashed second, so a set written across three
#      lines is still one match while the four comments that RECORD the removal
#      -- each of which quotes the set it replaced -- are not four refusals.
#      That trap is not hypothetical: this gate refused its own commit the first
#      time it was run, for exactly the reason `no-sql.sh` warns about in its
#      header. Coarse on purpose, in the same manner: it is here to make the
#      intent legible and to catch the fifth copy, not to be a JS parser.
#
# AND ONE CROSS-CHECK the kernel cannot make. `Currency` carries a CODE and
# nothing else — there is no minor-unit authority in `crates/dowiz-core` to
# read, so `vocab.js` emits the currency LIST and `lib/money.js` still owns
# `DECIMALS`. The one thing that can be checked is that they are about the same
# currencies, which is how `1500 lek` stops being drawn as `$15.00` the next
# time a currency is added. When the kernel grows `Currency::minor_units`, move
# the table into the generator and delete this paragraph.
#
# COST: this gate COMPILES RUST, which is the price of reading types instead of
# grepping them. Measured here, not guessed: 108 s from a clean target directory
# (dowiz-core is 300-odd modules), then 4.4 s / 4.6 s warm -- and 390 s for a
# third warm run that overlapped another cargo build on the same box, because
# cargo blocks on the package-cache lock. On a CI runner that is one cold build
# per job; on a developer box, run it after the build you were doing anyway.
set -eu
cd "$(dirname "$0")/../.."

COMMITTED=workers/api/public/lib/vocab.js
MONEY=workers/api/public/lib/money.js
GEN=tools/gen-vocab

command -v cargo >/dev/null 2>&1 || {
  echo "vocab: REFUSED — no cargo on PATH, so the generator cannot be run."
  echo "vocab: a gate that cannot execute is not a gate that passed."
  exit 1
}
[ -f "$COMMITTED" ] || { echo "vocab: REFUSED — $COMMITTED does not exist."; exit 1; }

TMP=$(mktemp) || exit 1
ERR=$(mktemp) || exit 1
trap 'rm -f "$TMP" "$ERR"' EXIT

# No pipeline here on purpose: `cargo run | tee` would report tee's status and
# a refusing generator would read as green (the trap this repo has fallen into
# before).
if ! ( cd "$GEN" && cargo run --quiet --offline ) >"$TMP" 2>"$ERR"; then
  echo "vocab: REFUSED — the generator exited non-zero. It says:"
  # Its own sentences, not dowiz-core's 119 build warnings. When it did not get
  # far enough to say anything (a compile error), the tail of the build log is
  # the honest thing to show instead -- never nothing.
  if grep -q '^gen-vocab:' "$ERR"; then
    grep '^gen-vocab:' "$ERR" | sed 's/^/vocab:   /'
  else
    echo "vocab:   (the generator said nothing; the last 20 lines of its stderr:)"
    tail -20 "$ERR" | sed 's/^/vocab:   /'
  fi
  exit 1
fi
[ -s "$TMP" ] || { echo "vocab: REFUSED — the generator produced nothing at all."; exit 1; }

if ! diff -u "$COMMITTED" "$TMP" > "$ERR" 2>&1; then
  echo "vocab: REFUSED — $COMMITTED is stale. The kernel and the browsers disagree:"
  sed 's/^/vocab:   /' "$ERR"
  echo "vocab: regenerate with:  cd $GEN && cargo run > ../../$COMMITTED"
  exit 1
fi

# The currency list, out of the generated file and out of the generated
# decimals table, both as sorted words. Both are now generated, so they must match.
gen_cur=$(sed -n '/^export const CURRENCIES = \[$/,/^\];$/p' "$COMMITTED" \
  | tr -d " '" | tr ',' '\n' | grep -E '^[A-Z]+$' | sort)
dec_cur=$(sed -n '/^export const DECIMALS = {$/,/^};$/p' "$COMMITTED" \
  | tr -d " " | tr ',' '\n' | cut -d: -f1 | grep -E '^[A-Z]+$' | sort)
if [ "$gen_cur" != "$dec_cur" ]; then
  echo "vocab: REFUSED — the kernel's currencies and generated DECIMALS disagree."
  echo "vocab:   CURRENCIES: $(printf '%s ' $gen_cur)"
  echo "vocab:   DECIMALS: $(printf '%s ' $dec_cur)"
  echo "vocab: a currency with no decimals renders through Intl's guess of two,"
  echo "vocab: which is how 1500 lek was drawn as \$15.00."
  exit 1
fi

copies=$(find workers/api/public -name '*.js' ! -name 'vocab.js' \
           ! -path '*/vendor/*' ! -path '*/map/*' -print | sort | while read -r f; do
  sed 's,//.*,,' "$f" | tr '\n' ' ' | grep -o "new Set(\[[^]]*\])" \
    | grep "'REJECTED'" | grep "'CANCELLED'" | sed "s|^|$f: |" || true
done)
if [ -n "$copies" ]; then
  echo "vocab: REFUSED — a hand copy of the kernel's status set is back:"
  printf '%s\n' "$copies" | sed 's/^/vocab:   /'
  echo "vocab: import REFUSED (or TERMINAL/ACTIVE/NEXT) from /lib/vocab.js instead."
  exit 1
fi

n=$(sed -n '/^export const STATUSES = \[$/,/^\];$/p' "$COMMITTED" \
  | grep -o "'[A-Z_]*'" | wc -l | tr -d ' ')
echo "vocab: $COMMITTED is what tools/gen-vocab emits — $n statuses, currencies $(printf '%s ' $gen_cur)| 0 hand copies"
