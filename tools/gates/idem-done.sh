#!/bin/sh
# G1 (AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24 §3a G-idem) — EVERY EXIT AFTER A
# CLAIMED IDEMPOTENCY KEY RECORDS ITS VERDICT.
#
# THE DEFECT THIS GUARDS (D1). `idempotency::guard` CLAIMS the key before the
# handler runs (`done: false`). A handler that then `return`s a refusal without
# telling the layer leaves the claim standing, and every retry with that key is
# answered "409 still running" for 24 hours. The courier and room outbox treats
# that 409 as "wait" and does not spend a try, so one refused tap wedged every
# tap queued behind it. Before this gate, only `booking/create.rs` recorded a
# refusal; eighteen other handlers returned without one.
#
# THE RULE. In every function body that calls `idempotency::guard(` (or
# `begin(`), every EXIT after the guard statement -- a `return`, or a `?` that
# propagates an error -- must either
#   * carry the verdict itself: `idem.refused(..)`, `idem.answered(..)`,
#     `idem.release(..)` or `idem.done(..)` in the same statement, or
#   * come after the guard was spent on the straight-line path: after a
#     `.done(`/`.refused(`/`.answered(`/`.release(` call at the body's own
#     brace depth (one inside a match arm or an `if` block spends it on that
#     branch only). `Guard` is moved by each, so the compiler refuses a second
#     verdict on the same path.
# `return`s inside a nested `async { .. }` block leave the BLOCK, not the
# handler, and are not exits (booking/create.rs records the block's answer).
#
# HOW IT COUNTS. Brace-matched function bodies with comments and string
# literals blanked first (the `unreached.py` / `consent.sh` technique), so a
# `return` in a comment or a "?" in an error message is not a hit.
#
# `sh tools/gates/idem-done.sh [ROOT]` -- ROOT defaults to the repo, so
# `idem-done.prove.sh` runs this same code against a scratch copy holding a
# deliberate defect. Exit 1 when the count is above the baseline (a ratchet
# down to 0), or below it without the baseline being lowered. Exit 2 when it
# found no guarded handler at all: a gate that finds nothing to measure has
# measured nothing.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT="${1:-$(cd "$HERE/../.." && pwd)}"
BASELINE="$HERE/idem-done.baseline"

