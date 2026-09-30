#!/usr/bin/env python3
"""check_words.py (item 7, retro D13/L1 mechanised): every numeric literal >= 0x1000 that is
NEW in `git diff HEAD -- bebop.bp` (since the 2026-09-29 split: in the diff of the WHOLE compiler
text -- bebop.bp + compiler/*.bp concatenated in codegen order, HEAD vs the tree -- so a word added
to an included file is seen, and a block MOVED between files is not a new word), inside an `em(insns, n, N)` or `st[i] = N` call, must
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
sys.path.insert(0, HERE)
import bp_src


def git_show(rel):
    r = subprocess.run(['git', 'show', 'HEAD:./' + rel], cwd=ROOT, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ''


def head_text():
    """HEAD's compiler text: HEAD's bebop.bp, the compiler/*.bp ITS use lines name (as they are at
    HEAD; a file new since HEAD is empty there), concatenated in the same order as bp_src.text."""
    main = git_show('bebop.bp')
    seen, out = set(), []
    for rel in bp_src.USE.findall(main):
        if rel in seen: continue
        seen.add(rel)
        out.append(git_show(rel))
    return ''.join(out) + main


def compiler_diff(old_text, new_text):
    import difflib
    return ''.join(difflib.unified_diff(old_text.splitlines(True), new_text.splitlines(True),
                                        'a/compiler', 'b/compiler'))


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
    elif os.environ.get('WORDS_BASE'):
        # a lane tree has no .git: diff against the base bebop.bp the lane runner hands us, no git at all
        base = os.path.abspath(os.environ['WORDS_BASE'])
        diff_text = compiler_diff(bp_src.text(os.path.dirname(base), os.path.basename(base)), bp_src.text(ROOT))
    elif subprocess.run(['git', 'ls-files', '--error-unmatch', 'bebop.bp'], cwd=ROOT,
                        capture_output=True, text=True).returncode == 0:
        # TRACKEDNESS, not a local .git: bebop-lang/ is a SUBDIRECTORY of the repo at
        # /root/dowiz, so it has no .git of its own and an isdir() test rejects the real
        # tree. A lane tree fails this test for the right reason -- .gitignore lists
        # .claude/lanes/, so bebop.bp there is untracked and the diff would be empty.
        diff_text = compiler_diff(head_text(), bp_src.text(ROOT))
    else:
        # 2026-09-13 (battery audit): from a lane tree `git` walked up to the MAIN repo, whose .gitignore
        # lists .claude/lanes/, so `git diff HEAD -- bebop.bp` was EMPTY and this printed PASS in every
        # lane battery while measuring nothing (and took a read lock on the main index to do it).
        print(f"words: NOT MEASURED -- {ROOT} is not a git repository; pass a diff file, or WORDS_BASE=<the base bebop.bp> to diff against")
        return 2
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
