#!/usr/bin/env python3
"""builtin_surface.py -- the language's BUILTIN surface, four ways, as a number.

Phase F prerequisite (2026-09-09). A builtin that exists in bebop.bp but not in
tools/typecheck.py's BUILTIN table is invisible to EVERY typecheck rung, and
silently so: the checker types the call `?` and no rung ever fires. That is how
`scan` (the 46-word scalar builtin A9 landed) sat outside the type checker.
Fixing `scan` fixes one name; this script closes the hole as a CLASS.

The authority is the COMPILER, not any list: emit_call_or_ctor dispatches every
builtin on the 131-rolling hash of its name (read_ident), so the set of
`name == <hash> then emit_<f>(` arms in bebop.bp IS the language's builtin
surface, and nothing can be added to the language without appearing there.

Names are recovered from those arms by HASH, never by string-munging the emitter
symbol: the symbol only proposes candidates (`emit_sys_scan` -> sys_scan, scan;
`emit_str_len_fn` -> str_len_fn, str_len; `emit_char_fn` -> char), and
read_ident's own hash decides which one is real. An arm whose hash matches no
candidate is reported UNRESOLVED and is a failure -- that is the case where the
compiler dispatches a name none of the four sources has ever heard of.

Compared against, in order:
  compiler   bebop.bp emit_call_or_ctor dispatch arms          (authority)
  typecheck  tools/typecheck.py BUILTIN keys                   (types every call)
  bpref      tools/bpref.py builtin_or_call, split into
             IMPLEMENTED / STUBBED-to-0 / ABSENT               (the oracle)
  language   docs/LANGUAGE.md "## Builtins" table              (the spec)
plus one informational fifth: bebop.bp's own T122 reserved-word table, which
rejects `fn <builtin>` -- a builtin missing THERE can be shadowed by a user fn.

Last line is the count known to all four. Exit 1 unless that equals the
compiler's own count, so a future builtin added to bebop.bp alone is RED here.

Usage: python3 tools/builtin_surface.py [--quiet]   (run from the repo root)
"""
import re
import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
M64 = (1 << 64) - 1


def read(rel):
    p = os.path.join(ROOT, rel)
    with open(p, encoding='utf-8', errors='replace') as f:
        t = f.read()
    if not t.strip():
        sys.exit('builtin_surface: %s is empty -- refusing to report a surface from nothing' % rel)
    return t


def h131(name):
    """read_ident's hash (bebop.bp): h = h*131 + byte, wrapped to 64 bits."""
    h = 0
    for ch in name.encode():
        h = (h * 131 + ch) & M64
    return h


def unsigned(v):
    return v & M64


# ---- (1) the compiler: the dispatch arms of emit_call_or_ctor ---------------
def compiler_set(src):
    arms = re.findall(r'name == (-?\d+) then (emit_\w+)\(', src)
    if not arms:
        sys.exit('builtin_surface: found NO dispatch arms in bebop.bp -- the '
                 'shape of emit_call_or_ctor changed and this gate is blind; fix the pattern')
    return [(unsigned(int(h)), f) for h, f in arms]


def candidates(sym):
    """Candidate builtin names for an emitter symbol. The hash picks the winner."""
    base = sym[len('emit_'):]
    out = {base}
    if base.endswith('_fn'):
        out.add(base[:-3])
    if base.startswith('sys_'):
        out.add(base[4:])                      # emit_sys_scan -> scan
        if base[4:].endswith('_fn'):
            out.add(base[4:-3])
    return out


# ---- (2) typecheck, (3) bpref, (4) LANGUAGE.md ------------------------------
def typecheck_set():
    """Read the BUILTIN keys TEXTUALLY. Importing typecheck.py runs its module
    body, which prints a census -- a gate whose last line is a number must not
    have another tool's output interleaved into it."""
    src = read('tools/typecheck.py')
    i = src.index('BUILTIN = {')
    depth, j = 0, i + len('BUILTIN = ')
    for k in range(j, len(src)):
        if src[k] == '{':
            depth += 1
        elif src[k] == '}':
            depth -= 1
            if depth == 0:
                j = k + 1
                break
    body = src[i:j]
    keys = set(re.findall(r"'([a-z0-9_]+)'\s*:\s*\(", body))
    if not keys:
        sys.exit('builtin_surface: could not read BUILTIN out of tools/typecheck.py')
    return keys


