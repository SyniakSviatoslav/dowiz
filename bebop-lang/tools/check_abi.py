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


# ROADMAP A6 (2026-09-08): T43's per-iteration allocation reset. Aggregates now
# bump the ARENA cursor x27 (the frame heap and its x14 are gone), so the `while`
# mark/reset pair that used to save and restore x14 saves and restores x27:
#   pmark  str x27,[sp,#80+8*d]      (a STORE -- writes memory, never a register,
#                                     so the x27/x28 rule below never saw it)
#   reset  ldr x27,[sp,#80+8*d]      (a LOAD into x27 -- this is the new form)
# emit_while_stmt gates the slot on `ldepth <= 20`, so d is 0..20 and the offset
# is 80..240; the words are `ldr x27,[sp,#0]` = 0xf94003fb + (10+d)*1024, derived
# with as+objdump. Allowlisting the exact 21 words (not the opcode shape) keeps
# the invariant that x27 is only ever restored from a mark THIS fn wrote: any
# other `ldr x27,[sp,#imm]` -- a different offset, a different base -- still FAILs.
T43_X27_RESET = {0xf94003fb + (10 + d) * 1024 for d in range(21)}


def sys_allow(bp):
    """Words emitted by `em(insns, n, <int>)` inside every `fn emit_sys_*` of bp,
    plus T43's x27 mark-reset loads (A6)."""
    src = open(bp).read()
    allow = set(T43_X27_RESET)
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
ZONES = [(0, 1, "fntab"), (2900, 3667, "b1_facts"), (3668, 3670, "b1_scratch"),
         (3700, 5235, "window"), (5240, 5246, "fold"), (5247, 5284, "jumps"),
         (5290, 5386, "slots"), (5387, 5388, "window_hdr"),
         (5389, 5393, "window_cs"), (5394, 5401, "hoist"), (5402, 5402, "arm_base"),
         (5403, 5403, "span_slots"), (5404, 5404, "frame"),
         (5410, 5537, "arrlen"),
         (5540, 5548, "bank"), (5549, 5599, "literals"), (5600, 5600, "budget"),
         (6000, 6999, "lit_table")]
# A16 prerequisite RELAYOUT (2026-09-09): the fn cap is 768, so the FLOATING fn zone
# (3*cnt + ecnt + 258 cells = 0..2816 at cnt=768, ecnt=255) needs everything above it
# to move. b1_facts is 768 cells because IT IS INDEXED BY FN INDEX -- see PERFN below,
# which is the gate that makes that structural instead of remembered. Top index 6999
# still fits fntab's zeros(8192), so no widening: the relayout costs ~0 bin_words.

# ---- per-fn zones and arrays must be >= the fn cap ------------------------
# THE DEFECT THIS EXISTS TO PREVENT (found 2026-09-09 by audit, not by a gate): raising
# the cap 512 -> 560 while b1_facts stayed 512 cells let fn index 512..514 write straight
# through b1_scratch -- the B1 call-site counter and skip-save flag -- i.e. a WRONG `bl`
# TARGET, silently. No gate could catch it: every construct, kernel and std_test is far
# under 512 fns. And `offs`, the fn-indexed start table, has FIVE zeros() sites including
# cli_compile's, the ordinary compile path; widening one is not widening it.
# So: read the cap out of bebop.bp and refuse any per-fn zone or array smaller than it.
# ---- every constant base must be REGISTERED, not merely inside some zone ------
# THE GAP THIS CLOSES (found 2026-09-09, one row after the zone map was relaid out):
# F3 commit 1 introduced the per-symbol array-length table at fntab[4810 + k]. Under
# the OLD layout 4810 was free space; under the relayout it falls INSIDE the window
# zone (3700..5235). The membership check passed -- 4810 is "in the zone map" -- while
# the table would have been writing through window entries, silently, with exactly the
# shape of the b1_facts defect. Membership is not enough: a base has to be the base we
# SAID it was. Adding one is now a deliberate act, which is the point.
REGISTERED = {
    0: "fntab", 1: "fntab",
    2900: "b1_facts",
    3668: "b1_scratch", 3669: "b1_scratch", 3670: "b1_scratch",
    3700: "window", 3701: "window", 3702: "window",
    5245: "fold", 5246: "fold",
    5247: "jumps", 5248: "jumps", 5265: "jumps", 5266: "jumps",
    5290: "slots",
    5387: "window_hdr", 5388: "window_hdr",
    5389: "window_cs", 5390: "window_cs", 5391: "window_cs", 5392: "window_cs", 5393: "window_cs",
    5394: "hoist", 5395: "hoist", 5396: "hoist", 5397: "hoist",
    5398: "hoist", 5399: "hoist", 5400: "hoist", 5401: "hoist",
    5402: "arm_base", 5403: "span_slots", 5404: "frame",
    5410: "arrlen",
    5540: "bank", 5541: "bank", 5542: "bank", 5543: "bank",
    5549: "literals", 5550: "literals", 5551: "literals", 5552: "literals",
    5600: "budget",
    6000: "lit_table",
}


