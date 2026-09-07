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
  byte offset % 4 == 0, entry inside the code region and at a prologue (or, since
  T118b, at the 39-word entry stub whose word 35 is `b` to a prologue),
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
STP_X15X14 = 0xA9BF3BEF   # stp x15,x14,[sp,#-16]!  (call-site x15/x14 save wrapper)
MAX_ARGS = 13             # x0..x7 direct + x8..x13 spill-slot params 9-14 (blueprint §1)


def argpass_window(span):
    """Positions in `span` where a write to x8..x13 is a REGISTER-MODEL-BLUEPRINT §3.8
    argument placement, not a violation: any word within the (up to MAX_ARGS) words
    immediately before a `bl`, or before its `stp x15,x14` save wrapper when present --
    generalises the old fixed ldr/mov-from-x0 patterns to whatever vs_place_args actually
    emits (a direct mov, a cs mov, a slot ldr, or a mat_const/mat_mulc sequence) for that
    argument. Kept tight: only the contiguous run immediately preceding one `bl`."""
    win = set()
    for j, w in enumerate(span):
        if (w >> 26) != 0x25:          # not a `bl`
            continue
        call_start = j - 1 if j > 0 and span[j - 1] == STP_X15X14 else j
        win.update(range(max(0, call_start - MAX_ARGS), call_start))
    return win


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


def entry_stub(bp):
    """T118b: the words of bebop.bp entry_stub (`st[i] = <int>`); the 0x14000000 word is `b main`."""
    m = re.search(r"^fn entry_stub\(.*?\n}\n", open(bp).read(), re.S | re.M)
    return [int(x) for x in re.findall(r"st\[\d+\] = (\d+)", m.group(0))] if m else []


def at_stub(W, entry, starts, stub):
    """entry is the T118b stub iff its words match the template and the `b` placeholder branches to a fn prologue."""
    span = W[entry:entry + len(stub)]
    if not stub or 0x14000000 not in stub or len(span) != len(stub):
        return False
    bi = stub.index(0x14000000)   # the `b .` placeholder cli_compile patches into `b main`
    if any(a != b for i, (a, b) in enumerate(zip(span, stub)) if i != bi):
        return False
    b = span[bi]
    imm = b & 0x3FFFFFF
    imm -= 1 << 26 if imm & (1 << 25) else 0
    return (b >> 26) == 5 and entry + bi + imm in starts


def check_bin(path, allow, stub=()):
    """-> (errors, sys_count, argpass_count)."""
    try:
        W, entry, code_end = load_bin(path)
    except ValueError as e:
        return [f"footer: {e}"], 0, 0
    starts = fn_starts(W, code_end)
    errs, nsys, narg = [], 0, 0
    if entry not in starts and not at_stub(W, entry, starts, stub):
        errs.append(f"entry word {entry} is not a fn prologue nor the T118b entry stub -> main (L11)")
    for i in range(code_end, len(W) - 1):
        if (W[i], W[i + 1]) == PROLOGUE:
            errs.append(f"fn prologue @{i} inside data cells")
    for k, s in enumerate(starts):
        e = starts[k + 1] if k + 1 < len(starts) else code_end
        span = W[s:e]
        if span[-1] != RET:
            errs.append(f"fn#{k} @{s} does not end with ret")
        argwin = argpass_window(span)
        for i in range(PRO_N, len(span) - EPI_N):
            w = span[i]
            for r in writes(w):
                if r in (27, 28):
                    if (w >> 5) & 31 not in (27, 28) and w not in allow:   # arena bump add x27,x27,<reg>; or a sys_clone child-arena rebind (a literal em() word of emit_sys_clone, in the allowlist)
                        errs.append(f"fn#{k} @{s + i}: {w:08x} writes x{r}")
                elif 9 <= r <= 13:
                    if w in allow:
                        nsys += 1
                    elif w in ARGPASS or i in argwin:
                        narg += 1
                    else:
                        errs.append(f"fn#{k} @{s + i}: {w:08x} writes x{r}")
    return errs, nsys, narg


# ---- (iii) fntab zone map -------------------------------------------------
# A2 step 0 relayout (2026-09-06/07): fn cap 300 -> 512 pushed the floating
# zone1/2/3 + enum + ft_cache envelope worst case (cnt=511, ecnt=255) up to
# index 2046, past the old b1_facts base (1500) -- every fixed zone below
# moved to make room; fntab (and ptab, the planning-pass twin) grew
# zeros(4096) -> zeros(8192).
ZONES = [(0, 1, "fntab"), (2200, 2711, "b1_facts"), (2712, 2714, "b1_scratch"),
         (2800, 4335, "window"), (4405, 4411, "fold"), (4412, 4449, "jumps"),
         (4450, 4546, "slots"), (4547, 4548, "window_hdr"),
         (4573, 4577, "window_cs"), (4578, 4585, "hoist"), (4640, 4648, "bank"),
         (4649, 4699, "literals"), (4750, 4750, "budget"),
         (5000, 5999, "lit_table")]
# window (2026-09-06, REGISTER-MODEL-BLUEPRINT; raised 128->512 2026-09-06 --
# emit_cond's parkable-`d` fix needs one extra live entry per nested if-level
# for the whole else-branch compile; moved 2000+3i -> 2800+3i in the A2 step 0
# relayout): fntab[2800+3i..2802+3i] = kind/p0/p1 of window entry i, capacity
# 512 entries (2800..4335) -- a compile-time LIST, decoupled from the
# 8-register free mask at [4548]
# (only REG/MULC-window/FLAGS kinds actually own a register). window_hdr:
# [4547] w (entry count, 0..512), [4548] free mask x0..x7. window_cs:
# [4573] cs mask, [4574] slot cursor, [4575] cs_hi, [4576] tsp, [4577] S.
# hoist (2026-09-07, A2 commit 2): fntab[4578+2k]/[4579+2k] = value/register
# of loop-invariant-constant pair k (k<4), live only during the compile of
# one `while` loop's own body text (set at loop entry, cleared right after
# the body compiles, before the bottom-test recompile -- vs_push swaps a
# matching CONST push for a direct SYM(register) reference while a pair is
# live). Register 0 in a slot means unused.
# b1_facts (2026-09-06, moved 1500 -> 2200 in step 0): fntab[2200+i] =
# per-fn packed planning facts (vc*2+has_alloc), i = the fn's index in
# collect_fns order, keyed by lookup on source position (fntab_fact_lookup)
# so two same-named fn definitions never alias each other's facts.
# b1_scratch (moved 1800 -> 2712): [2712] call-site counter, [2713]
# skip-save flag, [2714] this-compile's harvested fact word -- all
# reset/written once per compile_fn_at call (bebop.bp:B1, 2026-09-06).
# literals/lit_table (moved 3899-3903+ -> 4649-4652 headers / 5000+i table,
# cap raised 193 -> 1000 entries): headers [4649] cum cells, [4650] lcnt,
# [4651] L, [4652] cursor; table entries at fntab[5000+i].
LIT_BASE, LIT_END = 5000, 6000


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
    allow, stub = sys_allow(bp), entry_stub(bp)
    rc = 0
    for path in argv:
        errs, nsys, narg = check_bin(path, allow, stub)
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
