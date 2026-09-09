#!/usr/bin/env python3
"""undef_census.py -- ROADMAP F1. Count the undefined-behaviour sites.

Two numbers, and they are not the same kind of number. The script says which is
exact and which is not, because F1's whole point is that a count nobody can
audit is worth nothing.

  undef_census: n   EXACT. Occurrences of UNDEFINED in docs/LANGUAGE.md, each
                    printed with its line. Target 0: every one of them is a
                    place the language permits a program whose meaning is
                    "whatever the register holds".

  gen_avoid: n      PROVISIONAL, and deliberately so. The F1 row asks for "the
                    `avoid` shapes in bench/fuzz/gen.py". Measured 2026-09-09:
                    the string `avoid` occurs ZERO times in bench/fuzz/gen.py.
                    It occurs in docs/LANGUAGE.md:55 -- "the fuzzer avoids that
                    shape and so should you" -- so the row's gate points at the
                    file that is claimed ABOUT, not the file that does it. The
                    avoidance in gen.py is structural (a variable excluded from
                    a candidate list, a guard on a generated shape), not lexical,
                    so no grep can count it honestly. This script therefore
                    lists CANDIDATE sites and refuses to present the number as
                    exact. The fix is one line of convention, not more grepping:
                    mark each such guard in gen.py with `# LANG-AVOID: <hazard>`
                    and this counter becomes exact the day it lands. gen.py is
                    not this row's file to edit.

Usage: python3 tools/undef_census.py [--list]
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Comment vocabulary that, in this repository, marks a generator guard placed
# because the LANGUAGE cannot express the shape safely -- as opposed to a guard
# placed for generator convenience. Each hit is printed so it can be judged.
AVOID_HINTS = re.compile(
    r'never assigned|would be dead in bebop|unbound in bpref|not registered as an index'
    r'|looped forever|indices stay masked', re.I)


def die(msg):
    sys.stderr.write('undef_census: %s\n' % msg)
    raise SystemExit(2)


def read(rel):
    p = os.path.join(ROOT, rel)
    if not os.path.exists(p):
        die('missing %s -- refusing to print a count for a file that is not there' % rel)
    with open(p, encoding='utf-8', errors='replace') as f:
        return f.read().split('\n')


def main(argv):
    show = '--list' in argv

    lang = read('docs/LANGUAGE.md')
    undef = [(i, l.strip()) for i, l in enumerate(lang, 1) if 'UNDEFINED' in l]

    gen = read('bench/fuzz/gen.py')
    lexical = [(i, l.strip()) for i, l in enumerate(gen, 1) if 'avoid' in l.lower()]
    cand = [(i, l.strip()) for i, l in enumerate(gen, 1)
            if l.lstrip().startswith('#') and AVOID_HINTS.search(l)]
    marked = [(i, l.strip()) for i, l in enumerate(gen, 1) if 'LANG-AVOID:' in l]

    if not lang or not gen:
        die('a source file read empty -- fail loudly rather than print 0')

    print('== docs/LANGUAGE.md UNDEFINED sites (EXACT)')
    if undef:
        for i, l in undef:
            print('  docs/LANGUAGE.md:%d  %s' % (i, l[:110]))
    else:
        print('  (none -- target reached)')

    print('== bench/fuzz/gen.py hazard-avoidance guards')
    print('  lexical hits for "avoid": %d  <- the F1 gate\'s premise; it is 0 today,'
          ' so the gate as worded cannot move' % len(lexical))
    print('  explicit `# LANG-AVOID:` markers: %d (the exact form, once gen.py adopts it)'
          % len(marked))
    print('  candidate structural guards: %d (PROVISIONAL, listed below for judgement)'
          % len(cand))
    if show or not marked:
        for i, l in cand:
            print('    bench/fuzz/gen.py:%d  %s' % (i, l[:110]))

    print()
    print('undef_census: %d' % len(undef))
    if marked:
        print('gen_avoid: %d' % len(marked))
    else:
        print('gen_avoid: %d PROVISIONAL (no `# LANG-AVOID:` markers in gen.py; '
              'the count is a human judgement until they land)' % len(cand))
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
