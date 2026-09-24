#!/bin/sh
# venue-clock.sh's proof: a gate is triggered before it is trusted.
#
# Runs the gate against scratch copies of workers/api/public:
#   clean                                         -> 0
#   harmless: the word in a comment, UTC getters  -> 0   (the green half)
#   `state.loc.tzOffsetMinutes` in store/booking  -> 1   (D11's shape)
#   `new Date(f.value).getTime()` in checkout     -> 1   (D10's shape)
#   `offsetMinutes(tz(), Date.now())` in admin    -> 1   (D3's shape)
#   `import { midnightMs }` into admin/bookings   -> 1   (D3's one-offset form)
#   `d.getHours()` in room/                       -> 1
#   no booking-time.js                            -> 2   (unreadable input)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
P="$SCRATCH/r/workers/api/public"

copy() { rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"; cp -r "$REPO/workers/api/public" "$P"; }
run() { sh "$HERE/venue-clock.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; echo $?; }
want() { # want <rc> <label>
  rc=$(run); echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 -v '^$' "$SCRATCH/out" | cut -c1-110)"
  [ "$rc" -eq "$1" ] || { cat "$SCRATCH/out"; fail=1; }
}

copy; want 0 'clean'

copy
printf '\n// tzOffsetMinutes was never sent; new Date(f.value) read the phone.\nexport const utcHour = d => d.getUTCHours() + d.getUTCMinutes();\n' >> "$P/store/state.js"
want 0 'harmless: comment + UTC getters'

copy
printf '\nexport const legacyOff = () => state.loc.tzOffsetMinutes;\n' >> "$P/store/booking.js"
want 1 'tzOffsetMinutes in store/booking.js'

copy
printf '\nexport const legacyWhen = f => new Date(f.value).getTime();\n' >> "$P/store/checkout.js"
want 1 'new Date(f.value) in store/checkout.js'

copy
printf '\nimport { offsetMinutes as om } from "/lib/booking-time.js";\nexport const legacy = () => offsetMinutes(tz(), Date.now());\n' >> "$P/admin/bookings.js"
want 1 'offsetMinutes(tz(), Date.now()) in admin/bookings.js'

copy
printf '\nimport { midnightMs } from "/lib/booking-time.js";\n' >> "$P/admin/bookings.js"
want 1 'midnightMs imported into admin/bookings.js'

copy
f=$(ls "$P"/room/*.js | head -1)
printf '\nexport const hourNow = () => new Date().getHours();\n' >> "$f"
want 1 "getHours() in room/$(basename "$f")"

copy; rm "$P/lib/booking-time.js"
want 2 'no booking-time.js'

[ $fail -eq 0 ] && echo "venue-clock.prove: the gate fires in both directions" || echo "venue-clock.prove: FAILED"
exit $fail