found=$(ROOT="$ROOT" python3 - <<'PY'
import os, re, glob

ROOT = os.environ['ROOT']
GUARD = re.compile(r'idempotency::(guard|begin)\s*\(')
SPEND = re.compile(r'\.(done|refused|answered|release)\s*\(')

def blank(src):
    """Comments and string/char literals become spaces; offsets are kept."""
    out, i, n = list(src), 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            for k in range(i, j): out[k] = ' '
            i = j
        elif src.startswith('/*', i):
            j = src.find('*/', i + 2)
            j = n if j < 0 else j + 2
            for k in range(i, j):
                if src[k] != '\n': out[k] = ' '
            i = j
        elif c == 'r' and re.match(r'r#*"', src[i:i + 4]) and (i == 0 or not (src[i-1].isalnum() or src[i-1] == '_')):
            hashes = re.match(r'r(#*)"', src[i:]).group(1)
            end = '"' + hashes
            j = src.find(end, i + 2 + len(hashes))
            j = n if j < 0 else j + len(end)
            for k in range(i, j):
                if src[k] != '\n': out[k] = ' '
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            for k in range(i + 1, min(j, n)):
                if src[k] != '\n': out[k] = ' '
            i = j + 1
        elif c == "'" and re.match(r"'(\\.|[^\\'])'", src[i:i + 4]):
            m = re.match(r"'(\\.|[^\\'])'", src[i:i + 4])
            for k in range(i + 1, i + m.end() - 1): out[k] = ' '
            i += m.end()
        else:
            i += 1
    return ''.join(out)

def match_brace(s, b):
    depth, i = 1, b + 1
    while i < len(s) and depth > 0:
        if s[i] == '{': depth += 1
        elif s[i] == '}': depth -= 1
        i += 1
    return i

def drop_async_blocks(s):
    """Blank the inside of every `async { .. }` / `async move { .. }` block."""
    out = list(s)
    for m in re.finditer(r'\basync\s+(move\s+)?\{', s):
        b = m.end() - 1
        e = match_brace(s, b)
        for k in range(b + 1, e - 1):
            if out[k] != '\n': out[k] = ' '
    return ''.join(out)

CLOSURE_HEAD = re.compile(r'(?:^|[,(=\s])(?:move\s+)?\|[\w\s,:&()<>_\']*\|\s*(?:->\s*[\w:<>, ]+\s*)?$')

def in_closure(s, pos):
    """Is `pos` inside a closure's body? A `return` or `?` there leaves the
    CLOSURE (`filter_map(|it| Some(x?))`, `append_for(.., move |c| {..})`),
    not the handler. Walk out through every enclosing bracket: a `{` right
    after a closure head, or a `(` whose own text before `pos` starts one."""
    depth, inner = 0, pos
    for o in range(pos - 1, -1, -1):
        c = s[o]
        if c in ')}]':
            depth += 1
        elif c in '({[':
            if depth:
                depth -= 1
                continue
            if c == '{' and CLOSURE_HEAD.search(s[max(0, o - 120):o]):
                return True
            if c == '(':
                seg, d, flat = s[o + 1:inner], 0, []
                for ch in seg:
                    if ch in '({[': d += 1
                    elif ch in ')}]': d -= 1
                    flat.append(ch if d == 0 or ch in '({[' else ' ')
                if re.search(r'(?:^|[,(\s])(?:move\s+)?\|[\w\s,:&_\']*\|', ''.join(flat)):
                    return True
            inner = o
    return False

sites, hits = 0, []
for f in sorted(glob.glob(os.path.join(ROOT, 'workers/api/src/**/*.rs'), recursive=True)):
    if f.endswith('tests.rs') or '/tests/' in f or os.path.basename(os.path.dirname(f)) == 'idempotency':
        continue
    src = blank(open(f, encoding='utf-8').read())
    for fm in re.finditer(r'\bfn (\w+)\s*[<(]', src):
        b = src.find('{', fm.end())
        if b < 0:
            continue
        e = match_brace(src, b)
        body = src[b:e]
        g = GUARD.search(body)
        if not g:
            continue
        sites += 1
        # The guard's own statement ends at the first `;` at the body's depth
        # after the call: `let idem = match guard(..).await { .. };`. Its
        # `Err(r) => return Ok(r)` is the guard's own refusal or replay.
        depth, i = 0, g.start()
        while i < len(body):
            c = body[i]
            if c in '({[': depth += 1
            elif c in ')}]': depth -= 1
            elif c == ';' and depth == 0: break
            i += 1
        rest = drop_async_blocks(body[i + 1:])
        # The guard is SPENT only by a verdict on the straight-line path: one
        # at brace depth 0 of the body. A `.refused(` inside a match arm or an
        # `if` block spends it on that branch alone, so it does not excuse the
        # exits after it.
        limit, depth = len(rest), 0
        for k, c in enumerate(rest):
            if c == '{': depth += 1
            elif c == '}': depth -= 1
            elif depth == 0 and c == '.' and SPEND.match(rest, k):
                limit = k
                break
        if limit == len(rest):
            # No verdict on the straight-line path at all: the body's tail
            # expression is an exit too, and it records nothing.
            line = src[:e].count('\n') + 1
            hits.append(f"{os.path.relpath(f, ROOT)}:{line} {fm.group(1)}: the body ends with no verdict on its straight-line path")
        for x in re.finditer(r'\breturn\b|\?', rest[:limit]):
            if in_closure(rest, x.start()):
                continue
            # The statement the exit sits in: back to the previous `;`, `{`,
            # `}` or `=>`, forward to the next `;` or `,` at depth 0.
            s0 = max(rest.rfind(';', 0, x.start()), rest.rfind('{', 0, x.start()),
                     rest.rfind('}', 0, x.start()), rest.rfind('=>', 0, x.start()))
            depth, j = 0, x.start()
            while j < len(rest):
                c = rest[j]
                if c in '({[': depth += 1
                elif c in ')}]':
                    if depth == 0: break
                    depth -= 1
                elif c in ';,' and depth == 0: break
                j += 1
            stmt = rest[s0 + 1:j]
            if SPEND.search(stmt):
                continue
            line = src[:b + i + 1 + x.start()].count('\n') + 1
            what = 'return' if x.group(0) == 'return' else '`?`'
            hits.append(f"{os.path.relpath(f, ROOT)}:{line} {fm.group(1)}: {what} after the guard, verdict not recorded")

print(sites)
print(len(hits))
for h in hits:
    print('  ' + h)
PY
)
sites=$(printf '%s\n' "$found" | sed -n 1p)
n=$(printf '%s\n' "$found" | sed -n 2p)
names=$(printf '%s\n' "$found" | tail -n +3)

if [ -z "$sites" ] || [ "$sites" -eq 0 ]; then
  echo "idem-done: found no handler calling idempotency::guard under $ROOT/workers/api/src -- the parse broke, not the feature"
  exit 2
fi
[ -f "$BASELINE" ] || { echo "idem-done: no baseline at $BASELINE"; exit 2; }
b=$(awk -F= '/^unrecorded_returns=/{print $2}' "$BASELINE")

if [ "$n" -gt "$b" ]; then
  echo "idem-done: REFUSED -- $n exit(s) after a claimed idempotency key record no verdict (baseline $b, $sites guarded handlers):"
  printf '%s\n' "$names"
  echo "idem-done: answer through idem.refused(&place, status, &text) / idem.answered(&place, res) so a retry"
  echo "idem-done: replays the refusal instead of waiting 24 h on a claim nobody will finish (D1)."
  exit 1
fi
if [ "$n" -lt "$b" ]; then
  echo "idem-done: $n (baseline $b) -- the ratchet has fallen. Lower it to unrecorded_returns=$n in this commit."
  exit 1
fi
echo "idem-done: $n unrecorded exit(s) across $sites guarded handler(s) (baseline $b)"
