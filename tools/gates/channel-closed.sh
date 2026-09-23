#!/bin/sh
# G4 — WHERE AN ORDER CAME FROM IS A CLOSED SET, AND ONE FUNCTION WRITES IT.
#
# BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22 §5 G4, §3.6, §6 item 6. `channel` on an
# order was free text: the kernel stores any `Option<String>`, the storefront
# passed the word `"storefront"` by hand, the ebills mapper wrote `"ebills"` by
# hand, and nothing checked either against anything. A third route typing
# `"web"` (the kernel's own test word) would have filed its orders under a
# channel no reader has a branch for. `services/ordering/channel.rs` is the set;
# this gate is what keeps it the ONLY place the words are spelled.
#
# THREE COUNTS, over SHIPPING code only (tests.rs files and `#[cfg(test)]`
# regions removed by brace matching, the `unreached.py` way; comment lines
# dropped). The messaging files (`channels.rs`, `notify.rs`, `mcp.rs`,
# `integrations.rs`) are excluded: there `channel` means an inbox, not an
# order's source. `channel.rs` and `channel/` are excluded: they ARE the set.
#   (a) UNKNOWN — `"channel": "<word>"` / `channel: Some("<word>")` whose word
#       is not in `channel::ALL`. Any hit refuses, whatever the baseline.
#   (a) HAND COPIES — the same shapes holding a MEMBER, plus a member literal
#       passed to `place_order_at(`. A ratchet: `literals=` in the baseline.
#   (b) THE APPEND — non-test `.append(…EventKind::Placed` sites (`placed=`,
#       stays 1), and every function that builds a `PlaceIn { … }` for that
#       append must call `channel::stamp(` in the same body.
#
# `sh tools/gates/channel-closed.sh [ROOT]` — ROOT defaults to the repo so
# `channel-closed.prove.sh` can run this against a scratch copy holding a
# deliberate defect. Exit 1 on any refusal; exit 2 if the set itself could not
# be read (a gate that cannot find its set has measured nothing).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/channel-closed.baseline"

out=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob, sys
ROOT = os.environ['ROOT']
SRC = os.path.join(ROOT, 'workers/api/src')
SET = os.path.join(SRC, 'services/ordering/channel.rs')
MESSAGING = {'channels.rs', 'notify.rs', 'mcp.rs', 'integrations.rs'}

def fail2(msg):
    print('SETLESS ' + msg); sys.exit(0)

try:
    set_src = open(SET, encoding='utf-8').read()
except OSError:
    fail2('cannot read ' + SET)
m = re.search(r'pub const ALL\s*:\s*\[&(?:\'static )?str;\s*\d+\]\s*=\s*\[([^\]]*)\]', set_src)
if not m:
    fail2('no `pub const ALL` in channel.rs')
consts = dict(re.findall(r'pub const (\w+)\s*:\s*&(?:\'static )?str\s*=\s*"([^"]*)"', set_src))
ALL = []
for tok in [t.strip() for t in m.group(1).split(',') if t.strip()]:
    if tok.startswith('"'):
        ALL.append(tok.strip('"'))
    elif tok in consts:
        ALL.append(consts[tok])
    else:
        fail2('ALL names %s, which is not a string const in channel.rs' % tok)
if not ALL:
    fail2('ALL is empty')

def prod(s):
    out, i = [], 0
    while True:
        j = s.find('#[cfg(test)]', i)
        if j < 0:
            out.append(s[i:]); break
        out.append(s[i:j])
        k = s.find('{', j)
        if k < 0: break
        # `#[cfg(test)] mod tests;` has no body: skip to the `;`.
        semi = s.find(';', j)
        if 0 <= semi < k:
            i = semi + 1; continue
        depth, p = 0, k
        while p < len(s):
            if s[p] == '{': depth += 1
            elif s[p] == '}':
                depth -= 1
                if depth == 0: break
            p += 1
        i = p + 1
    body = ''.join(out)
    return '\n'.join('' if l.lstrip().startswith('//') else l for l in body.split('\n'))

def body_at(s, pos, open_ch, close_ch):
    b = s.find(open_ch, pos)
    depth, i = 1, b + 1
    while i < len(s) and depth > 0:
        if s[i] == open_ch: depth += 1
        elif s[i] == close_ch: depth -= 1
        i += 1
    return s[b:i]