def check_registered(bases):
    bad = []
    byname = {n: (lo, hi) for lo, hi, n in ZONES}
    for b in sorted(bases):
        z = REGISTERED.get(b)
        if z is None:
            bad.append("REGISTRY FAIL: fntab[%d] is not a registered base -- it may fall inside "
                       "a zone and still be the wrong one; add it to REGISTERED with its zone" % b)
        else:
            lo, hi = byname[z]
            if not (lo <= b <= hi):
                bad.append("REGISTRY FAIL: fntab[%d] is registered to %s (%d..%d) but does not "
                           "lie in it" % (b, z, lo, hi))
    return bad


PERFN_ZONES = ["b1_facts"]
PERFN_ARRAYS = ["fnames", "fpos", "offs", "sizes", "starts", "factbuf", "fnames_l", "fpos_l"]


def check_perfn(src_path):
    import re as _re
    src = open(src_path, encoding="utf-8", errors="replace").read()
    caps = {int(m) for m in _re.findall(r">= (\d+) then diag_exit\(s, 0, 104\)", src)}
    caps |= {int(m) for m in _re.findall(r"cnt\[0\] < (\d+)", src)}
    if not caps:
        print("PERFN FAIL: no fn cap found in %s -- this gate is blind, fix the pattern" % src_path)
        return 1
    if len(caps) > 1:
        print("PERFN FAIL: the fn cap disagrees with itself: %s" % sorted(caps))
        return 1
    cap = caps.pop()
    bad = 0
    for lo, hi, name in ZONES:
        if name in PERFN_ZONES and hi - lo + 1 < cap:
            print("PERFN FAIL: zone %s is %d cells for a fn cap of %d -- fn index %d would "
                  "write past it, silently" % (name, hi - lo + 1, cap, hi - lo + 1))
            bad = 1
    for m in _re.finditer(r"let\s+([A-Za-z_]\w*)\s*=\s*zeros\((\d+)\)", src):
        nm, n = m.group(1), int(m.group(2))
        if nm in PERFN_ARRAYS and n < cap:
            print("PERFN FAIL: array `%s` is zeros(%d) for a fn cap of %d -- an OOB store "
                  "at fn index %d" % (nm, n, cap, n))
            bad = 1
    if not bad:
        print("perfn: fn cap %d; every per-fn zone and array is at least that wide" % cap)
    return bad
# window (2026-09-06, REGISTER-MODEL-BLUEPRINT; raised 128->512 2026-09-06 --
# emit_cond's parkable-`d` fix needs one extra live entry per nested if-level
# for the whole else-branch compile; moved 2000+3i -> 2800+3i in the A2 step 0
# relayout): fntab[2800+3i..2802+3i] = kind/p0/p1 of window entry i, capacity
# 512 entries (2800..4335) -- a compile-time LIST, decoupled from the
# 8-register free mask at [4548]
# (only REG/MULC-window/FLAGS kinds actually own a register). window_hdr:
# [4547] w (entry count, 0..512), [4548] free mask x0..x7. window_cs:
# [4573] cs mask, [4574] slot cursor, [4575] cs_hi, [4576] tsp, [4577] S.
# span_slots (2026-09-07, ROADMAP A14): fntab[4592] = 1 while emit_cond parks the
# values live across an `if` whose arms can BIND -- vs_park_move then refuses a cs
# register as a park destination and vs_span_to_slots moves the already-resident
# ones to temp slots, so no arm-spanning relocation can happen.
# arm_base (2026-09-07, A4 fuzz DIVERGE hunt RC2): fntab[4591] = the window
# index (`sw`) at which the currently-compiling if-ARM started, or -1 when
# not inside an if-arm; check_reg_collision compares a relocation target's
# index against it to tell an arm-local entry (safe to relocate) from one
# that predates the arm (must exit 89 instead, see emit_cond_branch).
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
LIT_BASE, LIT_END = 6000, 7000   # A16 relayout 2026-09-09: lit_table moved 5000..5999 -> 6000..6999


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
    for e in check_registered(used):
        print(e)
        errs.append(e)
    if check_perfn(bp):
        errs.append("per-fn zone or array narrower than the fn cap (see PERFN FAIL above)")
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
