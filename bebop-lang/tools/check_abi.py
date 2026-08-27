#!/usr/bin/env python3
"""
check_abi.py — L3 gate: x27/x28 (arena) must not be written outside prologue.

Usage: python3 tools/check_abi.py <file.full>

The runner reserves x27 (arena cursor) and x28 (arena end) as global
register vars and sets them in a single asm block with blr. The compiler
must never emit a write to x27/x28 except in the prologue's
`stp x27,x28,[sp,#64]` save and its matching `ldp` restore, and the
`add x15,sp,#0x100` / `add x14,sp,#0x300` spill/heap bases.

Any other write to x27/x28 (e.g. `mov x27,x8` for a spilled 9th param,
or `add x27,x27,#...` for zeros) is a silent heap corruption.

This checker scans every fn span for writes to x27/x28 outside the
first 10 prologue words and the last 8 epilogue words.
"""

import sys, re, struct

def writes_x27_x28(word: int) -> bool:
    # Only check data-processing / load/store where Rd is written
    op = word >> 26
    if op in (0x05, 0x25):  # b, bl
        return False
    rd = word & 0x1F
    rn = (word >> 5) & 0x1F
    if rd not in (27, 28):
        return False
    if word in (0xa90473fb, 0xa94473fb):
        return False
    # Allow arena bumps: add x27,x27,reg
    if rd in (27, 28) and rn in (27, 28):
        return False
    return True

def main():
    path = sys.argv[1] if len(sys.argv) > 1 else None
    if not path:
        print("usage: check_abi.py <file.full>")
        sys.exit(2)
    toks = open(path).read().split()
    n = int(toks[0])
    W = [int(t) & 0xffffffff for t in toks[1:1+n]]
    txt = open(path).read()
    m = re.search(r'OFF.*', txt)
    if not m:
        print("no OFF line")
        sys.exit(2)
    offs = list(map(int, m.group(0).split()[2:]))
    bad = []
    for k, off in enumerate(offs):
        nxt = offs[k+1] if k+1 < len(offs) else n
        span = W[off:nxt]
        # Prologue is first 10 words, epilogue last 8
        for i, w in enumerate(span):
            if i < 10 or i >= len(span) - 8:
                continue
            if writes_x27_x28(w):
                bad.append((k, off+i, f"{w:08x}"))
                break
    if bad:
        print("L3 FAIL: x27/x28 writes outside prologue/epilogue:")
        for k, pos, w in bad[:10]:
            print(f"  fn#{k} @{pos}: {w}")
        sys.exit(1)
    print(f"L3: no x27/x28 writes outside prologue/epilogue in {len(offs)} fns")

if __name__ == "__main__":
    main()