unknown, copies, placed, unstamped = [], [], [], []
KEYED = re.compile(r'"channel"\s*:\s*"([^"]*)"|\bchannel\s*:\s*(?:Some\()?"([^"]*)"')
for f in sorted(glob.glob(SRC + '/**/*.rs', recursive=True)):
    rel = os.path.relpath(f, ROOT)
    base = os.path.basename(f)
    if base in MESSAGING or base == 'tests.rs' or '/tests/' in rel:
        continue
    if rel.endswith('services/ordering/channel.rs') or '/services/ordering/channel/' in rel:
        continue
    s = prod(open(f, encoding='utf-8').read())
    line = lambda pos: s.count('\n', 0, pos) + 1
    for mm in KEYED.finditer(s):
        w = mm.group(1) if mm.group(1) is not None else mm.group(2)
        (copies if w in ALL else unknown).append('%s:%d "%s"' % (rel, line(mm.start()), w))
    for mm in re.finditer(r'place_order_at\s*\(', s):
        args = body_at(s, mm.end() - 1, '(', ')')
        for lit in re.findall(r'"([^"]*)"', args):
            if lit in ALL:
                copies.append('%s:%d "%s" (place_order_at)' % (rel, line(mm.start()), lit))
    for mm in re.finditer(r'\.append\(\s*(?:dowiz_hub::)?EventKind::Placed\b', s):
        placed.append('%s:%d' % (rel, line(mm.start())))
    for mm in re.finditer(r'fn (\w+)\s*(?:<[^>]*>)?\s*\(', s):
        b = s.find('{', mm.end())
        if b < 0: continue
        body = body_at(s, b, '{', '}')
        if re.search(r'(?<!struct )\bPlaceIn\s*\{', body) and 'channel::stamp(' not in body:
            unstamped.append('%s::%s' % (rel, mm.group(1)))

print('SET ' + ','.join(ALL))
print('COUNTS %d %d %d %d' % (len(unknown), len(copies), len(placed), len(unstamped)))
for tag, rows in (('UNKNOWN', unknown), ('COPY', copies), ('PLACED', placed), ('UNSTAMPED', unstamped)):
    for r in rows:
        print('%s %s' % (tag, r))
PY
)

if printf '%s\n' "$out" | grep -q '^SETLESS'; then
  echo "channel-closed: NOT MEASURED — $(printf '%s\n' "$out" | sed -n 's/^SETLESS //p')"
  exit 2
fi
set -- $(printf '%s\n' "$out" | sed -n 's/^COUNTS //p')
unknown=$1; copies=$2; placed=$3; unstamped=$4
rows() { printf '%s\n' "$out" | sed -n "s/^$1 /  /p"; }

if [ ! -f "$BASELINE" ]; then
  printf 'literals=%s\nplaced=%s\n' "$copies" "$placed" > "$BASELINE"
  echo "channel-closed: baseline written at literals=$copies placed=$placed"
  exit 0
fi
bl=$(awk -F= '/^literals=/{print $2}' "$BASELINE")
bp=$(awk -F= '/^placed=/{print $2}' "$BASELINE")
rc=0
if [ "$unknown" -gt 0 ]; then
  echo "channel-closed: REFUSED — $unknown channel word(s) not in channel::ALL ($(printf '%s\n' "$out" | sed -n 's/^SET //p')):"
  rows UNKNOWN; rc=1
fi
if [ "$unstamped" -gt 0 ]; then
  echo "channel-closed: REFUSED — $unstamped PlaceIn builder(s) never call channel::stamp:"
  rows UNSTAMPED; rc=1
fi
if [ "$placed" -ne "$bp" ]; then
  echo "channel-closed: REFUSED — $placed non-test Placed append site(s) (baseline $bp); a new one must set the channel:"
  rows PLACED; rc=1
fi
if [ "$copies" -gt "$bl" ]; then
  echo "channel-closed: REFUSED — $copies hand-copied channel word(s) (baseline $bl); use the channel:: constants:"
  rows COPY; rc=1
elif [ "$copies" -lt "$bl" ]; then
  echo "channel-closed: $copies (baseline $bl) — the ratchet has fallen. Lower it to literals=$copies in this commit."
  rc=1
fi
[ $rc -eq 0 ] && echo "channel-closed: $copies (baseline $bl), $placed Placed append site(s), 0 unknown, 0 unstamped"
exit $rc
