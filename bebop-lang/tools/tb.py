#!/usr/bin/env python3
"""tb.py (2026-09-09) -- content-address a file, and find hits without printing the file.

WHY THIS EXISTS. `CLAUDE.md`'s token-economy table lists `tb` with the rule "re-read a file
only when its hash changed" and the usage `tb h <path>` / `tb s <needle> <path>`. Checked
2026-09-09: `tb` is not on PATH, and neither are `rtk`, `ponytail` or `headroom`; of the six
tools in a table headed "always on -- every session, verified 2026-09-04", one was present.
A rule that names a command nobody can run is not a rule, so the command is written here
rather than the rule being deleted.

WHAT IT IS FOR. The expensive thing in an agent session is not reading a file once, it is
reading it AGAIN to check whether it changed, and reading a whole file to find four lines.
`h` answers "did this change" in 8 bytes. `s` answers "where is this" in one line per hit.
Neither is clever; both are absent.

  tb h <path>...            crc32 (8 hex) + size + path, one line each
  tb s <needle> <path>...   LINE:text for each hit, needle is a fixed string not a regex
  tb s -e <re> <path>...    same, needle is a regex
  tb d <path> <crc>         exit 0 if the file still hashes to <crc>, exit 1 if it moved
  tb n <needle> <path>...   count only: path + hit count, prints no source at all

Exit codes: 0 ok; 1 changed (for `d`) or no hits (for `s`/`n`); 2 usage or unreadable.
`d` is the one that saves the most -- it is the whole "re-read only when the hash changed"
rule in one command, and it prints nothing on the common path.
"""
import re
import sys
import zlib


def crc(path):
    with open(path, "rb") as f:
        return zlib.crc32(f.read()) & 0xFFFFFFFF


def main(argv):
    if len(argv) < 2:
        sys.stderr.write(__doc__.split("\n\n")[-2] + "\n")
        return 2
    cmd, rest = argv[0], argv[1:]

    if cmd == "h":
        for p in rest:
            try:
                with open(p, "rb") as f:
                    b = f.read()
            except OSError as e:
                sys.stderr.write("tb: %s: %s\n" % (p, e.strerror))
                return 2
            print("%08x %9d %s" % (zlib.crc32(b) & 0xFFFFFFFF, len(b), p))
        return 0

    if cmd == "d":
        if len(rest) != 2:
            sys.stderr.write("tb d <path> <crc>\n")
            return 2
        try:
            got = crc(rest[0])
        except OSError as e:
            sys.stderr.write("tb: %s: %s\n" % (rest[0], e.strerror))
            return 2
        want = int(rest[1], 16)
        if got == want:
            return 0
        print("%08x != %08x %s" % (got, want, rest[0]))
        return 1

    if cmd in ("s", "n"):
        usere = False
        if rest and rest[0] == "-e":
            usere, rest = True, rest[1:]
        if len(rest) < 2:
            sys.stderr.write("tb %s [-e] <needle> <path>...\n" % cmd)
            return 2
        needle, paths = rest[0], rest[1:]
        rx = re.compile(needle) if usere else None
        total = 0
        for p in paths:
            try:
                with open(p, encoding="utf-8", errors="replace") as f:
                    lines = f.read().split("\n")
            except OSError as e:
                sys.stderr.write("tb: %s: %s\n" % (p, e.strerror))
                return 2
            hits = [(i + 1, l) for i, l in enumerate(lines)
                    if (rx.search(l) if rx else needle in l)]
            total += len(hits)
            if cmd == "n":
                print("%6d %s" % (len(hits), p))
            else:
                for ln, text in hits:
                    prefix = ("%s:" % p) if len(paths) > 1 else ""
                    print("%s%d:%s" % (prefix, ln, text))
        return 0 if total else 1

    sys.stderr.write("tb: unknown command %r (h, s, n, d)\n" % cmd)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
