#!/bin/sh
# G3 — THE VENUE'S DAY IS READ FROM THE VENUE'S ZONE, AT THE INSTANT BEING BUILT.
#
# THREE DEFECTS OF ONE SHAPE (audit 2026-09-24, D3 D10 D11), all proved, all in
# screens whose helper was already right:
#   D3  store/booking.js and admin/bookings.js took the offset in force NOW
#       (`offsetMinutes(tz, now())`) and applied it to every day in the strip.
#       On 24 October a booking for Monday the 26th at 19:00 was stored as
#       18:00, and the console's 25 October (25 hours long) was `from + 1440`.
#   D10 store/checkout.js read the "later" field with `new Date(f.value)`, in
#       the PHONE's zone: a phone set to Kyiv picking 19:00 sent the kitchen 18:00.
#   D11 store/state.js read `loc.tzOffsetMinutes`, which the hub never sends,
#       and fell back to +120 -- Tirane's SUMMER offset -- for "open now".
# `lib/booking-time.js` owns the arithmetic (by zone NAME, per candidate
# instant, node-tested over every hour of 2026). A screen that does its own
# reaches none of those tests, which is how all three shipped.
#
# REFUSED in public/{store,admin,room,courier}/**/*.js and public/lib/*.js,
# outside lib/booking-time.js and *.test.*, comments stripped:
#   a  `tzOffsetMinutes`                        -- a field nobody sends
#   b  `new Date(x.value)` / `Date.parse(`      -- a form value in the phone's zone
#   c  local-time getters/setters on a Date     -- `getHours() getDate() getDay()
#      getMinutes() getMonth() getFullYear() setHours( setMinutes( setDate(`;
#      the phone's wall, never the venue's. The UTC forms are fine.
#   d  `offsetMinutes(.., now())` / `(.., Date.now())` -- today's offset, which
#      a caller then applies to another day
#   e  importing the ONE-OFFSET forms (`midnightMs slotOf weekdayOf minuteNow`)
#      from booking-time.js into a screen: they take one offset for every day;
#      screens use `venueSlot venueDayRange venueClock venueWallMs`.
# The count is a ratchet in venue-clock.baseline (it was 0 when written).
#
#   sh tools/gates/venue-clock.sh [ROOT]      # ROOT defaults to this repo
# Exit 0 at or under the baseline, 1 over it, 2 when it could not read its
# input (no screens, or no booking-time.js): a gate that finds nothing to
# measure has measured nothing. Proof: tools/gates/venue-clock.prove.sh.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/venue-clock.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob, sys

ROOT = os.environ['ROOT']
PUB = os.path.join(ROOT, 'workers/api/public')
if not os.path.isfile(os.path.join(PUB, 'lib/booking-time.js')):
    print('UNREADABLE no workers/api/public/lib/booking-time.js'); sys.exit(0)

files = []
for d in ('store', 'admin', 'room', 'courier'):
    files += glob.glob(os.path.join(PUB, d, '**/*.js'), recursive=True)
files += glob.glob(os.path.join(PUB, 'lib/*.js'))
files = sorted(f for f in files if '.test.' not in f and not f.endswith('lib/booking-time.js'))
if len(files) < 10:
    print(f'UNREADABLE only {len(files)} screen files under {PUB}'); sys.exit(0)

def strip(src):
    """Comments out, strings and code kept, line numbers kept."""
    out, i, n, q = [], 0, len(src), None
    while i < n:
        c = src[i]
        if q:
            out.append(c)
            if c == '\\' and i + 1 < n:
                out.append(src[i + 1]); i += 2; continue
            if c == q: q = None
            i += 1; continue
        if c in '\'"`':
            q = c; out.append(c); i += 1; continue
        if src.startswith('//', i):
            j = src.find('\n', i); i = n if j < 0 else j; continue
        if src.startswith('/*', i):
            j = src.find('*/', i + 2); j = n if j < 0 else j + 2
            out.append('\n' * src.count('\n', i, j)); i = j; continue
        out.append(c); i += 1
    return ''.join(out)

RULES = [
    ('a tzOffsetMinutes', re.compile(r'\btzOffsetMinutes\b')),
    ('b form value parsed in the phone zone', re.compile(r'new\s+Date\(\s*[\w$.\[\]\'"]+\.value\s*\)|\bDate\.parse\(')),
    ('c local-time Date accessor', re.compile(r'\.(getHours|getDate|getDay|getMinutes|getMonth|getFullYear|setHours|setMinutes|setDate)\(')),
    ('d offset of NOW', re.compile(r'\boffsetMinutes\((?:[^()]|\([^()]*\))*?,\s*(Date\.now|now)\(\)')),
]
ONE_OFFSET = {'midnightMs', 'slotOf', 'weekdayOf', 'minuteNow'}
IMPORT = re.compile(r'import\s*\{([^}]*)\}\s*from\s*[\'"][^\'"]*booking-time\.js[\'"]')

hits = []
for f in files:
    code = strip(open(f, encoding='utf-8').read())
    rel = os.path.relpath(f, PUB)
    for name, rx in RULES:
        for m in rx.finditer(code):
            hits.append(f'{rel}:{code.count(chr(10), 0, m.start()) + 1}  {name}: {m.group(0)}')
    for m in IMPORT.finditer(code):
        names = {p.strip().split(' as ')[0].strip() for p in m.group(1).split(',')}
        for bad in sorted(names & ONE_OFFSET):
            hits.append(f'{rel}:{code.count(chr(10), 0, m.start()) + 1}  e one-offset form imported: {bad}')

print(len(hits), len(files))
for h in hits:
    print('  ' + h)
PY
)
case "$found" in UNREADABLE*) echo "venue-clock: $found"; exit 2;; esac
n=$(printf '%s\n' "$found" | head -1 | cut -d' ' -f1)
scanned=$(printf '%s\n' "$found" | head -1 | cut -d' ' -f2)
names=$(printf '%s\n' "$found" | tail -n +2)
case "$n" in ''|*[!0-9]*) echo "venue-clock: could not count ($found)"; exit 2;; esac

b=$(awk -F= '/^violations=/{print $2}' "$BASELINE" 2>/dev/null)
case "$b" in ''|*[!0-9]*) echo "venue-clock: no baseline at $BASELINE"; exit 2;; esac

if [ "$n" -gt "$b" ]; then
  printf '%s\n' "$names"
  echo "venue-clock: $n violations over $scanned files, baseline $b -- the venue's day is read from its zone, per instant (lib/booking-time.js)"
  exit 1
fi
[ -n "$names" ] && printf '%s\n' "$names"
echo "venue-clock: violations=$n (baseline $b) over $scanned screen files"
exit 0
