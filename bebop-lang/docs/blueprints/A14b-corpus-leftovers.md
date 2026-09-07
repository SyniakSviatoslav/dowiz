# A14b — the three leftovers of A14's 330-repro corpus sweep

Status: 2026-09-08 — Part 1 LANDED (analysis), Part 2 SPEC READY (this file). ROADMAP row A14b,
opened by the A14 landing at commit 21e92aa / bebop.bin `7d8262a1`. Read `docs/WORKER-CARD.md`
first. Line numbers below are as of `bebop.bp` md5 `67ce8d5b938e90fe9e67ecdbac2d7c58`; the main
session is editing that file, so **anchor on function names, not on line numbers**.

## Background

A14 made the pre-`if` park path-independent (temp SLOT, not a cs register) and took the sweep of
the 330 `UNSUPPORTED-89-*.bp` repros in `~/.cache/bebop/fuzzd/repros` from 0/330 to 327/330.
Leftovers: 100744 (exit 89), 100671 + 100828 (`trap 81`, frame heap). Operator decision
2026-09-08: **the trap-81 pair waits for A6**; A14b is seed 100744 only.

## Part 1 — LANDED

Shrunk `UNSUPPORTED-89-100744.bp` to `bench/parity_constructs/neg/c92_letlive2.bp` (11 lines,
`5:2049: let binding while a call temp is live`), journal `1788820570`. **Correction from Part 2's
instrumented run:** that journal line's attribution is wrong. The site that fires is
`check_reg_collision`'s **arm_spanning** trap (bebop.bp:1855), *not* the `bit==1 && idx<0` assert
(bebop.bp:1861). Evidence: a scratch compiler with both `diag_exit(...,89)` sites replaced by a
window dump prints `site=1` (arm_spanning). The verdict letter (A — a case A14's mechanism does not
cover) still stands.

## Part 2 — ROOT CAUSE (measured, not conjectured)

Instrumented compiler (`/root/.cache/bebop/s27/w/dbg*.bp`, built with the committed `./bebop.bin`)
traced on `c92_letlive2.bp`. Three events, in order, all inside ONE if-arm (`fntab[4591] == 7`):

```
E6 3>2/101 1007 10   vs_settle_sym_alias: entry 6 SYM x19 -> REG x1   (arm base 7, w=10)
P4 6 24  1007 11     vs_park_to_cs:       entry 6 REG x1  -> CS  x24  (arm base 7, w=11)
1 24 1006 224 18 ... check_reg_collision: reg=x24 idx=6 < armbase 7 -> exit 89
```

Chain: (1) the arm contains a write to `v0` (`let _ = v0 = …`), so `vs_settle_sym_alias`
(bebop.bp:2459, its `vs_mat` at 2466) — the RC2 snapshot fix — scans the window **from index 0**
and materialises a *pre-arm* pending `SYM x19` operand into a window register, emitting the `mov`
inside one arm only. (2) That entry is now kind 2, so when the arm exhausts x0..x7 `vs_alloc` →
`vs_park_deepest` (2237) → `vs_park_move` picks it and, because `fntab[4592]` is 0 inside the arms
(2148), parks it to a **cs register** x24. (3) The next `let` in that same arm gets symbol register
19+S = x24 from `sym_bind`, `check_reg_collision` finds the owning entry at idx 6 < arm base 7, and
the A4/RC2 arm_spanning guard exits 89 (bebop.bp:1855).

A14 guaranteed "no pre-`if` entry owns a register" for kinds 2/4/6/7 (`vs_park` + `vs_span_to_slots`
at emit_cond, bebop.bp:3125-3127). It left **kind 3 (SYM)** alone — correct at the time, wrong once
`vs_settle_sym_alias` can turn a pre-arm SYM into a register-owning entry *inside* an arm.

