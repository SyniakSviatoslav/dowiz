#!/usr/bin/env bash
# Design-system gate — the mechanically checkable half of
# docs/design/dowiz-interfaces/DOWIZ-INTERFACES-PLAN.md §8.
#
# §8.4 says the nine coherence rules are "mechanically checkable". This is that
# check. It exists for the same reason the zero-dep gate exists: a rule nothing
# enforces is a rule that drifts, and the drift is invisible until three
# surfaces disagree about what a price looks like.
#
# It checks what a script honestly CAN. Rules about hierarchy, air and story are
# judgement and are not pretended at here.
set -uo pipefail
cd "$(dirname "$0")/.."

# ── does the page parse AS A MODULE? ──
#
# Every surface loads its script with `type="module"`, and module scope is
# stricter than script scope in one way that bites: a duplicate top-level
# function declaration is legal in a script and a FATAL parse error in a module.
# `node --check <file>` checks it as a script and says nothing; the page then
# fails to parse and the whole console is blank with one line in a console
# nobody has open.
#
# That is not hypothetical -- a second `loadCouriers` shadowed the first for one
# commit, and the admin pane would have been entirely dead.
mod_fail=0
if command -v node >/dev/null 2>&1; then
  # FOUND, NOT LISTED, and that is the fix rather than a longer list. This was
  # four hard-coded paths plus `lib/*.js`, so `public/store/*.js` was never
  # checked at all -- an entire surface, the customer-facing one. A backtick
  # inside an HTML comment inside a template literal made `store/booking.js`
  # unparseable while this gate stayed GREEN, and the page a customer opens
  # would have been blank. A list that has to be remembered is a list with a
  # hole in it; the hole is always the directory added most recently.
  #
  # Only files that ARE modules: a `.js` with no import and no export is a
  # classic script and parsing it under module rules would fail for a reason
  # that is not a defect.
  for f in $(find workers/api/public -name '*.js' ! -name '*.test.mjs' | sort); do
    [ -f "$f" ] || continue
    grep -qE '^[[:space:]]*(import|export)[[:space:]]' "$f" || continue
    if ! err=$(node --input-type=module --check < "$f" 2>&1); then
      echo "  FAIL  [module] $f does not parse as an ES module"
      echo "$err" | sed -n '1,4p' | sed 's/^/          /'
      mod_fail=1
    fi
  done
else
  echo "  note  node is not on PATH: the module parse check did not run"
fi

# The money module's own tests. It is JavaScript a browser runs, so a Rust
# crate passing says nothing about what a customer sees; this is the only place
# that check can live.
# The booking clock's own tests. A slot is minutes since the epoch and a
# venue's day is the VENUE's, not the phone's -- two rules that were RED first
# and where the implementation was wrong, not the test: clamping a wrapping
# window to midnight cut an hour off every late kitchen's evening.
node workers/api/public/lib/booking-time.test.mjs >/dev/null || {
  echo "booking-time-gate: workers/api/public/lib/booking-time.test.mjs FAILED" >&2
  node workers/api/public/lib/booking-time.test.mjs >&2
  exit 1
}

node workers/api/public/lib/money.test.mjs >/dev/null || {
  echo "money-gate: workers/api/public/lib/money.test.mjs FAILED" >&2
  node workers/api/public/lib/money.test.mjs >&2
  exit 1
}

# The console's replica: the fold that runs in a BROWSER, on a copy the server
# does not hold. It has to agree with `workers/api/src/fold.rs` exactly -- two
# folds that disagree are a queue that disagrees with the venue's own log.
node workers/api/public/lib/replica.test.mjs >/dev/null || {
  echo "replica-gate: workers/api/public/lib/replica.test.mjs FAILED" >&2
  node workers/api/public/lib/replica.test.mjs >&2
  exit 1
}

python3 scripts/design_gate.py "$@"
gate=$?
[ "$mod_fail" = 0 ] || exit 1
exit $gate
