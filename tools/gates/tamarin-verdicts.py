#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The Tamarin verdicts, held against the committed list. A falsified lemma is in the prover's
output and not in its exit code, so without this a change of verdict is a green job.

    python3 tools/gates/tamarin-verdicts.py <prover output> [--write <file>]

Every lemma of docs/design/tamarin/bebop_seal.spthy must have a verdict in the summary, or this
exits 2: a parse error, or a prover that stopped early, has no summary to compare.

docs/design/tamarin/expected-verdicts.txt holds `lemma verdict` per line. WHILE IT IS ABSENT there
is nothing to compare: the verdicts are printed, the run says so, and it exits 0. The list is never
written by hand or from the README's table, which is a prediction: it is the `--write` file of a
real run, `tamarin.verdicts` in the `tamarin-out` artifact, committed after someone has read it.

Exit 0 green or no list yet; 1 a verdict moved; 2 no verdict for every lemma."""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MODEL = os.path.join(HERE, "..", "..", "docs", "design", "tamarin")
LINE = re.compile(r"^\s*(\w+) \((?:all-traces|exists-trace)\): (verified|falsified|analysis incomplete)\b")


def lemmas(spthy):
    return re.findall(r"^lemma\s+(\w+)", open(spthy).read(), re.M)


def verdicts(out):
    """The summary's verdicts. The prover prints a summary per file and repeats it, so the last wins."""
    got = {}
    for x in out.splitlines():
        m = LINE.match(x)
        if m:
            got[m.group(1)] = m.group(2).replace(" ", "-")
    return got


def read_list(path):
    rows = [x.split() for x in open(path) if x.strip() and not x.startswith("#")]
    return {k: v for k, v in rows}


def main(argv):
    if len(argv) not in (2, 4) or (len(argv) == 4 and argv[2] != "--write"):
        sys.exit(__doc__)
    model = os.environ.get("TAMARIN_MODEL", MODEL)
    names = lemmas(os.path.join(model, "bebop_seal.spthy"))
    got = verdicts(open(argv[1], errors="replace").read())
    for k in names:
        print("tamarin: %-40s %s" % (k, got.get(k, "NO VERDICT")))
    if len(argv) == 4:
        with open(argv[3], "w") as f:
            f.writelines("%s %s\n" % (k, got[k]) for k in names if k in got)
    missing = [k for k in names if k not in got]
    if not names or missing:
        print("::error::tamarin: %d of %d lemmas have no verdict (a parse error or an early stop): this is no result" % (len(missing), len(names)))
        return 2
    expected = os.path.join(model, "expected-verdicts.txt")
    if not os.path.exists(expected):
        print("::warning::tamarin: no expected-verdicts.txt, so these %d verdicts are compared with nothing: no verdict has been pinned yet" % len(names))
        return 0
    want = read_list(expected)
    moved = ["%s: expected %s, got %s" % (k, want.get(k, "(not listed)"), got.get(k, "(no such lemma)")) for k in sorted(set(want) | set(names)) if want.get(k) != got.get(k)]
    for m in moved:
        print("::error::tamarin: verdict moved: " + m)
    print("tamarin: %d lemmas, %d moved" % (len(names), len(moved)))
    return 1 if moved else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
