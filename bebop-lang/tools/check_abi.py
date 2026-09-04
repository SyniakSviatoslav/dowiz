#!/usr/bin/env python3
"""
check_abi.py — structural invariants over emitted .bin artifacts (T40).

Usage:
  python3 tools/check_abi.py <a.bin> [b.bin ...]      # (i) register zones + (iv) footer/entry identity
  python3 tools/check_abi.py --fntab bebop.bp [x.bp]  # (iii) fntab zone map + literal-count trap

.bin format (seed/seed.S): LE32 code words, then string-literal data cells,
then LE64 entry BYTE offset as the last 8 bytes. Every fn starts with the
emit_prologue signature (stp x29,x30,[sp,#-16]! ; mov x29,sp) and ends with
ret; the code region ends at the last ret, data cells follow.

(i) register-zone law:
  x27/x28 (arena cursor/end) are written only by the prologue/epilogue and
  the arena bump `add x27,x27,<reg>`.
  x9-x13 (T25 bank) are written only by the prologue/epilogue, plus two
  DOCUMENTED allowlists that vanish when T25 S1/S2 land:
    sys     - scratch words of the emit_sys_* builtin emitters (parsed from
              bebop.bp `em(insns, n, <word>)` constants inside those fns);
    argpass - `ldr x9..x13,[sp]` call-site pops of parameters 9-13
              (bebop.bp emit_call: `pop(insns, n, i, fntab)` for i >= 9).
  Everything else is a violation.
(iv) footer/entry identity (L11/L12): size >= 16, size % 4 == 0, entry
  byte offset % 4 == 0, entry inside the code region and at a prologue,
  every fn span ends with ret, no fn starts inside the data cells.

Decoder: minimal AArch64 register-write classifier by op0 (bits 28-25);
SIMD/FP words are ignored (no GPR write tracked). No external disassembler.
"""

import os, re, struct, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROLOGUE = (0xA9BF7BFD, 0x910003FD)  # stp x29,x30,[sp,#-16]! ; mov x29,sp
RET = 0xD65F03C0
PRO_N, EPI_N = 10, 8                   # emit_prologue / emit_epilogue word counts
ARGPASS = {0xF94003E0 + r for r in range(9, 14)} | {0xAA0003E0 + r for r in range(9, 14)}
# ldr x9..x13,[sp] (canonical pop into an arg register) and mov x9..x13,x0 (the same
# pop after T96 step-1 elision of its own push) -- both vanish when T25 S1 lands


def load_bin(path):
    """-> (words, entry_word_index, code_end). Raises ValueError on a broken footer."""
    b = open(path, "rb").read()
    if len(b) < 16 or len(b) % 4:
        raise ValueError(f"size {len(b)} (need >= 16 and a multiple of 4)")
    e = struct.unpack("<Q", b[-8:])[0]
    W = list(struct.unpack(f"<{(len(b) - 8) // 4}I", b[:-8]))
    if e % 4 or e // 4 >= len(W):
        raise ValueError(f"entry byte offset {e} outside code ({len(W)} words)")
    rets = [i for i, w in enumerate(W) if w == RET]
    if not rets:
        raise ValueError("no ret word")
    return W, e // 4, rets[-1] + 1


def fn_starts(W, code_end):
    return [i for i in range(code_end - 1) if (W[i], W[i + 1]) == PROLOGUE]


def writes(w):
    """GPRs written by word w (x0..x30; 31 = sp/xzr)."""
    op0 = (w >> 25) & 0xF
    if op0 in (8, 9):                      # DP immediate, adr/adrp
        return [w & 31]
    if op0 in (0xA, 0xB):                  # branches / system
        if (w >> 26) == 0x25 or (w & 0xFFFFFC1F) == 0xD63F0000:  # bl / blr
            return [30]
        return []
    if op0 in (5, 0xD):                    # DP register
        if (w & 0x1FE00000) == 0x1A400000:  # ccmp/ccmn
            return []
        return [w & 31]
    if op0 in (4, 6, 0xC, 0xE):            # loads/stores
        if (w >> 26) & 1:                  # SIMD/FP
            return []
        r = []
        hi = (w >> 28) & 3
        if hi == 2:                        # pair
            if (w >> 22) & 1:
                r += [w & 31, (w >> 10) & 31]
            if (w >> 23) & 3 in (1, 3):    # post/pre writeback
                r.append((w >> 5) & 31)
        elif hi == 3:                      # single register
            if (w >> 22) & 3:
                r.append(w & 31)
            if not (w >> 24) & 1 and not (w >> 21) & 1 and (w >> 10) & 3 in (1, 3):
                r.append((w >> 5) & 31)
        elif (w >> 27) & 7 == 3:           # ldr literal
            r.append(w & 31)
        return r
    return []                              # SIMD/FP data processing


def is_cond_branch(w):
    return ((w & 0xFF000010) == 0x54000000 or (w & 0x7E000000) == 0x34000000
            or (w & 0x7E000000) == 0x36000000)


