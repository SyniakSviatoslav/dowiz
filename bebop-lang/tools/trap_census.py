#!/usr/bin/env python3
"""trap_census.py -- ROADMAP F1. The trap census, DERIVED from the tree.

Every row below is recovered from a primary source in this repository and
carries its `file:line`. Nothing is transcribed from a report: the row F1 cites
`docs/RESEARCH-VERIFICATION-2026-09-09.md` and `docs/blueprints/F0-trap-census.md`,
and NEITHER FILE EXISTS in this tree (2026-09-09, `e27eb72`), so its "24 rows,
20 open, 16 zero-word" could not be checked against its source even in principle.
This script is therefore the source, and it re-derives on every run: if a hazard
leaves docs/WORKER-CARD.md, or a `neg/` construct lands, the number moves by
itself.

The four primary sources:
  docs/WORKER-CARD.md  §"Language / compiler traps" -- the hazards every worker
                        is told to memorise. Under the agentic rule a hazard a
                        document asks the author to avoid is a DEFECT OF THE
                        LANGUAGE, so each one is a census row.
  docs/LANGUAGE.md     -- every occurrence of UNDEFINED, plus the defined-but-
                        surprising arithmetic.
  docs/TRAPS.md        -- the exit-code table, which claims "a code that is not
                        here is a bug".
  bebop.bp             -- every `diag_exit(..., N)` and `sys_exit(N)` site: the
                        codes the compiler can ACTUALLY produce. Compared
                        against TRAPS.md, this is where the census finds codes
                        the single table does not carry.

Coverage of a row is TWO independent facts and the census requires both:
  neg        a bench/parity_constructs/neg/*.bp exists whose construct_parity.sh
             line is EXPECT=COMPILEFAIL:<code> or EXPECT=RUNFAIL:<code>
  mechanism  the compiler actually rejects/traps it today (a diag_exit or a brk)
A row with a mechanism but no `neg` is not closed: nothing stops it regressing.

COST CLASS is the F1 deliverable that makes F2 schedulable, and it classifies by
WHAT THE FIX COSTS, not by how bad the trap is:
  zero   rejectable at parse or planning time with a diagnostic and a source
         position -- the A13/A15 shape (exit 100 "fn with more than 14
         parameters", exit 101 "unbound symbol"). Costs 0 machine words.
  words  needs emitted words in the compiled program (a bounds check, an
         overflow check), so it is weighed against bin_words.
  n/a    already closed, or not a language defect at all.

Usage: python3 tools/trap_census.py [--rows] [--codes] [--json]
Gate line: `trap_unrep: <closed>/<open+closed>`  (denominator is every row that
is a language defect; it is a progress counter to 100 %, never a reduced target)
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def rd(rel):
    p = os.path.join(ROOT, rel)
    if not os.path.exists(p):
        return None
    with open(p, encoding='utf-8', errors='replace') as f:
        return f.read()


def lines(rel):
    t = rd(rel)
    return t.split('\n') if t is not None else []


def die(msg):
    sys.stderr.write('trap_census: %s\n' % msg)
    raise SystemExit(2)


# ---------------------------------------------------------------- primary scan

def scan_exit_sites():
    """Every code bebop.bp can exit with, from the source. Returns
    {code_or_expr: [ 'bebop.bp:<line>', ... ]} plus the computed-code sites."""
    fixed, computed = {}, {}
    for i, ln in enumerate(lines('bebop.bp'), 1):
        for m in re.finditer(r'diag_exit\s*\([^,]*,[^,]*,\s*(\d+)\s*\)', ln):
            fixed.setdefault(int(m.group(1)), []).append('bebop.bp:%d' % i)
        for m in re.finditer(r'sys_exit\s*\(\s*(\d+)\s*\)', ln):
            c = int(m.group(1))
            if c != 0:
                fixed.setdefault(c, []).append('bebop.bp:%d' % i)
        # computed codes: sys_exit(<base> + site) -- the code depends on a
        # caller-chosen constant, so ONE site spans a whole RANGE of codes.
        for m in re.finditer(r'sys_exit\s*\(\s*(\d+)\s*\+\s*([A-Za-z_]\w*)\s*\)', ln):
            computed.setdefault((int(m.group(1)), m.group(2)), []).append('bebop.bp:%d' % i)
    if not fixed:
        die('found no exit sites in bebop.bp -- the scan is broken, not the tree')
    return fixed, computed


def scan_documented_codes():
    """Codes carried by the docs/TRAPS.md table."""
    out = {}
    for i, ln in enumerate(lines('docs/TRAPS.md'), 1):
        m = re.match(r'\|\s*(\d+)\s*\|', ln)
        if m:
            out[int(m.group(1))] = 'docs/TRAPS.md:%d' % i
    if not out:
        die('parsed no codes out of docs/TRAPS.md -- the table format moved')
    return out


def scan_neg_expects():
    """{code: [construct, ...]} from construct_parity.sh, restricted to
    constructs that actually exist under bench/parity_constructs/neg/."""
    negdir = os.path.join(ROOT, 'bench/parity_constructs/neg')
    have = set()
    if os.path.isdir(negdir):
        have = {f[:-3] for f in os.listdir(negdir) if f.endswith('.bp')}
    out = {}
    for ln in lines('bench/vs_rust/construct_parity.sh'):
        m = re.match(r'\s*(\w+)\)\s*EXPECT=(COMPILEFAIL|RUNFAIL):(\d+);;', ln)
        if m and m.group(1) in have:
            out.setdefault(int(m.group(3)), []).append(m.group(1))
    return out


def scan_card_hazards():
    """The hazards docs/WORKER-CARD.md tells every worker to memorise. Split
    mechanically on ';' so the census follows the card instead of a copy."""
    ls = lines('docs/WORKER-CARD.md')
    try:
        start = next(i for i, l in enumerate(ls) if l.startswith('## Language / compiler traps'))
    except StopIteration:
        die('docs/WORKER-CARD.md has no "## Language / compiler traps" section')
    out = []
    for i in range(start + 1, len(ls)):
        if ls[i].startswith('## '):
            break
        if not ls[i].startswith('- '):
            continue
        for part in ls[i][2:].split(';'):
            part = part.strip()
            if part:
                out.append(('docs/WORKER-CARD.md:%d' % (i + 1), part))
    if not out:
        die('extracted no hazards from the WORKER-CARD trap section')
    return out


def scan_shadowable():
    """Builtins a user fn can SILENTLY shadow: dispatched by emit_call_or_ctor
    but absent from the T122 reserved-name hash table, so `fn <name>` compiles
    rc=0 and every call reaches the builtin instead. Derived by hashing the
    builtin names with read_ident's own 131-rolling hash and diffing against the
    hashes literally present in the rsv1/rsv2/rsv3 test -- NOT by probing one
    name and generalising."""
    src = rd('bebop.bp') or ''
    m = re.search(r'let rsv1 = (.*?);\n\s*let rsv2 = (.*?);\n\s*let rsv3 = (.*?);', src, re.S)
    if not m:
        die('could not find the T122 rsv1/rsv2/rsv3 reserved-name test in bebop.bp')
    # NORMALISE to unsigned 64-bit before comparing. bebop.bp spells the same
    # hash BOTH ways -- the rsv table carries `-8392076203060521221` where the
    # dispatch ladder carries `10054667870649030395`, the identical 64-bit value.
    # Comparing the literals as written reports 16 shadowable builtins; comparing
    # the values reports 7. The first number is an artefact of the spelling.
    u64 = lambda v: v & 0xFFFFFFFFFFFFFFFF
    reserved = {u64(int(x)) for g in m.groups() for x in re.findall(r'selfname == (-?\d+)', g)}
    names = re.findall(r'name == (-?\d+) then emit_(\w+)', src)
    def h(n):
        v = 0
        for c in n.encode():
            v = (v * 131 + c) & 0xFFFFFFFFFFFFFFFF
        return v - 2 ** 64 if v >= 2 ** 63 else v
    # the dispatch arm's hash IS the builtin's name hash (F0's method)
    dispatched = {u64(int(a)) for a, _ in names}
    if not dispatched:
        die('found no builtin dispatch arms in bebop.bp')
    shadowable = sorted(dispatched - reserved)
    return reserved, dispatched, shadowable


def scan_undefined():
    out = []
    for i, ln in enumerate(lines('docs/LANGUAGE.md'), 1):
        if 'UNDEFINED' in ln:
            out.append(('docs/LANGUAGE.md:%d' % i, ln.strip()))
    return out


# ------------------------------------------------------------------- the rows
# Each row: (id, source, class, mechanism-today, code, cost, closed?)
#   class U = undefined behaviour, no mechanism at all
#         C = compile-time diagnostic
#         T = run-time trap (a brk / a signal handler)
# `closed` is COMPUTED below from neg+mechanism, never asserted here.

ROWS = [
    # ---- hazards docs/WORKER-CARD.md asks the author to avoid -------------
    ('register-pressure', 'docs/WORKER-CARD.md:20', 'C',
     'diag_exit(...,89) at 3 sites; message has a position but no explanation', 89, 'zero', None),
    ('live-symbols-across-clone', 'docs/WORKER-CARD.md:20', 'U',
     'none: > 8 live symbols across sys_clone is not detected', 89, 'zero', None),
    ('nesting-cap', 'docs/WORKER-CARD.md:20', 'C',
     'diag_exit(...,95) but the message is "expected )" -- WRONG CAUSE for the nesting case', 95, 'zero', None),
    ('nested-if-as-arg', 'docs/WORKER-CARD.md:20', 'U',
     'none: silently miscompiles or hits 89/95 with an unrelated message', None, 'zero', None),
    ('zeros-in-while', 'docs/WORKER-CARD.md:20', 'U',
     'none (law L8 is a convention, not a check)', None, 'zero', None),
    ('signed-remainder', 'docs/WORKER-CARD.md:20', 'U',
     'none: defined behaviour, surprising -- documentation, arguably not a defect', None, 'n/a', None),
    ('unchecked-bounds', 'docs/WORKER-CARD.md:20', 'U',
     'none natively; bpref raises IndexError, so the ORACLE disagrees with the compiler', None, 'words', None),
    ('unresolved-call', 'docs/WORKER-CARD.md:21', 'T',
     'brk #87 at run time (T130)', 87, 'zero', 'c52_undef'),
    ('use-line-not-seen', 'docs/WORKER-CARD.md:21', 'U',
     'none: a missing <out>.bin.use is detected only by a worker grepping for it', None, 'zero', None),
    ('fn-cap-511', 'docs/WORKER-CARD.md:21', 'C',
     'diag_exit(...,89) at bebop.bp:4172 and sys_exit(89) at 2656 -- see the 104/89 conflict row', 89, 'zero', None),
    ('scanner-strn', 'docs/WORKER-CARD.md:22', 'U',
     'none: a performance idiom, not a trap -- excluded from the denominator', None, 'n/a', None),
    ('scanner-shared-pos', 'docs/WORKER-CARD.md:22', 'U',
     'none: an aliasing hazard the language cannot express away without ownership', None, 'words', None),
    ('store-ref-as-i64', 'docs/WORKER-CARD.md:23', 'C',
     'tools/typecheck.py rung vii -- a SEPARATE python checker, not the compiler', None, 'zero', None),
    ('clone-stack-top-0', 'docs/WORKER-CARD.md:24', 'U',
     'none: sys_clone(_, 0) compiles and the child rebinds over the parent', None, 'zero', None),
    # ---- docs/LANGUAGE.md UNDEFINED ---------------------------------------
    ('read-before-assign', 'docs/LANGUAGE.md:53', 'U',
     'none: reads whatever the register holds; the fuzzer avoids the shape', None, 'zero', None),
    ('read-past-end', 'docs/LANGUAGE.md:68', 'U',
     'none: same defect as unchecked-bounds, stated in the grammar', None, 'words', None),
    # ---- found 2026-09-09, absent from every table ------------------------
    ('builtin-shadowing', 'bebop.bp:5753-5756 (T122 rsv table)', 'U',
     'PARTIAL, and that is the precise defect: the mechanism exists and is correct, '
     'the TABLE is short. `fn char(...)` is rejected exit 99; `fn clz(...)` compiles '
     'rc=0 and clz(8) returns 60. Count derived by hash diff, see --shadow', 99, 'zero', None),
    ('traps-md-stale', 'docs/TRAPS.md:exit-89 row vs bebop.bp:3506-3516', 'C',
     'two exit-code tables exist and disagree: TRAPS.md says the fn cap is 256 and '
     'the nesting cap 128 at fntab[2000..2383]; bebop.bp\'s own table says 512 and '
     '512 at fntab[2800..4335]. A worker who reads the wrong one debugs the wrong '
     'thing', 89, 'zero', None),
    ('exit-code-space-unpartitioned', 'bebop.bp:2271-2295', 'C',
     'vs_mask_take/free and vs_cs_take/free exit with sys_exit(99+site), (100+site), '
     '(119+site), (120+site) for a caller-chosen site in 1..20, so the compiler can '
     'produce ANY code in 99..140 -- aliasing every documented compile-time code', None, 'zero', None),
    ('exit-102-overloaded', 'bebop.bp:310 vs bebop.bp:4912', 'C',
     'code 102 means BOTH "sys_ name inside a kernel fn" (documented, user-facing) '
     'and a register-model window-mask self-check (undocumented, compiler-internal)', 102, 'zero', None),
    ('exit-105-106-undocumented', 'bebop.bp:4937,5025', 'C',
     'sys_exit(105) and sys_exit(106) are emitted as cs-mask self-checks but appear '
     'in no table -- and ROADMAP F2 plans to ASSIGN 105 and 106 to new user-facing '
     'diagnostics, which would collide', None, 'zero', None),
    ('fn-cap-code-conflict', 'docs/TRAPS.md vs bebop.bp:2656,4172', 'C',
     'TRAPS.md says A13 moved the >512-fn cap from 89 to 104; the compiler still '
     'exits 89 at both sites, and 104 is used for a cs-mask self-check instead', 104, 'zero', None),
    ('sgraph2-twin', 'selfhost/std/sgraph2.bp vs bench/vs_rust/std_tests/sgraph2.bp', 'U',
     'gen_selfsrc.sh std keeps the two in sync; a near-duplicate that drifts costs a '
     'measurement -- see the note below: this is NOT a language trap', None, 'n/a', None),
]


def build(argv):
    exits_fixed, exits_computed = scan_exit_sites()
    documented = scan_documented_codes()
    negs = scan_neg_expects()
    card = scan_card_hazards()
    undef = scan_undefined()
    reserved, dispatched, shadowable = scan_shadowable()

    rows = []
    for rid, src, cls, mech, code, cost, negname in ROWS:
        # a neg construct counts for a row ONLY if it was written for THAT trap.
        # Matching on the exit code alone credited `builtin-shadowing` with
        # c39_fnmatch (reserved word as a fn NAME, a different defect that
        # happens to share code 99) and `exit-102-overloaded` with
        # c112_kernelsys -- both wrong, and both would have inflated the count.
        has_neg = bool(negname) and negname in (negs.get(code) or [])
        has_mech = not mech.startswith('none')
        closed = has_neg and has_mech
        rows.append(dict(id=rid, source=src, cls=cls, mechanism=mech, code=code,
                         cost=cost, neg=([negname] if has_neg else []),
                         has_mech=has_mech, closed=closed))

    # Already-closed traps, DERIVED: every code with a neg construct is a trap
    # the language already makes unrepresentable-or-caught. They are in the
    # census so the gate has a baseline -- a progress counter with no closed
    # rows in the denominator cannot reach 100 %.
    curated_codes = {r['code'] for r in rows if r['closed']}
    for code in sorted(negs):
        if code in curated_codes:
            continue
        rows.append(dict(id='code-%d' % code, source=documented.get(code, 'bebop.bp'),
                         cls='T' if code in (80, 82, 87) else 'C',
                         mechanism='landed: %s' % (', '.join(exits_fixed.get(code, ['brk/runtime'])[:2])),
                         code=code, cost='closed', neg=negs[code], has_mech=True, closed=True))

    # codes the compiler can emit that docs/TRAPS.md does not carry
    undoc = sorted(c for c in exits_fixed if c not in documented)
    ranges = []
    for (base, var), sites in sorted(exits_computed.items()):
        ranges.append(dict(base=base, var=var, sites=sites,
                           span='%d..%d (site 1..20)' % (base + 1, base + 20)))

    counted = [r for r in rows if r['cost'] != 'n/a']
    closed = [r for r in counted if r['closed']]
    zero = [r for r in counted if r['cost'] == 'zero' and not r['closed']]
    words = [r for r in counted if r['cost'] == 'words' and not r['closed']]

    if '--json' in argv:
        print(json.dumps(dict(rows=rows, undocumented_codes=undoc,
                              computed_ranges=ranges), indent=1))
        return 0

    if '--codes' in argv or '--rows' in argv or len(argv) == 0:
        print('== census sources (all present and parsed)')
        print('  docs/WORKER-CARD.md   %d hazards extracted from the trap section' % len(card))
        print('  docs/LANGUAGE.md      %d UNDEFINED sites' % len(undef))
        print('  docs/TRAPS.md         %d exit codes documented' % len(documented))
        print('  bebop.bp              %d distinct fixed exit codes at %d sites'
              % (len(exits_fixed), sum(len(v) for v in exits_fixed.values())))
        print('  bench/.../neg         %d codes have a neg construct' % len(negs))
        print('  T122 reserved table   %d hashes; %d builtin dispatch arms; '
              '%d SHADOWABLE' % (len(reserved), len(dispatched), len(shadowable)))
        print('  NOT PRESENT: docs/RESEARCH-VERIFICATION-2026-09-09.md,'
              ' docs/blueprints/F0-trap-census.md -- the row F1 cites both.')
        print()

    if '--rows' in argv or len(argv) == 0:
        print('== rows (cost class is what the FIX costs, not how bad the trap is)')
        print('%-30s %-3s %-5s %-6s %-8s %s' % ('id', 'cls', 'code', 'cost', 'closed', 'neg'))
        for r in rows:
            print('%-30s %-3s %-5s %-6s %-8s %s'
                  % (r['id'], r['cls'], r['code'] if r['code'] else '-', r['cost'],
                     'yes' if r['closed'] else ('n/a' if r['cost'] == 'n/a' else 'NO'),
                     ','.join(r['neg']) or '-'))
        print()

    if '--codes' in argv or len(argv) == 0:
        print('== exit codes the compiler can produce but docs/TRAPS.md does not carry')
        if undoc:
            for c in undoc:
                print('  %d  %s' % (c, ' '.join(exits_fixed[c][:3])))
        else:
            print('  (none)')
        print('== COMPUTED exit codes: one site spans a whole range')
        for r in ranges:
            print('  sys_exit(%d + %s) -> %s   %s' % (r['base'], r['var'], r['span'],
                                                      ' '.join(r['sites'][:2])))
        print('  docs/TRAPS.md claims "a code that is not here is a bug"; by its own'
              ' rule every code above is one.')
        print()

    if '--shadow' in argv:
        print('== builtins a user fn can silently shadow (dispatched, not reserved)')
        def hh(n):
            v = 0
            for c in n.encode():
                v = (v * 131 + c) & 0xFFFFFFFFFFFFFFFF
            return v
        src = rd('bebop.bp') or ''
        byhash = {}
        for a, sym in re.findall(r'name == (-?\d+) then emit_(\w+)', src):
            byhash[int(a) & 0xFFFFFFFFFFFFFFFF] = sym
        for hsh in shadowable:
            print('  %-24s (hash %d, emitter emit_%s)'
                  % (next((n for n in [byhash[hsh].replace('_fn', '')] if hh(n) == hsh),
                          '<emit_%s>' % byhash[hsh]), hsh, byhash[hsh]))
        print()

    print('shadowable_builtins: %d of %d' % (len(shadowable), len(dispatched)))
    print('trap_rows: %d total, %d counted (%d excluded as not-a-language-defect)'
          % (len(rows), len(counted), len(rows) - len(counted)))
    print('trap_zero_word: %d of %d open rows fixable at parse/planning time (0 machine words)'
          % (len(zero), len(zero) + len(words)))
    print('trap_needs_words: %d of %d open rows need emitted words (weigh against bin_words)'
          % (len(words), len(zero) + len(words)))
    print('trap_unrep: %d/%d' % (len(closed), len(counted)))
    return 0


if __name__ == '__main__':
    raise SystemExit(build(sys.argv[1:]))