def bpref_split(src):
    """IMPLEMENTED = an explicit `if name == 'x'` arm in builtin_or_call;
    STUBBED = swept up by the `clock_ms or name.startswith('sys_')` -> 0 arm."""
    i = src.index('def builtin_or_call')
    body = src[i:src.index('\n    def ', i + 10) if '\n    def ' in src[i + 10:] else len(src)]
    stub_all_sys = "name.startswith('sys_')" in body
    # a name that appears in the `name == 'x' or name.startswith('sys_')` arm is a
    # STUB (that arm returns 0), not an implementation -- clock_ms is the only one,
    # and counting it as implemented is what makes the impl total read 10 instead
    # of the arithmetically forced 9 (9 impl + 25 stub + 2 absent = 36).
    stub_named = set(re.findall(r"name == '([a-z0-9_]+)' or name\.startswith", body))
    impl = set(re.findall(r"if name == '([a-z0-9_]+)'", body)) - stub_named
    return impl, stub_all_sys, stub_named


def language_set(src):
    i = src.index('## Builtins')
    body = src[i:src.index('\n## ', i + 5)]
    return set(re.findall(r'`([a-z0-9_]+)\s*\(', body)), body


def reserved_set(src):
    """T122: bebop.bp rejects `fn <keyword-or-builtin>` (exit 99) by hash."""
    hs = set()
    for m in re.finditer(r'let rsv\d = (.+?);\n', src):
        hs |= {unsigned(int(x)) for x in re.findall(r'selfname == (-?\d+)', m.group(1))}
    return hs


def main():
    quiet = '--quiet' in sys.argv
    bebop = read('bebop.bp')
    bpref_src = read('tools/bpref.py')
    lang_src = read('docs/LANGUAGE.md')

    arms = compiler_set(bebop)
    tc = typecheck_set()
    impl, stub_all_sys, stub_named = bpref_split(bpref_src)
    lang, lang_body = language_set(lang_src)
    rsv = reserved_set(bebop)

    # the candidate pool: every name any source knows, plus what the emitter symbols suggest
    pool = set(tc) | impl | lang | stub_named
    for _, sym in arms:
        pool |= candidates(sym)
    by_hash = {}
    for nm in pool:
        by_hash.setdefault(h131(nm), nm)

    resolved, unresolved = [], []
    for hv, sym in arms:
        nm = by_hash.get(hv)
        (resolved.append((nm, sym, hv)) if nm else unresolved.append((hv, sym)))
    resolved.sort()

    rows, complete = [], 0
    for nm, sym, hv in resolved:
        in_tc = nm in tc
        if nm in impl:
            bp = 'impl'
        elif nm == 'clock_ms' or (stub_all_sys and nm.startswith('sys_')):
            bp = 'stub0'
        else:
            bp = 'ABSENT'
        in_lang = nm in lang
        ok = in_tc and bp != 'ABSENT' and in_lang
        complete += ok
        rows.append((nm, sym, in_tc, bp, in_lang, h131(nm) in rsv, ok))

    if not quiet:
        print('== builtin surface, four ways (authority: bebop.bp emit_call_or_ctor) ==')
        print('%-24s %-26s %-9s %-7s %-8s %-8s' % ('builtin', 'emitter', 'typecheck', 'bpref', 'LANGUAGE', 'reserved'))
        for nm, sym, in_tc, bp, in_lang, in_rsv, ok in rows:
            print('%-24s %-26s %-9s %-7s %-8s %-8s %s'
                  % (nm, sym, 'yes' if in_tc else 'MISSING', bp,
                     'yes' if in_lang else 'MISSING', 'yes' if in_rsv else 'MISSING',
                     '' if ok else '<-- gap'))
        for hv, sym in unresolved:
            print('UNRESOLVED hash %d dispatched to %s -- no source names it' % (hv, sym))
        extra_tc = sorted(tc - {r[0] for r in rows})
        extra_lang = sorted(lang - {r[0] for r in rows})
        if extra_tc:
            print('typecheck BUILTIN keys the compiler does NOT dispatch: %s' % ', '.join(extra_tc))
        if extra_lang:
            print('LANGUAGE.md names the compiler does NOT dispatch: %s' % ', '.join(extra_lang))
        dup = [n for n in set(re.findall(r'`([a-z0-9_]+)\s*\(', lang_body))
               if len(re.findall(r'`%s\s*\(' % re.escape(n), lang_body)) > 1]
        if dup:
            print('LANGUAGE.md Builtins lists these MORE THAN ONCE: %s' % ', '.join(sorted(dup)))
        print('compiler dispatches %d, resolved %d, unresolved %d; typecheck %d, bpref impl %d / stub %d / absent %d, LANGUAGE %d'
              % (len(arms), len(resolved), len(unresolved), sum(r[2] for r in rows),
                 sum(r[3] == 'impl' for r in rows), sum(r[3] == 'stub0' for r in rows),
                 sum(r[3] == 'ABSENT' for r in rows), sum(r[4] for r in rows)))

    if unresolved:
        print('builtins known to all four: %d of %d' % (complete, len(arms)))
        sys.exit(1)
    print('builtins known to all four: %d of %d' % (complete, len(arms)))
    sys.exit(0 if complete == len(arms) else 1)


main()
