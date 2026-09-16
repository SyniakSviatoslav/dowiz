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
  for f in workers/api/public/app.js workers/api/public/admin/app.js \
           workers/api/public/courier/app.js workers/api/public/lib/*.js; do
    [ -f "$f" ] || continue
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
node workers/api/public/lib/money.test.mjs >/dev/null || {
  echo "money-gate: workers/api/public/lib/money.test.mjs FAILED" >&2
  node workers/api/public/lib/money.test.mjs >&2
  exit 1
}

python3 scripts/design_gate.py "$@"
gate=$?
[ "$mod_fail" = 0 ] || exit 1
exit $gate
