#!/bin/sh
# F32 — THE WORKER DECIDES WHAT TIME IT IS IN ONE PLACE.
#
# THE DEFECT THIS COUNTS, and it has cost two commits already. `486a5c38`
# ("analytics: the days were 24 hours apart, and the venue's are not") and the
# timezone fix recorded in `dowiz-venue-timezone` were both a handler deciding
# what "now" and "today" mean from its own clock read. A handler that reads the
# clock itself cannot be TOLD what time it is, so the only way to test a day
# boundary, a promo window or an expiry is to wait for one -- which is why none
# of them had a test until the pure part was lifted out.
#
# `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` P3 is the fix: `now_ms` is
# read ONCE per request and passed down.
#
# IT COUNTS `now_ms()` AS WELL AS `Date::now()`, AND THAT IS THE WHOLE
# DIFFERENCE BETWEEN THIS AND AN INSTRUMENT THAT MEASURES NOTHING. P3's stated
# check is "count of `Date::now` outside the allow-list, baseline 27". That
# number is reachable in an afternoon by wrapping every call in a helper, which
# moves no decision anywhere and leaves every handler still choosing its own
# time. What matters is the SITE that decides, not the spelling it decides in.
# Counted both ways the real figure is 93, not 27 -- measured 2026-09-22, and
# the gap is worth knowing before anyone budgets P3 from the blueprint's
# estimate.
#
# THREE PLACES ARE ALLOWED, each for a stated reason rather than because it was
# inconvenient to move:
#   * the definition of `owner::now_ms` -- the one canonical read.
#   * `otel.rs` -- the tracer. It measures ELAPSED WALL TIME; a span handed a
#     fixed clock would report every request as taking zero ms. An instrument
#     that reads the real clock is not a decision that reads the real clock.
#   * `hubdo.rs` -- inside the Durable Object, which is where an append is
#     stamped and a courier fix is timed. That read happens after the request
#     has crossed the boundary and is the object's own.
#
# COMMENTS ARE STRIPPED BEFORE ANYTHING IS COUNTED, and this gate needed the
# lesson immediately: the commit that deleted two of the three copies of
# `now_ms` left a note in each file saying "three copies of
# `Date::now().as_millis() as i64`", and the first version counted BOTH notes --
# so removing two clock reads moved the number by zero. `no-sql.sh` had just
# been rewritten for exactly the same reason. A gate that punishes the note
# explaining a deletion teaches people to delete the note.
#
# THE RATCHET MAY ONLY FALL. Same mechanism as file-size and no-sql: a target
# nobody reaches in one commit still has to be monotone, or it is a wish.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/clock.baseline

# `owner.rs` is NOT exempt as a file -- only the one-line body of `now_ms` is.
# The rest of it is counted, which is the point: `owner.rs:1382` stamps a record
# from the wall clock inside a handler, and that is the shape P3 removes.
hits() {
  for f in $(find workers/api/src -name '*.rs' | sort); do
    sed 's,//.*,,' "$f" | grep -nE 'Date::now\(\)|now_ms\(\)' | sed "s|^|$f:|"
  done \
    | grep -v '^workers/api/src/otel\.rs:' \
    | grep -v '^workers/api/src/hubdo\.rs:' \
    | grep -v '^workers/api/src/owner\.rs:[0-9]*: *Date::now()\.as_millis() as i64 *$'
}
n=$(hits | wc -l | tr -d ' ')
echo "clock: $n site(s) decide the time for themselves, outside the three allowed places"
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "clock: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
if [ "$n" -gt "$baseline" ]; then
  echo "clock: FAILED — $((n - baseline)) site(s) ADDED. The ratchet only goes down."
  echo "Per file now:"
  hits | sed 's|:.*||' | sort | uniq -c | sort -rn | sed 's|workers/api/src/|  |'
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "clock: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
[ "$n" -eq 0 ] && echo "clock: ZERO. The Worker decides the time once and passes it down."
exit 0