def sys_allow(bp):
    """Words emitted by `em(insns, n, <int>)` inside every `fn emit_sys_*` of bp."""
    src = open(bp).read()
    allow = set()
    for m in re.finditer(r"^fn emit_sys_\w+\(.*?\n}\n", src, re.S | re.M):
        allow |= {int(x) for x in re.findall(r"em\(insns, n, (\d+)\)", m.group(0))}
    return allow


def check_bin(path, allow):
    """-> (errors, sys_count, argpass_count)."""
    try:
        W, entry, code_end = load_bin(path)
    except ValueError as e:
        return [f"footer: {e}"], 0, 0
    starts = fn_starts(W, code_end)
    errs, nsys, narg = [], 0, 0
    if entry not in starts:
        errs.append(f"entry word {entry} is not a fn prologue (L11)")
    for i in range(code_end, len(W) - 1):
        if (W[i], W[i + 1]) == PROLOGUE:
            errs.append(f"fn prologue @{i} inside data cells")
    for k, s in enumerate(starts):
        e = starts[k + 1] if k + 1 < len(starts) else code_end
        span = W[s:e]
        if span[-1] != RET:
            errs.append(f"fn#{k} @{s} does not end with ret")
        for i in range(PRO_N, len(span) - EPI_N):
            w = span[i]
            for r in writes(w):
                if r in (27, 28):
                    if (w >> 5) & 31 not in (27, 28):   # arena bump add x27,x27,<reg>
                        errs.append(f"fn#{k} @{s + i}: {w:08x} writes x{r}")
                elif 9 <= r <= 13:
                    if w in allow:
                        nsys += 1
                    elif w in ARGPASS:
                        narg += 1
                    else:
                        errs.append(f"fn#{k} @{s + i}: {w:08x} writes x{r}")
    return errs, nsys, narg


# ---- (iii) fntab zone map -------------------------------------------------
ZONES = [(0, 1, "fntab"), (3655, 3661, "fold"), (3700, 3796, "slots"),
         (3890, 3898, "bank"), (3899, 3999, "literals"), (4000, 4000, "budget")]
LIT_BASE, LIT_END = 3903, 4000


def zone_of(b):
    return next((z for lo, hi, z in ZONES if lo <= b <= hi), None)


def count_literals(bp):
    """Mirror of bebop.bp scan_literals: `"..."` outside `//` comments."""
    s, i, n = open(bp).read(), 0, 0
    while i < len(s):
        if s.startswith("//", i):
            i = s.find("\n", i)
            if i < 0:
                break
        elif s[i] == '"':
            n += 1
            j = s.find('"', i + 1)
            i = len(s) if j < 0 else j
        i += 1
    return n


def check_fntab(bp, extra):
    src = open(bp).read().split("\n")
    errs, used = [], {}
    for ln, line in enumerate(src, 1):
        for b in re.findall(r"fntab\[(\d+)", line):
            used.setdefault(int(b), ln)
    for b, ln in sorted(used.items()):
        if zone_of(b) is None:
            errs.append(f"{bp}:{ln}: fntab[{b}] outside the zone map")
    sizes = {int(x) for x in re.findall(r"fntab = zeros\((\d+)\)", "\n".join(src))}
    if not sizes or min(sizes) <= LIT_END:
        errs.append(f"fntab allocation {sizes} does not cover index {LIT_END}")
    trap = [ln for ln, l in enumerate(src, 1) if f"fntab[{LIT_BASE} + lcnt[0]] =" in l]
    guarded = any(str(LIT_END) in l for ln in trap for l in src[ln - 3:ln])
    print(f"fntab zones: {len(used)} constant bases, all in "
          + "/".join(z for _, _, z in ZONES))
    print(f"literal trap ({LIT_BASE} + nlits >= {LIT_END}): "
          + ("PRESENT" if guarded else f"MISSING at {bp}:{trap[0] if trap else '?'}")
          + " (compile-time trap owned by bebop.bp)")
    for f in [bp] + extra:
        nl = count_literals(f)
        if LIT_BASE + nl >= LIT_END:
            errs.append(f"{f}: {nl} literals, {LIT_BASE}+{nl} >= {LIT_END} (F-F collision)")
        else:
            print(f"literals {f}: {nl} ({LIT_END - LIT_BASE - nl} headroom)")
    return errs


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    if argv[0] == "--fntab":
        errs = check_fntab(argv[1], argv[2:])
        for e in errs:
            print("FNTAB FAIL:", e)
        return 1 if errs else 0
    bp = os.path.join(ROOT, "bebop.bp")
    if argv[0] == "--allow-from":
        bp, argv = argv[1], argv[2:]
    allow = sys_allow(bp)
    rc = 0
    for path in argv:
        errs, nsys, narg = check_bin(path, allow)
        if errs:
            rc = 1
            print(f"ABI FAIL {path}:")
            for e in errs[:10]:
                print("  " + e)
        else:
            print(f"ABI ok {path}: x27/x28 clean, x9-x13 allowlisted sys={nsys} argpass={narg}")
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
