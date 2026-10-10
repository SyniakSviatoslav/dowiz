#!/usr/bin/env python3
"""bp_prune.py -- flatten a .bp program's `use` includes and drop the top-level fns `main` cannot reach.

W-BATRED (2026-10-10). Why it exists: a gate program that embeds the WHOLE compiler (a main-stripped
bebop.bp concatenated ahead of a driver, std_golden's gb_pool block) carries every compiler fn whether
the driver calls it or not, and the compiler refuses a program of 768+ fns (E104, the cap is tied to
the fntab zone layout, bebop.bp collect_fns). R2 (compiler/tre.bp, +23 fns) and L4 (compiler/emit1.bp,
+15) took gb_pool_build.bp from under the cap to 800 fns, so gb_pool stopped COMPILING (E104) and
std_golden reported it as `EMPTY(ran, printed nothing)` from a stale last_rc. Dead fns are not what
that gate tests; the pool dispatch is.

Semantics kept: `use "P"` is expanded exactly as bebop.bp's use_expand does it -- depth first, each path
once, includes ahead of the including text, the `use` line itself commented out. Reachability is
over identifier tokens outside comments; identifiers inside STRING literals count too (an
over-approximation: a fn named only in a string is kept). Everything that is not a top-level fn
(module/struct/enum lines, comments) is kept verbatim. A fn whose name is defined twice is a
refusal (exit 3), not a guess.

Usage: bp_prune.py <in.bp> <out.bp> [--root main]   (resolves `use` paths against the cwd, like the
compiler). Prints `bp_prune: kept K of N fns (dropped D)` on stderr. Exit 2 = an include is missing.
"""
import re
import sys

USE = re.compile(r'^use "([^"\n]*)"')
IDENT = re.compile(r'[A-Za-z_][A-Za-z0-9_]*')
FN = re.compile(r'fn\s+([A-Za-z_][A-Za-z0-9_]*)')


def expand(text, seen, out):
    """bebop.bp use_scan/use_one/use_comment: deps first, once each, use lines commented."""
    for line in text.split('\n'):
        m = USE.match(line)
        if m and m.group(1) not in seen:
            seen.add(m.group(1))
            try:
                inc = open(m.group(1)).read()
            except OSError as e:
                sys.stderr.write('bp_prune: open failed: %s (%s)\n' % (m.group(1), e))
                sys.exit(2)
            expand(inc, seen, out)
    out.append('\n'.join('//' + l[2:] if USE.match(l) else l for l in text.split('\n')) + '\n')


def items(src):
    """Split into top-level items: ('fn', name, text) or ('other', None, text). Brace depth is
    counted outside `//` comments and string literals (no char literals in the language)."""
    res, i, n, start, depth, kind, name = [], 0, len(src), 0, 0, None, None
    bol = True
    while i < n:
        c = src[i]
        if c == '/' and src.startswith('//', i):
            j = src.find('\n', i)
            i = n if j < 0 else j
            continue
        if c == '"':
            i += 1
            while i < n and src[i] != '"':
                i += 2 if src[i] == '\\' else 1
            i += 1
            bol = False
            continue
        if depth == 0 and bol and kind is None:
            m = FN.match(src, i)
            if m and src.startswith('fn', i):
                if i > start:
                    res.append(('other', None, src[start:i]))
                start, kind, name = i, 'fn', m.group(1)
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0 and kind == 'fn':
                j = src.find('\n', i)
                end = n if j < 0 else j + 1
                res.append(('fn', name, src[start:end]))
                start, kind, name, i, bol = end, None, None, end, True
                continue
        bol = c == '\n'
        i += 1
    if start < n:
        res.append(('fn' if kind else 'other', name, src[start:]))
    return res


def refs(text):
    return set(IDENT.findall(re.sub(r'//[^\n]*', '', text)))


def main(argv):
    if len(argv) < 3:
        sys.stderr.write(__doc__)
        return 2
    root = argv[argv.index('--root') + 1] if '--root' in argv else 'main'
    out = []
    expand(open(argv[1]).read(), set(), out)
    its = items(''.join(out))
    fns = {}
    for k, name, text in its:
        if k == 'fn':
            if name in fns:
                sys.stderr.write('bp_prune: fn %s defined twice -- refusing to choose\n' % name)
                return 3
            fns[name] = text
    if root not in fns:
        sys.stderr.write('bp_prune: no fn %s\n' % root)
        return 3
    live, todo = set(), [root] + [r for k, _, t in its if k == 'other' for r in refs(t) if r in fns]
    while todo:
        f = todo.pop()
        if f in live:
            continue
        live.add(f)
        todo.extend(r for r in refs(fns[f]) if r in fns and r not in live)
    with open(argv[2], 'w') as w:
        w.write(''.join(t for k, name, t in its if k == 'other' or name in live))
    sys.stderr.write('bp_prune: kept %d of %d fns (dropped %d)\n' % (len(live), len(fns), len(fns) - len(live)))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