Minimal positive repro built from that chain (9 lines, bpref = **17**; `./bebop.bin` = exit 89 at
`6:146`; both ablations — drop the `let _ = v = 9 in`, or drop the `let t = 4 in t` — compile clean,
so both ingredients are necessary):

```bebop
fn f8(a: i64, b: i64, c: i64, d: i64, e: i64, g: i64, h: i64, i: i64) -> i64 {
  a + i
}
fn main() -> i64 {
  let v = 3;
  let r = f8(v, v, v, v, v, v, v, (if f8(1, 1, 1, 1, 1, 1, 1, 1) then (let _ = v = 9 in f8(v + 1, v + 2, v + 3, v + 4, v + 5, v + 6, v + 7, (let t = 4 in t))) else 5));
  r
}
```

## Part 2 — VERDICT: (i) accept the program

The exit 89 is a **false positive of a correct guard**. Two reasons a reviewer can check:

1. The program is legal and its value is defined: `python3 tools/bpref.py` on the corpus original
   gives **10**, on the repro above **17**. Nothing here exceeds the documented >8-live-symbol
   restriction (`c92_letlive2` has one live symbol, `v0`), and it is not the `s0 + tsp0 > 256`
   bound.
2. The arm_spanning trap is *right* to refuse — the residency it sees really is path-dependent —
   but the compiler created that residency itself, one step earlier, by relocating a pre-arm entry
   inside a single arm. Fix the relocation, not the trap. **`check_reg_collision` must not be
   weakened**: after the fix it is unreachable from this shape and stays as the assertion that
   closes RC2.

## Part 2 — CHANGE PLAN

- Extend `vs_span_to_slots` (bebop.bp:2182) so the pre-`if` demotion also covers **kind 3 (SYM)**
  entries below the top, exactly as it covers kind 4. Register-resident symbol (`p0 < 100`):
  `vs_park_to_slot(insns, n, fntab, i, p0)` — its `str` word already parameterises Rt over all 32
  registers and its `if d0 < 8` mask guard already skips x19..x26. Spilled symbol (`p0 >= 100`):
  `vs_alloc` a temp, emit `vs_mat_sym`'s existing spilled-symbol `ldr` word
  (`4181721568 + (p0 - 100) * 1024 + t`), `vs_park_to_slot(..., i, t)`, `vs_mask_free`.
  **No new instruction word** is introduced, so `tools/check_words.py` needs no new objdump row.
- Nothing else changes: `emit_cond` still gates the call on `pure == 0` and still brackets it with
  `fntab[4592] = 1`, so the `vs_alloc` inside the spilled branch can only park to a slot.
- Invariant afterwards: **at every point where `fntab[4591] >= 0`, no window entry with index
  `< fntab[4591]` is kind 2/3/4/6/7** — i.e. every pre-arm entry is a SLOT or a CONST, neither of
  which any arm can relocate. Recommended permanent proof for the reviewer: a two-word guard in
  `vs_set_entry` — `idx < fntab[4591] && kind != 1 && kind != 5` → `sys_exit(<new code>)` — in the
  same style as the `vs_mask_take`/`vs_cs_take` choke-point asserts.
