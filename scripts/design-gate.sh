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
python3 scripts/design_gate.py "$@"
