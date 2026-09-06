#!/usr/bin/env python3
"""check_words.py (item 7, retro D13/L1 mechanised): every numeric literal >= 0x1000 that is
NEW in `git diff HEAD -- bebop.bp`, inside an `em(insns, n, N)` or `st[i] = N` call, must
appear (decimal or 0x hex) in $BEBOP_TMP/words.objdump -- the `as` + `objdump -d` listing the
author produces BEFORE editing (L1: asm -> objdump -> script -> LE int -> scripted insert).
No bebop.bp diff, or no new literal >= 0x1000, is a pass. Run by tools/battery.sh.

Usage: tools/check_words.py [DIFF_FILE]
  DIFF_FILE: unified diff text to scan instead of `git diff HEAD -- bebop.bp` (for a scratch
  test against a copy of bebop.bp, e.g. `git diff --no-index old.bp new.bp > DIFF_FILE`).
env: BEBOP_TMP (default /tmp/opencode)
"""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
LIT = re.compile(r'\bem\(\s*insns\s*,\s*n\s*,\s*(-?\d+)\s*\)|\bst\[[^\]]*\]\s*=\s*(-?\d+)\s*;')


def new_literals(diff_text):
    lits = []
    for line in diff_text.splitlines():
        if not line.startswith('+') or line.startswith('+++'):
            continue
        for m in LIT.finditer(line):
            n = int(m.group(1) if m.group(1) is not None else m.group(2))
            if abs(n) >= 0x1000:
                lits.append(n)
    return lits


def verified(n, text):
    dec = str(n)
    hx = format(n & 0xFFFFFFFF, 'x')
    tl = text.lower()
    return re.search(r'\b%s\b' % re.escape(dec), text) or re.search(r'\b%s\b' % re.escape(hx), tl)


def main(argv):
    if argv:
        diff_text = open(argv[0]).read()
    else:
        diff_text = subprocess.run(['git', 'diff', 'HEAD', '--', 'bebop.bp'], cwd=ROOT,
                                    capture_output=True, text=True).stdout
    lits = new_literals(diff_text)
    if not lits:
        print("words: PASS (no bebop.bp diff, or no new em()/st[] literal >= 0x1000)")
        return 0
    tmp = os.environ.get('BEBOP_TMP', '/tmp/opencode')
    obj = os.path.join(tmp, 'words.objdump')
    if not os.path.exists(obj):
        print(f"words: FAIL {len(lits)} new literal(s) {lits} but {obj} is missing -- "
              f"recipe (L1): as the word, `objdump -d` it into $BEBOP_TMP/words.objdump, THEN edit bebop.bp")
        return 1
    text = open(obj).read()
    missing = [n for n in lits if not verified(n, text)]
    if missing:
        print(f"words: FAIL {len(missing)} unverified literal(s) {missing} not found (decimal or hex) in {obj} -- "
              f"recipe (L1): as the word, `objdump -d` it into $BEBOP_TMP/words.objdump, THEN edit bebop.bp")
        return 1
    print(f"words: PASS ({len(lits)} new literal(s) verified against {obj})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