- Proof RC2 stays closed: the `arm_spanning` `diag_exit(s, srcpos, 89)` at bebop.bp:1855 is
  **untouched**, and `c91_letlive) EXPECT=12` (A14's own regression row) still passes.
- Measured on a scratch build of exactly this change (`/root/.cache/bebop/s27/w/fix.bp`, built by
  the committed `./bebop.bin`): self-hosts to a fixpoint (gen2 md5 == gen3 md5
  `a56e7a8e713cad51c26c6a0aab1a39ed`), compiler grows 40411 → 40504 words (**+93**, +372 bytes);
  seed 100744 (both the corpus original and `c92_letlive2`-derived form) compiles and runs to
  **10**; the repro above runs to **17**; smoke values unchanged: c91_letlive 12, c90_symalias 0,
  c05_if 111, c06_let 7, c16_compound 3, c12_match 6, c13_array 119, c07_while 45.
- RISK to watch: more temp slots per branch-`if` pushes `fntab[4576]` (tsp) up, so the
  `s0 + tsp0 > 256` trap and the frame-heap `trap 81` pair get *less* headroom. The corpus sweep is
  the gate that catches it; if it regresses, narrow the trigger with a text pre-scan of the two
  arms for a binder/writer (`let` / a bare `=`) done once in `emit_cond`, beside `arm_is_pure`, so
  both passes see identical text.

## Part 2 — ACCEPTANCE (all must be GREEN, shown verbatim)

1. `SERIAL=1 PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen` → the **fixpoint
   line** (gen3 == gen4) with its md5; then promote via `cp $OUT/gen4.bin bebop.bin.tmp && mv`.
2. `bench/vs_rust/invariants.sh --freeze` clean, and
   `SERIAL=1 BEBOP_TMP=$OUT FREEZE=1 SRC=bebop.bp tools/battery.sh ./bebop.bin $OUT/bat` →
   `battery: GREEN`.
3. Construct rows in `bench/vs_rust/construct_parity.sh`:
   - `c92_letlive2) EXPECT=COMPILEFAIL:97;;` — **changed from 89**. The shrunk repro has no tail
     expression in `main`; `python3 tools/bpref.py` on it says
     `SyntaxError: fn main: body has no tail expression (bebop.bin exits 97)`, and the fixed
     compiler reaches the parser and reports 97. Update the comment above the row too.
   - (renamed on merge 2026-09-08: the card was written as `c94_symspan`, but B1 step 1's
     `c94_fsync` took that number in commit abaab38 -- the repro is `c95_symspan`.)
   - NEW positive row `c95_symspan) EXPECT=17;;` with the 9-line program above as
     `bench/parity_constructs/c95_symspan.bp` -- HELD OUT of that directory until the fix lands,
     because construct_parity.sh globs `*.bp` and an EXPECT-less COMPILEFAIL turns the battery RED;
     the program is the fenced block above, and it still reproduces on 42ce19e5 (rc 89 at 6:146,
     bpref 17, re-verified after the A13+A15 / A9 / B1-step-1 merges). (EXPECT from `python3 tools/bpref.py`). This is the
     real regression guard; `c92_letlive2` only proves the 89 is gone.
   - `c91_letlive) EXPECT=12;;` and `c90_symalias) EXPECT=0;;` unchanged and passing.
   - Final line `construct parity: pass=<N> fail=0`.
4. Corpus: `/root/.cache/bebop/s26/sweep.sh ./bebop.bin $OUT/sweep` →
   `SWEEP … match=328 mismatch=0 compile89=0 runtimeout=0` (330 total; the two trap-81 seeds
   100671/100828 remain outliers until A6 — quote them explicitly, do not claim 330/330).
5. `WORD_DELTA` lines from the chain plus a `bench/parity_constructs/word_budget.txt` row for every
   construct that grew, and a `bin_words` budget line for the compiler's own +~93 words
   (measured 40411 → 40504; confirm against the chain's own number, do not reuse this one).
6. Census: `bcond/cbz/tbz` counts must **not** increase (the change emits only `str`/`ldr`), so no
   `census_allow.txt` line should be needed; if one is, the change is not the one specified here.
7. ONE journal line, `<epoch> H:… | DID:… | GOT:… | VERDICT:…`.

## The sweep harness

`/root/.cache/bebop/s26/sweep.sh <bin> <outdir>` compiles + runs every `UNSUPPORTED-89-*.bp` with
the given binary and compares against the `expected=` value in each file's header. It prints one
`SWEEP bin=… match=… mismatch=… compile89=… runtimeout=…` line and writes outliers to
`<outdir>/sweep.txt` (~2.5 min for 330 files). Reuse it; do not write a second one.
