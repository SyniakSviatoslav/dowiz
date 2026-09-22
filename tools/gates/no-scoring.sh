#!/bin/sh
# F36 — NOBODY IS SCORED. The job `CLAUDE.md` said existed.
#
# THE DEFECT IS THE PROMISE ITSELF. `CLAUDE.md:102-106` states, as settled
# fact, that trust is a signed capability and never a score, "enforced two
# ways: a CI job (`no-courier-scoring`) fails the build if a
# `courier_score/rating/reputation` identifier appears in kernel/engine, and
# routing enums omit `Ord`/`PartialOrd`".
#
# THE SECOND HALF IS TRUE. THE FIRST HALF WAS NOT. No workflow matched
# `scor|reputation|rating` — the job has never existed, and the red line every
# reader was told is machine-enforced rested on one type-system property and a
# paragraph. That is the same class as `worker_errors`, a table that never
# received a row, and `StockLedger::stranded()`, a report nothing called: a
# capability believed in because it was written down.
#
# WHAT IT COUNTS, and the scope is `DECISIONS.md` OD-8's, widened by what a
# score is actually called in a product like this. A tier, a VIP flag and a
# rank are ratings of a person whatever they are computed from — the direction
# of the gaze is what makes them one, not the arithmetic.
#
# COMMENTS ARE STRIPPED FIRST, for the reason three gates here learned the hard
# way: `crates/dowiz-core/src/domain.rs` DOCUMENTS the refusal in a doc comment
# naming `courier_score`, and a gate that refuses the explanation of a rule is
# a gate that gets the explanation deleted.
#
# IT LOOKS FOR A PARTICIPANT BEING SCORED, not for the word "score", and it
# took two corrections to get there. The first pattern matched `[a-z_]*rating`
# and reported 190 sites, nearly all "ope-rating" and "gene-rating". Anchoring
# the boundaries still gave 167, because `crates/dowiz-core` is full of search
# heuristics, agent orchestration and P2P centrality that compute scores of
# THINGS — a chunk, a candidate, a node. None of those is a participant, and
# `DECISIONS.md` OD-8 is about people.
#
# So the pattern pairs a PERSON with a JUDGEMENT: `courier_score`,
# `customer_rating`, `venue_tier`. Plus `reputation` and `vip` bare, which have
# no innocent reading in a product like this. A gate with 167 false positives
# is a gate somebody deletes, which is worse than the gate that was never
# written.
#
# STRINGS ARE NOT EXEMPT. A `"rating"` in a JSON key is a score reaching a
# client just as surely as a struct field; the venue's own Google rating is the
# one real exception and it is named below, because it is a rating OF THE
# VENUE, by the public, which is the opposite direction of gaze.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/no-scoring.baseline

hits() {
  # THE PRODUCT'S OWN PATH, not the research modules. `crates/dowiz-core` also
  # holds agent orchestration, retrieval and P2P work whose scores are of
  # candidates and chunks; the decision path is what OD-8 governs.
  for f in $(find crates/dowiz-core/src/decision crates/dowiz-core/src/domain.rs \
               crates/dowiz-core/src/order_machine.rs kernel/src/decision \
               crates/dowiz-hub/src workers/api/src tools/native-spa-server/src \
               -name '*.rs' 2>/dev/null | sort); do
    sed 's,//.*,,' "$f" \
      | grep -nE '(courier|customer|client|venue|staff|waiter|user|diner|guest|partner)_(score|rating|rank|tier|reputation)|(score|rating|rank|tier|reputation)_of_(courier|customer|venue|staff)|\breputation\b|\bvip\b|\bVIP\b' \
      | sed "s|^|$f:|"
  done \
    | grep -v 'google' \
    | grep -v 'reviewCount' || true
}

n=$(hits | grep -c . || true)
echo "no-scoring: $n site(s) that rate a participant"
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-scoring: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
if [ "$n" -gt "$baseline" ]; then
  echo "no-scoring: FAILED — $((n - baseline)) more than the baseline of $baseline."
  echo "no-scoring: trust is a signed capability, never a score (DECISIONS.md OD-8)."
  hits | sed 's|^|  |'
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-scoring: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
[ "$n" -gt 0 ] && hits | sed 's|^|  |'
exit 0
