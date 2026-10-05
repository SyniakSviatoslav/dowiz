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
# NO_SCORING_ROOT points the whole scan at a scratch tree (the proof); unset, the product.
R=${NO_SCORING_ROOT:-.}
BASELINE_FILE=${NO_SCORING_BASELINE:-tools/gates/no-scoring.baseline}

# ── GATE-1, AND WHERE A GUEST MAY BE SCORED (W-MR0, 2026-10-04) ──────────────────────────────
# OPERATOR DECISION 2026-10-04, verbatim: "смак і поведінку гостя треба оцінювати ... на пристрої
# і сервері" -- the guest's TASTE and BEHAVIOUR are scored, on the device and on the server,
# automatically, stopped by the guest's one-tap objection (ruling of the same day; DECISIONS.md D0). That decision is a scope, and
# this gate is where the scope is written down. Three classes of name:
#
#   1. OD-8, UNCHANGED, EVERYWHERE ON THE SERVER: a participant rated, ranked, tiered, given a
#      reputation or a VIP flag (`courier_score`, `customer_tier`, `vip`). A taste profile is not a
#      verdict on a person; a tier is. Staff and courier rules are untouched (the staff
#      personal-KPI amendment is a separate row).
#   2. GUEST TASTE/BEHAVIOUR (`guest_taste`, `customer_affinity`, `guest_segment`, `customer_ltv`...):
#      ALLOWED in the guest's own browser (workers/api/public/store/**, never scanned for these) and
#      in the ONE server module that computes the profile, `$SCORER_RS` below. REFUSED in every other
#      server file: a score computed anywhere else escapes the objection and the DPIA.
#   3. PRICING BY PERSON, REFUSED EVERYWHERE, the browser included: `price_sensitivity`,
#      `willingness_to_pay`, `wtp`. The operator asked for scoring, not pricing. A price, a discount,
#      an eligibility or a refusal set from a guest's score is personalised pricing, which EU
#      Omnibus Directive 2019/2161 (art. 6(1)(ea) of 2011/83/EU) requires to be disclosed to every
#      guest, and a refusal from a score is an Art. 22 GDPR decision. So the two scorer files may
#      not even NAME a price, a discount, a promo, an eligibility or a refusal (`MONEY_WORDS`).
#
# Matched case-insensitively, so `GUEST_AFFINITY` or `Guest_Ltv` counts too (a CamelCase
# `GuestAffinity` has no underscore and is not matched; the snake field it serialises is).
# Comments are stripped first, as before.
PERSON_JUDGED='(courier|customer|client|venue|staff|waiter|user|diner|guest|partner)_(score|rating|rank|tier|reputation)|(score|rating|rank|tier|reputation)_of_(courier|customer|venue|staff)|\breputation\b|\bvip\b'
GUEST_SCORED='(guest|customer|client|user|diner)_(taste|affinity|propensity|segment|cohort|ltv|spend|profile)'
PRICED_BY_PERSON='price_sensitivity|willingness_to_pay|\bwtp\b'
MONEY_WORDS='price|discount|promo|eligib|refus'
# 4. WHAT GUESTS' NOTES ARE ABOUT, BY PERSON (W-VOICE P16b, 2026-10-05): the topics of
#    `feedback.text` are folded per DISH per WEEK only (`services/analytics/kitchen/topics.rs`).
#    "late" counted per courier is a courier's record; "rude" per waiter is a waiter's. So a
#    topic, a sentiment or a complaint keyed by a person or an order is refused on the server.
PERSON_TOPICS='(courier|customer|client|guest|staff|waiter|user|diner|person|order|phone)_(topics?|sentiment|mood|complaints?)|(topics?|sentiment|mood|complaints?)_(by|per|of)_(courier|customer|client|guest|staff|waiter|user|diner|person|order|phone)'
# The ONE server module allowed to hold class 2 (and its tests directory), and the device scorer.
SCORER_RS=workers/api/src/services/customers/taste
SCORER_JS=workers/api/public/store/taste.js

server_files() {
  # THE PRODUCT'S OWN PATH, not the research modules. `crates/dowiz-core` also
  # holds agent orchestration, retrieval and P2P work whose scores are of
  # candidates and chunks; the decision path is what OD-8 governs.
  for d in crates/dowiz-core/src/decision crates/dowiz-core/src/domain.rs crates/dowiz-core/src/order_machine.rs \
           kernel/src/decision crates/dowiz-hub/src workers/api/src; do
    [ -e "$R/$d" ] && find "$R/$d" -name '*.rs'
  done | sort
}
browser_files() { [ -d "$R/workers/api/public" ] && find "$R/workers/api/public" -name '*.js' ! -name '*.test.*' | sort; }
code() { sed 's,//.*,,' "$1"; }   # comments out (a `//` inside a string is cut too: a gate may over-count, never under)

hits() {
  for f in $(server_files); do
    rel=${f#"$R"/}
    code "$f" | grep -niE "$PERSON_JUDGED|$PRICED_BY_PERSON|$PERSON_TOPICS" | sed "s|^|$rel:|"
    case "$rel" in
      "$SCORER_RS".rs|"$SCORER_RS"/*) code "$f" | grep -niE "$MONEY_WORDS" | sed "s|^|$rel: (a guest scorer naming money) |" ;;
      *) code "$f" | grep -niE "$GUEST_SCORED" | sed "s|^|$rel: (a guest score outside $SCORER_RS) |" ;;
    esac
  done
  for f in $(browser_files); do
    rel=${f#"$R"/}
    code "$f" | grep -niE "$PRICED_BY_PERSON" | sed "s|^|$rel:|"
    [ "$rel" = "$SCORER_JS" ] && code "$f" | grep -niE "$MONEY_WORDS" | sed "s|^|$rel: (a guest scorer naming money) |"
  done
} 2>/dev/null
filtered() { hits | grep -v 'google' | grep -v 'reviewCount' || true; }

n=$(filtered | grep -c . || true)
echo "no-scoring: $n site(s) that rate a participant, price by person, or score a guest outside the one scorer"
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-scoring: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
if [ "$n" -gt "$baseline" ]; then
  echo "no-scoring: FAILED — $((n - baseline)) more than the baseline of $baseline."
  echo "no-scoring: trust is a signed capability, never a score (DECISIONS.md OD-8); a guest is scored only in $SCORER_RS and $SCORER_JS, never to set a price (D0)."
  filtered | sed 's|^|  |'
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "no-scoring: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
[ "$n" -gt 0 ] && filtered | sed 's|^|  |'
exit 0
