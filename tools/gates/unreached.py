#!/usr/bin/env python3
"""F35 — A CAPABILITY NOBODY CALLS.

THE DEFECT CLASS, and it is the one this repo keeps paying for. `StockLedger::
stranded()` is a conservation report: it has been able to name every order
holding ingredients it should not since the day it was written, it has tests,
and NOTHING IN PRODUCTION EVER CALLED IT. The same shape produced
`worker_errors`, a table that never received a row in its whole life;
`Hub::chain_check`, which shipped in phase 4 and was first called by the
nightly months later; and four browser copies of a status set that were each
short by the same state.

The common form is not "dead code". It is a capability that EXISTS, COMPILES
and IS TESTED, so every signal a normal project has says it is fine — and no
live path reaches it. A green test suite is not evidence that anything is
switched on.

WHAT IT COUNTS. Every `pub fn` declared in the three trees below, referenced
nowhere in the SHIPPING code of any consumer. Test modules are removed first,
because being called only by its own tests is precisely the state this looks
for.

TWO BUGS OF ITS OWN, BOTH FOUND BY CHECKING THE OUTPUT BY HAND BEFORE
BELIEVING IT, and both left in the record because an audit that is itself
wrong is worse than none -- it gets acted on:

  * 24 -> 11: the first version searched only `workers/api`, and `dowiz-hub`
    has TWO consumers. It accused the entire roster and subscription
    subsystems of being dead because the twin server was not in the list.
  * 11 -> 6: `prod()` cut each file at the FIRST `#[cfg(test)]`, so everything
    after a mid-file test module read as test code. `hubdo.rs` has one around
    line 240, and three real calls to `EventKind::from_u8` below it were
    invisible.

WHAT IT CANNOT SEE: a capability reached only through a trait object or a
macro, and one whose only caller is a browser. It is a name-reference scan, not
a call graph. It is enough for the shape above, which is what it is for.

Run: `python3 tools/gates/unreached.py`. Exit 1 when the count rises above
`tools/gates/unreached.baseline`.
"""
import re, subprocess, collections, os
os.chdir(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))  # its OWN repo: a hard-coded /root/dowiz made every worktree run measure the main tree (2026-09-24)
# WHERE CAPABILITIES ARE DECLARED, and WHERE ANY CONSUMER COULD CALL THEM.
# The second list must be complete or the catalogue is a fiction: `dowiz-hub`
# has TWO consumers, and leaving the twin server out would accuse the whole
# roster subsystem of being dead.
DECL = ['crates/dowiz-hub/src', 'crates/bebop-store/src', 'workers/api/src']
USE  = DECL + ['tools/native-spa-server/src']
dfiles = subprocess.check_output(['find']+DECL+['-name','*.rs']).decode().split()
ufiles = subprocess.check_output(['find']+USE+['-name','*.rs']).decode().split()
src = {f: open(f).read() for f in set(dfiles) | set(ufiles)}

def prod(s):
    """The code that ships: every `#[cfg(test)]` region removed by BRACE
    MATCHING, not by cutting at the first one.

    THE BUG THIS REPLACES, and it is the whole reason an audit gets verified
    before it is believed: cutting at the first `#[cfg(test)]` treats
    everything after a mid-file test module as test code. `hubdo.rs` has one at
    line ~240, so three real calls to `EventKind::from_u8` below it were
    invisible and the function was catalogued as dead. A dead-code audit that
    is itself wrong is worse than none: it gets acted on."""
    out, i = [], 0
    while True:
        j = s.find('#[cfg(test)]', i)
        if j < 0:
            out.append(s[i:]); break
        out.append(s[i:j])
        k = s.find('{', j)
        if k < 0: break
        depth, m = 0, k
        while m < len(s):
            if s[m] == '{': depth += 1
            elif s[m] == '}':
                depth -= 1
                if depth == 0: break
            m += 1
        i = m + 1
    return ''.join(out)

decls = collections.defaultdict(list)
for f in dfiles:
    for m in re.finditer(r'^\s*pub(?:\(crate\))? (?:async )?fn (\w+)', src[f], re.M):
        decls[m.group(1)].append(f)

SKIP = {'new','default','fmt','from','load','create','to_bytes','clone','next','eq','hash','as_str','len','is_empty'}
rows = []
for name, where in decls.items():
    if name in SKIP: continue
    pat = re.compile(r'\b' + re.escape(name) + r'\b')
    p = t = 0
    for f in ufiles:
        s = src[f]
        pp = prod(s)
        np_, nt = len(pat.findall(pp)), len(pat.findall(s)) - len(pat.findall(pp))
        if f in where: np_ -= 1
        p += max(0, np_); t += max(0, nt)
    if p == 0:
        rows.append((where[0], name, t))
rows.sort()
n = len(rows)
print(f"unreached: {n} public capability/capabilities that no shipping code references")
by = collections.defaultdict(list)
for f, name, t in rows: by[f].append((name, t))
for f, items in sorted(by.items()):
    print(f"  {f}  ({len(items)})")
    for name, t in sorted(items):
        print(f"      {name}" + (f"   [{t} test refs: tested, and unreachable]" if t else "   [NO TESTS EITHER]"))

BASE = 'tools/gates/unreached.baseline'
if not os.path.exists(BASE):
    open(BASE, 'w').write(f"{n}\n")
    print(f"unreached: baseline recorded at {n}")
    raise SystemExit(0)
base = int(open(BASE).read().strip())
if n > base:
    print(f"unreached: FAILED — {n - base} more than the baseline of {base}.")
    print("unreached: something was built and nothing was wired to it. Wire it, or delete it")
    print("unreached: and say in the deletion what it was for. `#[allow(dead_code)]` is not an answer.")
    raise SystemExit(1)
if n < base:
    open(BASE, 'w').write(f"{n}\n")
    print(f"unreached: ratchet lowered {base} -> {n}. Commit the baseline with the change.")
raise SystemExit(0)
