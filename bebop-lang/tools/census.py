#!/usr/bin/env python3
"""
census.py — branch census per .bin (T40 ii / T51 gate).

  python3 tools/census.py <a.bin> [...]                 # print table rows
  python3 tools/census.py --check census.txt <a.bin>... # fail on any INCREASE

Row: name words bcond cbz tbz b bl ret  (words = code words, data cells
excluded). Gate columns are the conditional branches bcond/cbz/tbz: an
increase vs the frozen table fails; a decrease prints DECREASE (re-freeze
the table in the rung that earned it). Rows must match the table 1:1.
"""

import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from check_abi import load_bin

COLS = ("words", "bcond", "cbz", "tbz", "b", "bl", "ret")
GATE = ("bcond", "cbz", "tbz")


def census(path):
    W, _, end = load_bin(path)
    c = dict.fromkeys(COLS, 0)
    c["words"] = end
    for w in W[:end]:
        if (w & 0xFF000010) == 0x54000000: c["bcond"] += 1
        elif (w & 0x7E000000) == 0x34000000: c["cbz"] += 1
        elif (w & 0x7E000000) == 0x36000000: c["tbz"] += 1
        elif (w & 0xFC000000) == 0x14000000: c["b"] += 1
        elif (w & 0xFC000000) == 0x94000000: c["bl"] += 1
        elif w == 0xD65F03C0: c["ret"] += 1
    return c


def row(name, c):
    return " ".join([name] + [str(c[k]) for k in COLS])


def read_table(path):
    t = {}
    for line in open(path):
        p = line.split()
        if p and not p[0].startswith("#"):
            t[p[0]] = dict(zip(COLS, map(int, p[1:])))
    return t


def main(argv):
    if argv[:1] == ["--freeze-check"]:
        # D11-F: a re-freeze may raise bcond/cbz/tbz of a bin ONLY if census_allow.txt
        # (committed with the change) carries `<bin> <col> <new_value> <reason...>`.
        table, allow, rc = read_table(argv[1]), {}, 0
        for line in open(argv[2]):
            if line.strip() and not line.startswith("#"):
                b, k, v = line.split()[:3]; allow[(b, k)] = int(v)
        for path in argv[3:]:
            name = os.path.basename(path).rsplit(".", 1)[0]
            c = census(path)
            if name not in table:
                continue  # a new bin is allowed (it gets its first row)
            for k in GATE:
                if c[k] > table[name][k] and allow.get((name, k)) != c[k]:
                    print(f"CENSUS FREEZE REFUSED {name}: {k} {table[name][k]} -> {c[k]} without a census_allow.txt line `{name} {k} {c[k]} <reason>`"); rc = 1
        if rc == 0:
            print("census freeze: every increase is covered by census_allow.txt")
        return rc
    if argv[:1] == ["--check"]:
        table, rc = read_table(argv[1]), 0
        seen = set()
        for path in argv[2:]:
            name = os.path.basename(path).rsplit(".", 1)[0]
            seen.add(name)
            c = census(path)
            if name not in table:
                print(f"CENSUS FAIL {name}: not in table -> {row(name, c)}"); rc = 1; continue
            old = table[name]
            for k in GATE:
                if c[k] > old[k]:
                    print(f"CENSUS FAIL {name}: {k} {old[k]} -> {c[k]} (INCREASE)")
                    # item 4 (retro D13): the ready-to-paste census_allow.txt candidate line
                    print(f"  candidate census_allow.txt line: {name} {k} {c[k]} <reason>")
                    rc = 1
                elif c[k] < old[k]:
                    print(f"CENSUS DECREASE {name}: {k} {old[k]} -> {c[k]} (re-freeze table)")
        for name in sorted(set(table) - seen):
            print(f"CENSUS FAIL {name}: in table but not measured"); rc = 1
        if rc == 0:
            print(f"census: {len(seen)} bins, no conditional-branch increase")
        return rc
    print("# " + " ".join(("name",) + COLS) + "  (tools/census.py; gate = bcond/cbz/tbz never increase)")
    for path in argv:
        print(row(os.path.basename(path).rsplit(".", 1)[0], census(path)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
