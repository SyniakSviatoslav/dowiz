Status: 2026-09-13, READ-ONLY research pass over /root/dowiz/bebop-lang (no git command run, nothing
written under /root/dowiz; the working tree's `bebop.bp` is 2026-09-13 12:36, newer than `bebop.bin`
09:00, so line numbers below are against the DIRTY source and may drift by a few lines after the main
session's next commit). Written for the operator decision of 2026-09-13 (floats, strings as values,
module contents become REQUIRED) as re-aimed by the same-day addendum (posit/fixed-point not IEEE;
zero-allocation fixed-length strings; modules as elaboration-time namespace flattening).

Legend: every claim carries `file:line` I read. **ESTIMATE** = derived from an algorithm or from the
tree's own measured unit costs, not measured. **SPECULATIVE** = I could not cite or run it.
**KNOWLEDGE** = background not in the tree (the posit standard, Lean's `Float`), stated so it can be
checked. **MEASURED** = run today through `tools/slot.sh` on a copy of `seed` + the promoted `bebop.bin`
in my scratchpad (§7 lists every probe, its command and its output; four ran, two did not). Slot 1 was
held by the main session (`L32-final`) for most of the pass; the compile probes ran at 12:59:58 when it
freed, in 0 s.

**Two defects found by the probes, independent of any decision here (§7):** (1) `let x = 1.5; x`
COMPILES rc=0 and prints `495807598623` -- silent garbage in a syntactic position, the F8-step-0 class
(ROADMAP.md:201), and it belongs in WAVE 0 as a neg construct whether or not floats ever land;
(2) `module m { fn f() -> i64 { 7 } }` compiles and prints 7 (the braces are invisible, `f` is global)
while `tools/bpref.py` answers `UNSUPPORTED:builtin f` -- the compiler and the oracle disagree today on
a form both are documented to accept.

Numbers used throughout, re-derived today:

| fact | value | where |
|---|---|---|
| promoted compiler | 43,023 words, bcond 1223, cbz 126, tbz 0 | bench/vs_rust/census.txt:2 |
| constructs | 97 positive + 18 `neg/` = **115** | `ls bench/parity_constructs{,/neg}/*.bp` |
| constructs that touch strings (`"`, `char`, `str_len`) | **22** positive; 0 use fp; 9 use `use` | grep over bench/parity_constructs |
| std gates | 117; **25** oracles bake Q32 arithmetic (`>> 32` / `fp_mul`); 20 std files call `fp_mul`/`isqrt` | `grep -c '^gate ' bench/vs_rust/std_golden.sh`; bench/oracles/*.py |
| files carrying `module core { }` | 43 (std/prelude/constructs), and the compiler itself | bebop.bp:17; grep `^module ` |
| fn cap / budget | 768 / 547 measured by padding | bebop.bp:6356-6360; ROADMAP.md:104 |
| honest kernels vs Rust | K1H 1.8x, K2H 3.3x, K3H 3.4x, K4 1.6x (2026-09-06); K2H 2.6x, K3H 2.4x after B1 | ROADMAP.md:51; bench/vs_rust/REPORT-honest.md:9-10,33 |
| four implementations of the language | bebop.bp; tools/bpref.py 717 lines; formal/Bebop/*.lean 2,853 lines; tools/typecheck.py 177 lines | `wc -l` |
| Lean status | 7 `axiom`s, 0 proved theorems; Semantics.lean 607 lines "mirrors bpref.py"; runs OFF-BOX, never run on-box | formal/Bebop/Theorems.lean:105,169,220,231,287,295,305; formal/README.md:12-16 |

---

# 1. RESEQUENCING RECOMMENDATION (read this first)

## 1.0 The one-paragraph answer

Under the addendum's designs -- fixed point (or posit) as pure integer bit functions, strings as
fixed-length byte arrays on the same arena with no allocation the program did not write, modules as a
textual flattening that leaves zero words -- **none of the three changes the runtime semantics that F4
mechanises beyond three small, enumerable deltas** (§1.3). The original brief's fear ("if
floats/strings/modules land after F4 starts, F4 is redone") was premised on IEEE arithmetic, heap
strings and runtime modules; with those refused, **F4 is not the risk and does not move**. The double
work that IS real sits in three places, and each is avoided by folding rather than by reordering:

1. **Strings ARE A8 + F3 at cell width 1.** A `[u8; N]` string is F3 arm (ii)'s declared length
   (ROADMAP.md:196 `[i64; k]`/`[i64; n]`) over A8's second cell width (docs/blueprints/A8-typed-tables-u32.md:19
   already reserves tag 2 `str`, 4 `[u32]`, 5 `[str]`) with A7 step 2's `(off << 32 | len)` handle as the
   runtime view (docs/blueprints/A7-byte-arena-and-str-values.md §3). Three rows already own the three
   halves; a fourth row would build the side channel a second time. **Fold strings into A8 (tag + forms)
   and F3 (declared length), and re-land A7 step 2 FIRST as their producer.**
2. **Floats ARE A8's tag 6.** docs/blueprints/A8-typed-tables-u32.md:19 already reserves type tag 6 `fp`
   ("a tagged i64; the store's layout digest already distinguishes fp"), docs/LANG-DB-DESIGN.md:154 already
   says "`fp` (Q32) is an i64 with a type tag in the layout, zero cost", and F9's first theorem is already
   the Q32 statement (formal/Bebop/Theorems.lean:36-60). **Fold floats into A8 as "typed arithmetic on tag
   6" plus one hardware-multiply builtin (A9 already lists `umulh`, docs/blueprints/A9-neon-builtins.md:3).**
   Do NOT open an IEEE row; do NOT put posit in the compiler (§2, §5).
3. **Modules, monomorphisation and `test` expansion are ONE textual rewriter.** A16 step 4 is
   "instantiation textual in the `use_expand` shape, 8-12 fns" (docs/blueprints/A16-closures-generics-hof.md:78-80,
   222-224); A24 step 2 stopped on "the missing primitive -- 'append a RANGE of the source' -- is in no file"
   (ROADMAP.md:110); module flattening needs exactly `copy a source range` + `rename identifiers in a range`.
   **Build the rewriter once, as its own row, before A16 step 4 and A24 step 2, and make its output (the
   `.use` file the compiler already writes, docs/LANGUAGE.md:32-33) the ONE program text that bpref,
   typecheck.py and the Lean harness consume.** That is also the answer to "four implementations": the
   flattener is written once, not four times.

## 1.1 The table

| row | move | why | what is redone if we don't |
|---|---|---|---|
| **A7 step 2** (`str`-as-handle re-land WHOLE; reverted 2026-09-12, ROADMAP.md:94) | from "close A7" at the END of WAVE 2 (ROADMAP.md:113) to the **HEAD of WAVE 2, before A8** | it is the PRODUCER every string design needs (the byte region, the literal copy, `char` = `ldrb [x17, off+i]`); bpref is ALREADY on the handle representation (tools/bpref.py:574-579, :603-609) while the compiler emits raw `adr` pointers (bebop.bp:4024-4035) -- the oracle is ahead of the compiler and only stays in agreement because no construct observes the handle bits | A8's tag 2/5/7 forms would be written against raw pointers and rewritten when the handle lands; the 22 string constructs are re-frozen twice |
| **A8** (typed tables) | no move in order; **amend the blueprint BEFORE it starts**: add tag 7 `[u8]` with declared length, make tag 6 `fp` carry operator dispatch, and add `smulh`/`umulh` to its builtin list | A8 is "the single most-depended-on open row" (ROADMAP.md:113) and its 4th stab cell is where both the fp tag and the u8 tag live (docs/blueprints/A8-typed-tables-u32.md:19, §4) | the stab layout (`zeros(385)` -> 513, every site) is touched a second time for u8, a third for fp |
| **F3** arm (ii) declared lengths | **BLOCKED** until the F3 blueprint is rewritten width-generic (`[T; n]` for T in i64/u32/u8) | ROADMAP.md:196 plans `[i64; k]`/`[i64; n]` only; a `[u8; 32]` string is the same side channel (`fntab[5410+lk]`, ROADMAP.md:112) at width 1; F3's own gate "bounds_census = 100 % of index sites" must count byte indexes too | the side channel, the discharge rule and `bounds_census` are built for one width and re-built for two more |
| **A16 step 4** (monomorphisation) | **BLOCKED** until the rewriter row (§1.2 "A25") lands; A16 steps 1-3 unchanged | it is a textual pass "in the `use_expand` shape" (blueprint :222-224) = the same two primitives modules need | two textual rewriters over the same source, two `.use`-writing paths, two sets of position remaps for `diag_exit` |
| **A24 step 2** (`test` expansion) | **BLOCKED** until the rewriter row; then it is ~2 days, not "a few hundred lines" | its named blocker (ROADMAP.md:110) is the range-copy primitive AND a `put_num` inside the generated program; the second is answered by S4 (prelude `fmt.bp` over a `[u8; N]` buffer, §3.2), not by string values | the range-copy primitive written a third time |
| **A17 step 1c** (`<file>:` prefix, 35 call sites) | no move | a `str` value that can be PARKED in a byte buffer (S1) makes the cheap alternative the row rejected (ROADMAP.md:105 "park the path in the arena") sound; do it after S1 | threading a parameter through 35 sites and then deleting it |
| **A21 / A22** | no move | A22 step 2's per-symbol tag IS A8 step 1a (ROADMAP.md:112) -- already merged; nothing here adds a tag family they did not plan | -- |
| **B7 step 2** (planner, `explain`) | no move; add dependency "after S2" | `qdsl_explain` today returns a FAKE handle `((bp[0]-1) << 32) \| (bp[0]-1)` (selfhost/std/qdsl.bp:494-508) and the construct folds it with `char()` (selfhost/std/c70_qdsl.bp:4-13); B7 step 2's `explain` must not be built on that | the explain path rewritten once real string views exist |
| **B8** | no move (already "needs A7") | -- | -- |
| **F4** (Lean semantics) | **does NOT move and is NOT blocked**; add three delta rows to its ladder (§1.3) | its value type stays `Int64` (formal/Bebop/Basic.lean:20-22); the deltas are one arithmetic function, one byte-addressing rule over packed cells, and zero for modules if the harness reads the flattened `.use` | nothing is redone; the fear was IEEE/GC/runtime-modules, all refused |
| **F5** (translation validation) | no move (WAVE 4); add dependency "after A8 s2b/s2c" | its inventory is "check_words.py's allowlist over the 532 emission sites" (ROADMAP.md:198); `ldrb/strb/smulh/extr` forms must be in it before the decoder is written | a decoder extended after the fact, the `tv_fragments` count re-baselined |
| **F6** (certificate checker) | no move; **the `ring` rule decision (ROADMAP.md:199) is the reason to refuse posit in the compiler** | Q32 multiply is a ring identity + one floor lemma (docs/RESEARCH-PROOF-DESIGN-2026-09-09.md:281); posit multiply has clz and variable shifts on the path, which closes the `ring` route and leaves only the SAT route the study rates HIGH-risk for 64x64 miters | -- |
| **F8** (dependent types) | no move; F8's "(c) declared lengths are the first dependent types" must be width-generic from its first commit | same side channel as F3 | the elaborator's length rule written for one width |
| **F9** (first theorems) | no move; statement unchanged; **add a bridge theorem** `fp_mul_hw a b = fp_mul_spec a b` when the hardware form lands | Theorems.lean:36-60 already states sign*floor(\|a\|\|b\|/2^32); the `smulh` form is the 128-bit product the spec is written against, so the proof gets easier | -- |
| **bpref** | no move; 0 change for S0 (already handles), +1 rule for fp (`(\|a\|*\|b\|) >> 32` with sign), reads the flattened `.use` for modules | tools/bpref.py:574-579; tools/bpref.py:257-258 today SKIPS `module NAME { ... }` wholesale -- fed `module m { fn f() -> i64 { 7 } }` it answers `UNSUPPORTED:builtin f` while the compiler prints 7 (both MEASURED today, §7) | a second flattener written in Python |
| **typecheck.py** | +2 tags (u8, fp), reads the flattened `.use` | tools/typecheck.py:6 types list; :64-67 `str` in arithmetic is already a finding | -- |
| **the 115-construct corpus** | 22 string constructs RE-FROZEN at S0 (WORD_DELTA, values unchanged: the handle is value-preserving by design, c68_strval header); **c28_plusplus / d03_plusplus do NOT flip** -- `++` stays exit 96 under zero-allocation strings (§3); ~8 new constructs (§4) | bench/parity_constructs/neg/c28_plusplus.bp:1-5; bench/diag_neg/d03_plusplus.bp:1 | freezing the 22 twice if A8's forms land before A7's handle |
| **F4's 86-construct Lean harness** | grows by the same ~8 | formal/Bebop/Conformance.lean:5-6 | -- |
| **tcheck.bp** (F6, own 511-fn cap) | untouched | it is a program (selfhost/tcheck.bp:17-19); modules flatten before it is compiled | -- |

## 1.2 One new row, proposed: A25 "the source rewriter" (before A16 s4, A24 s2, and modules)

Two primitives and one driver, all in the `use_expand` shape (bebop.bp:7030-7049: scan, build one byte
buffer, export to `<out>.use`, slurp it back):

- `src_copy_range(dst, off, src, from, to)` -- what A24 step 2 said is "in no file" (ROADMAP.md:110);
  `use_append_str` (bebop.bp:6882) is the whole-string special case of it.
- `src_rename_idents(dst, from, to, table)` -- rewrite every identifier in a range whose 131-rolling hash
  (bebop.bp:151-163) is in `table` to a new spelling; skips strings and `//` comments exactly as
  `collect_fns` does (bebop.bp:5978-5999).
- drivers: `mod_expand` (modules, §4.3), `mono_expand` (A16 s4), `test_expand` (A24 s2).

Cost: **ESTIMATE** 250-400 lines of bebop.bp, 8-12 fns (A16's own estimate for `mono_expand` alone,
blueprint :80), 0 words in any compiled program, +10-15 b.cond in the compiler (one impure `if` arm per
skip/copy decision, the A24 step 1 precedent was +16 for four scanner guards, census_allow.txt line
`bebop bcond 1223`), 2 weeks. Gate: byte-identity of the `.use` output against a hand-flattened twin for
three fixtures (a module, a generic instantiation, a `test` block), plus `diag_check.sh` positions still
correct after expansion (positions are already counted over the `.use` text, bebop.bp:43-44).

## 1.3 Where the original brief was wrong to worry (addendum item 4)

| feared | what the re-aimed design actually does to it |
|---|---|
| "F4 mechanises the whole language; floats change the value domain" | the value domain stays `Int64` (Basic.lean:20-22). A Q32 multiply is ONE new function `fp_mul_hw : Int64 -> Int64 -> Int64`, ~10 lines of Lean, and its spec already exists (Theorems.lean:36-60). A posit would be one BitVec function too (§2.2). Neither adds a rule to `evalExpr` unless `*` dispatches on a type tag, and then it is one match arm |
| "strings need a second memory (bytes) in the semantics" | bpref already has it (`self.bytes`, bpref.py:576-579); the Lean model can keep ONE cell array and define `char(s,i)` as a shifted byte of a packed cell (§3.2), which is the thesis's own model (docs/LANG-DB-DESIGN.md:146-152 "every persisted value is a run of i64 cells") |
| "modules need a symbol-table redesign the semantics must model" | zero: the emitter, bpref, typecheck and Lean all read the flattened text; `read_ident`'s hash is unchanged; `fntab_lookup` (bebop.bp:673-688) is unchanged. The ONLY semantic fact modules add is that two `fn f` in two modules are different functions -- which today is silently "last match wins" (bebop.bp:700-704) |
| "F5's instruction inventory is frozen" | it grows by four forms (`ldrb`, `strb`, `smulh`/`umulh`, `extr`), all register-parameterised single words; `hvham` already emits NEON words that check_abi ignores (tools/check_abi.py:26-27), so the inventory has grown before |
| "bpref cannot model floats" | it cannot model IEEE bit-exactly without care (§2.3); it models Q32 and posit EXACTLY because they are integer functions on Python ints |
| "Q32 vs floats coexist and money.bp is threatened" | money.bp uses plain i64 minor units, not Q32 (selfhost/std/money.bp:3-21); no float design touches it; its `mulov` proof route is unchanged (RESEARCH-PROOF-DESIGN:278) |

What I was RIGHT to worry about, and the addendum does not dissolve: (i) the **speed gate** -- a
software fraction format cannot meet "<= 2.0x a Rust twin" if the twin is hardware f64 (§2.5); (ii) the
**A7 step 2 re-land** is a real prerequisite that broke self-hosting once already (ROADMAP.md:94) and is
scheduled LAST in its wave; (iii) **three textual rewriters** would be written if nobody names the
shared primitive.

---

# 2. FLOATS

## 2.1 (a) What exists today

- The language is integer-only: "every value is a 64-bit integer (i64)" (docs/LANGUAGE.md:5-9);
  "floats (Q32 fixed point lives in selfhost/prelude/fp.bp)" under NOT in the language (LANGUAGE.md:132-133).
- `selfhost/prelude/fp.bp:1-5`: "Q32 fixed-point core shared by every fp gate (T38)"; `fp_mul` (:6-30)
  is a signed 64x64 -> Q32 product via 32/16-bit limbs "(no i128, no >> on negatives)"; `isqrt` (:35-44)
  is a clz-seeded Newton descent "bit-exact against math.isqrt on 1.2M values". Representation: Q31.32,
  one i64, 32 fraction bits (implicit in `>> 32`, :11-13, :24-25).
- 20 std programs call `fp_mul`/`isqrt` (cache, calcbound, csr, drift, fir, kalman, lcjit, lcres, msuper,
  qlora, scoord, seigtime, sgamma, sinc, spectral, srepl, tdg, tdgcurv, tdggeo, tq); 25 python oracles bake
  the same integer arithmetic (bench/oracles/*.py). These gates are GREEN in Q32: the workload evidence
  says range and precision beyond Q31.32 have not been needed.
- The number parser has no fraction branch: `read_num_val` (bebop.bp:3418-3440) reads decimal/`0x` digits
  and stops; `.` after an expression is FIELD ACCESS (bebop.bp:4356-4361 `is_dot -> emit_field_access`).
  **MEASURED:** `fn main() -> i64 { let x = 1.5; x }` compiles rc=0 on the promoted binary and prints
  `495807598623` -- the `1` is emitted, `.5` is taken as a field access, and a `ldr` off a constant
  address returns whatever sits there. No diagnostic. bpref refuses the same text as `UNSUPPORTED:.`. This
  is a loud-failure violation of the compiler in the F8-step-0 sense and is the first construct on the
  fp ladder (FP-1, §5.1), scheduled in WAVE 0, not with floats.
- **MEASURED word cost of the limb form:** fp.bp + `fn main() -> i64 { fp_mul(3 << 32, 5 << 32) >> 32 }`
  compiles to 1316 bytes against 724 for a bare `fn main() -> i64 { 3 }`: **148 words for `fp_mul` +
  `isqrt` + the call site** (the program printed 15, correct). `fp_mul` alone is **ESTIMATE 95-110** of
  those (isqrt is ~35-45, the call ~8-10).
- The emitter has NO FP register file usage and no FP words; the only SIMD precedent is `hvham`'s NEON
  popcount (bebop.bp:977-1000, `ldp q0,q1 ... cnt ... addv`), and `tools/check_abi.py:26-27` ignores
  SIMD/FP words ("no GPR write tracked").
- History already decided against IEEE once: T87 "f64 at the boundary only ... no float arithmetic
  instruction is ever emitted (FMOV/FADD banned by the census)" (HISTORY.md:2259-2265); the attic retired
  six modules with reason class **f64** ("the API is over `[f64]`; `bebop.bin` is i64-only, no exact fold",
  selfhost/std/attic/README.md:7). docs/RESEARCH-VERIFICATION-2026-09-09.md:283 names "one value type
  Z/2^64 with every operator total and NO floats" as the thing that makes the proof design tractable.
- The store already reserves the type: "Scalars: i64 only (doctrine). `fp` (Q32) is an i64 with a type tag
  in the layout, zero cost" (docs/LANG-DB-DESIGN.md:154); A8's tag table has `6 fp` (docs/blueprints/A8-typed-tables-u32.md:19).
- SPEEDUP-ANALYSIS.md:180 (M12): "A78: i64 mul 3 cyc, f64 fmul 3 cyc; hardware sdiv ~10 cyc, fsqrt ~15
  cyc ... fp_div/isqrt are 32-step software loops ... 100-300 ops where hardware needs 1".

## 2.2 The three candidates, adversarially

### IEEE-754 binary64 (the thing to argue against)

Honesty first, because the addendum's reason is not the strongest one available. **KNOWLEDGE:** the
five basic IEEE operations (+ - * / sqrt) are correctly rounded and therefore bit-identical on EVERY
conforming implementation for a given rounding mode; on AArch64 with Linux's default FPCR (RNE, FZ=0,
DN=0) `fmul d0,d1,d2` is deterministic. The "different behaviour on different processors" the operator
names comes from (1) x87 80-bit intermediates (x86 legacy, absent on AArch64), (2) FTZ/DAZ and FMA
contraction switched on by C compilers under `-ffast-math`/`-O3`, (3) libm transcendental functions,
(4) NaN payload/sign propagation, which ARM and x86 resolve differently. This compiler emits exact
words and has no libm, so (1)-(3) cannot enter unless an emitter writes them; only (4) is a genuine
cross-ISA indeterminacy, and only through bit inspection of NaNs. **So the reason to refuse IEEE here is
not hardware chaos; it is verification cost**, and that reason is decisive on its own:

- Lean 4 core's `Float` is opaque (extern C doubles; no definitional unfolding; nothing about `Float.mul`
  is provable) -- **KNOWLEDGE**; Mathlib has no operational IEEE model. F4 would need a hand-written
  softfloat in `BitVec 64` (~300-500 lines, denormals, five rounding cases, NaN canonicalisation) and
  then an AXIOM that the hardware `fmul` equals it. Under the tree's rule "if Lean disappeared tomorrow,
  what would still hold?" (ROADMAP.md:21-22), nothing in-tree would check that axiom: bpref on the same
  box uses the same FPU through Python floats, so bpref-vs-bebop.bin agreement on float arithmetic is
  hardware-vs-hardware, i.e. vacuous as a semantics check (it still checks the compiler's PLUMBING, which
  is worth something, but not what F4 is for).
- bpref divergences that are NOT vacuous and would each be a DIVERGE the oracle cannot adjudicate:
  Python raises `ZeroDivisionError` on `x/0.0` where hardware gives inf; Python `%` on floats has the
  sign of the divisor where `fmod` has the sign of the dividend; `float(int)` rounds the same but
  `int(float)` on NaN/inf raises; NaN payload bits differ by ISA. Each needs a hand-written rule.
- F6 would need IEEE multiply bit-blasted: a 53x53 multiplier plus a normaliser and a rounder with
  denormal handling -- larger than the fp_mul miter the study already rates HIGH-risk (RESEARCH-PROOF-DESIGN:281).
- ABI: a second register file. `check_abi.py`'s register-zone law (tools/check_abi.py:14-25) tracks GPR
  writes only; d0-d31 would need their own zone law, their own caller-saved discipline across `bl`
  (d8-d15 are callee-saved in AAPCS64 -- **KNOWLEDGE**), their own spill slots (the register model spills
  to `[x15, #(S+k)*8]` and traps at `S + tsp > 64`, docs/REGISTER-MODEL-BLUEPRINT.md:366), and the
  14-parameter convention would need a second argument sequence (v0-v7). That is the "second register
  allocator" the brief asked about, and the answer is yes -- unless floats are integer bit patterns, in
  which case it is NO: they live in x-registers like every other i64. **This is the strongest practical
  argument for integer formats in THIS compiler**: zero change to `vs_alloc`/`vs_park`/`sym_bind`
  (bebop.bp:236-253), zero change to the x27/x28 and x9-x13 laws, zero new spill class.

### Posit (posit64, es=2) -- primary candidate per the addendum, researched

**KNOWLEDGE (Posit Standard 2022):** an n-bit posit is `sign | regime | exponent(es bits) | fraction`;
the regime is a run of k identical bits terminated by the opposite bit, encoding a power of
`useed = 2^(2^es)` (es = 2 is FIXED by the 2022 standard for every size, so useed = 16); value =
(-1)^s * 16^k * 2^e * (1 + f). Exactly two special patterns: `0` (all zeros) and `NaR` (1 followed by
zeros); no +/-0, no infinities, no subnormals. Rounding: round-to-nearest, ties-to-even on the encoding
lattice; no result overflows to NaR -- magnitudes above maxpos round to maxpos, nonzero magnitudes below
minpos round to minpos (no underflow to zero). Comparison is two's-complement integer comparison of the
bit pattern (the one property posit shares with fixed point for free). posit64 es=2: maxpos = 2^248,
minpos = 2^-248, 59 fraction bits near 1.0, tapering to 52 at 2^32 (equal to f64 there) and worse
beyond. The standard also defines a quire (exact accumulator, 16n = 1024 bits for posit64).

(i) **Cost in words, no FPU, ESTIMATE from the SoftPosit-style algorithm** (each step is the AArch64
integer instruction list a hand emitter would write; the count is not measured -- the measurement is
§7 probe 4):

| step of `posit64_mul(a, b)` | words |
|---|---|
| NaR / zero exits (both operands) | 4 |
| sign strip: `cmp; cneg` x2 | 4 |
| regime decode x2: `cls`/`clz` on the bits after the sign, shift the run out (`lslv`), extract es (`ubfx`), fraction with hidden 1 (`orr`, `lsl`) | 12 |
| fraction multiply: `mul` + `umulh`; normalise (`tst; csel; lsr`); scale = 4(ka+kb) + ea + eb + carry | 10 |
| encode: regime run from scale (`asr #2`, `and #3`), build the run (`mov; lslv; sub`), place exp+frac (`orr; lsrv`), RNE with guard and sticky (`and; cmp; cinc; tst; ...`), saturate at maxpos/minpos (`cmp; csel` x2), sign (`eor; cneg`) | 25-30 |
| prologue/epilogue if a fn (bebop.bp `emit_prologue` 10 + `emit_epilogue` 8, tools/check_abi.py:36) | 18 |
| **total** | **~75-80 as a fn; ~55-60 inline** |

`posit64_add` is worse (align by the scale difference with a variable shift, add or subtract, renormalise
with `clz`, then the same encode): **ESTIMATE 90-130 words**. A call site costs 6-10 words (argument
placement + `bl` + result, ROADMAP.md:104's `call_fn` figure is 5-6). Compare the existing `fp_mul`
limb form, **MEASURED with `isqrt` and its call at 148 words** (§2.1; `fp_mul` alone ESTIMATE 95-110 --
my pre-measurement guess of 60-70 was low by ~40 %, which is the reason every posit number above must be
read as an estimate of the same quality until §7 probe 4b is run), and the hardware Q32 form below at
8-10 words inline: **roughly 10x fewer words than today's `fp_mul`, and roughly the same as a posit
multiply's ENCODE step alone.**

(ii) **Lean 4:** yes, mechanisable as a pure `BitVec 64 -> BitVec 64 -> BitVec 64` function with no
axioms -- total, deterministic, `decide`-able on concrete inputs, and `bv_decide` can bit-blast it (the
tree already found CaDiCaL inside the Lean toolchain, ROADMAP.md:199). It is EASIER than IEEE (no
special-case zoo, one rounding rule, no axiom to the hardware because there is no hardware) and
**HARDER than Q32**: Q32 multiply is `(|a| * |b|) >>> 32` with a sign, one lemma; posit multiply has a
`clz`, two barrel shifts and a variable-position rounder between the same multiplier and the result.

(iii) **QF_ABV for F6:** yes, it bit-blasts -- a barrel shifter is 6 mux levels x 64, `clz` is a
priority tree, both cheap in gates -- BUT the multiplier inside is the SAME 64x64 miter the design study
rates "HIGH for SAT (different multiplier decompositions)" for `fp_mul` (RESEARCH-PROOF-DESIGN:281), and
posit adds ~4 barrel shifters and a rounder on top. Worse: **the `ring` route the operator ruled in
(ROADMAP.md:199 "ONE word-level rule -- polynomial normal form over Z/2^64") is CLOSED for posit**: `clz`
and data-dependent shifts are not polynomial. So posit is bit-blastable but leaves F6 with only its risky
route, while Q32 keeps both.

(iv) **Against widening Q32:** "clean bit structure, fixed overflow behaviour, single rounding rule" is
true of Q31.32 today: it is an integer, overflow wraps like every Bebop integer (docs/LANGUAGE.md:73),
`fp_mul` truncates toward zero by construction (fp.bp:6-30), and it already carries F9's theorem. What
posit buys over Q32 is DYNAMIC RANGE (2^+-248 vs 2^+-31 at 2^-32 resolution) and 27 more fraction bits
near 1.0. What it costs is ~10x the words per multiply, ~30-100x the cycles of hardware f64 (§2.5), the
`ring` route, and a second oracle to write by hand (there is no posit library the tree may depend on:
"no dependency outside the tree", ROADMAP.md:12-13). No gate in the tree has asked for the range. A
rational pair `(num, den)` is exact but needs gcd normalisation per op and overflows in a few multiplies;
not a kernel format. **Verdict: posit does NOT earn its complexity over Q32 in this tree today.** If a
workload needs the range, posit is a LIBRARY (`selfhost/std/posit.bp`, a `fn` per op over i64, the exact
shape fp.bp has) with a hand-written python oracle -- zero compiler change, zero F4/F5/F6 change.

### Deterministic fixed point, Q31.32, with a hardware multiply (recommended)

The existing format, with one change: a `smulh`/`umulh` builtin (already on A9's list,
docs/blueprints/A9-neon-builtins.md:3 "`umulh` any time") lets `fp_mul` become a ~10-word straight line
that is BIT-IDENTICAL to today's limb form and to F9's spec:

```
cmp  xa,#0 ; cneg xa,xa,lt        ; |a|
cmp  xb,#0 ; cneg xb,xb,lt        ; |b|
umulh xh,xa,xb ; mul xl,xa,xb     ; 128-bit |a||b|
extr  xr,xh,xl,#32                ; floor(|a||b| / 2^32)   (one word: (hi:lo) >> 32)
eor  xs,a,b ; cmp xs,#0 ; cneg xr,xr,lt   ; sign
```

The sign-magnitude form matters: a plain `smulh/mul/extr` on the signed operands gives
floor-toward-minus-infinity, which differs from `fp_mul` by 1 ulp on negative products with a nonzero
low half (e.g. a = -1, b = 1: fp_mul gives 0, the signed form gives -1). Keeping truncation-toward-zero
keeps the 25 oracles' values and F9's statement unchanged. **Every word above is asm text to be derived
by as+objdump before it is typed (AGENTS L1, tools/check_words.py:1-6); I have not derived them.**

## 2.3 (b) The minimal honest design in this compiler's shape

1. **A9 builtins `umulh(a,b)` / `smulh(a,b)`** (register-parameterised single words, the `umulh` row A9
   already costs); dispatch arm in `emit_call_or_ctor` (bebop.bp:1865) = +1 b.cond each (the crc32b
   precedent, census_allow.txt line `bebop bcond 1213`); T122 table (bebop.bp:6160-6167) + typecheck
   BUILTIN + bpref + LANGUAGE.md, gated by tools/builtin_surface.py. Then `fp_mul_hw` is a 10-line fn in
   fp.bp, gate = bit-equality with `fp_mul` on fp.bp's own 1.2M-value method plus every fp std gate unchanged.
2. **A8 tag 6 `fp`** on params, returns and `let` (the 4th stab cell, docs/blueprints/A8-typed-tables-u32.md:19,
   §4); `fp` + `i64` mixing -> exit 84 (A8's code), mirrored in typecheck.py.
3. **Literal `1.5`**: `read_num_val` gets a fraction branch taken only when a DIGIT RUN is followed by
   `.` and a digit -- no clash with `.f`, which follows an identifier or `)` (bebop.bp:4356-4361); value =
   round-to-nearest of the decimal to Q31.32 as a CONST tag (0 words until materialised, 1-4 words
   `movz/movk` when it is). `.5` (no leading digit) refused with a diagnostic (a free code below 99,
   docs/TRAPS.md free list 65-79 minus 65 now taken by F3). bpref: Python `Fraction` -> exact Q32 round.
4. **Operator dispatch in `vs_binop`** on two fp-tagged operands: `+ - < <= == !=` unchanged (0 new
   words: Q32 add IS integer add; ordering IS integer ordering); `*` emits the 10-word inline form (or a
   `bl fp_mul_hw`); `/` -> `bl fp_div` (a library fn: 128/64 division has no AArch64 instruction, so it
   stays a ~64-step loop, M12) or refused with a diagnostic until a gate needs it. `%`, `<<`, `>>`, `&`
   on fp -> exit 84.
5. **Nothing else changes**: no second register file, no new spill class, no calling-convention change
   (fp values ARE i64s in x0..x13), no store layout change (LANG-DB-DESIGN:154 already tags fp), no GC,
   no arena use. The ABI invariant (x27/x28 clean, x9-x13 allowlisted, tools/check_abi.py:14-25) is
   untouched because no new emitter writes those registers.

Q32 is NOT retired; it is PROMOTED from a prelude convention to a typed value. `fp.bp` keeps `fp_mul`
(the limb form) as the oracle twin of `fp_mul_hw` until F9 proves them equal, then the limb form is
attic'd. money.bp needs nothing: it is i64 minor units (selfhost/std/money.bp:3-21); the "exact-equality
discipline" is served by fp being deterministic integers.

## 2.4 (c) Cost

| unit | fp (Q32 typed) | posit in the compiler (NOT recommended) |
|---|---|---|
| words in bebop.bin | **ESTIMATE** +150-250 (two builtin emitters ~20 each, literal branch ~40, dispatch ~60, diag arm ~30) | +400-700 (two ~80-130-word op emitters or their call plumbing, the same literal/dispatch/diag) |
| words per program | 0 for `+ - cmp`; 10 per fp `*` site (or 6-10 for a `bl`); 1-4 per literal | 6-10 per op site + 200-250 once for the two op fns |
| new b.cond (census frozen at 1223) | +2 builtin arms, +1-2 dispatch arms, +1 diag arm = **+5-6**, each an allow line | +8-12 |
| lines of bebop.bp | ~200-300 | ~500-800 (an op emitter is em() word blocks; the A9 `scan` builtin is 46 emitted words for a simpler loop, tools/typecheck.py:26-27) |
| Lean | +1 function, +1 bridge theorem; F9 statement unchanged | +2 BitVec functions (~150 lines), F6 `ring` route closed |
| weeks | **2-3** (0.5 builtins, 1 tag+literal, 1 dispatch+constructs), single-stage bootstrap (the compiler need not use fp) | 4-6, and an oracle written from the standard by hand |

For scale: the roadmap costs capture-inference closures at ~2 weeks and a table-driven front end at
~2,000 lines (ROADMAP.md:113); fp-typed Q32 is a closure-sized item, posit-in-compiler is a front-end-sized one.

## 2.5 (d) What it breaks -- and the speed conflict the operator must be told

- **Constructs:** 0 of 115 use fp; WORD_DELTA 0 on all of them is the gate. New: `c_fp_ops` (literal,
  `* / + cmp`, negative products at the truncation boundary: a = -1, b = 1 must give 0), `neg/c_fp_mix`
  (COMPILEFAIL:84), `neg/c_fp_dotlit` (`.5`).
- **Gates:** 25 Q32 oracles and 20 std programs are unchanged by construction (bit-identical multiply);
  the fp std gates are the regression net.
- **Invariants:** gen3 == gen4 not threatened -- the compiler does not use fp (single-stage). Census:
  +5-6 b.cond with allow lines. `builtin_surface: 36 of 36` becomes 38 of 38 or goes RED.
- **bpref / typecheck:** both model it exactly (Python ints). No DIVERGE class is un-adjudicable.
- **THE SPEED GATE CONFLICT (addendum item 5).** TG-DONE row 1 requires every honest row <= 2.0x its
  Rust twin (ROADMAP.md:51,60-62), and K2H/K3H were 3.3x/3.4x on 2026-09-06 (now 2.6x/2.4x after B1,
  REPORT-honest.md:33) BEFORE any fraction arithmetic. **ESTIMATE, unmeasured:** on A78 an f64 `fmul` is
  3-4 cycles latency at 2 per cycle; the 10-word Q32 form is ~7-8 cycles on the dependent path and 10
  issue slots, so a multiply-dense fp kernel reads **~2-4x latency-bound, up to 8x throughput-bound**
  against an f64 twin; a posit software multiply at ~40-60 dependent cycles reads **~15x latency-bound,
  50-100x throughput-bound**. Add/compare-dense fp kernels read 1.0x (one word either way). So: **no
  software fraction format can meet the 2.0x gate against a hardware-f64 twin on a multiply-dense
  kernel; posit cannot meet it on any kernel with a multiply in the loop.** This is a roadmap conflict,
  not a detail: either (a) the fp honest row's twin is Rust doing the SAME integer Q32 arithmetic (fair,
  ~1.0-1.5x reachable, f64 reported alongside as report-only the way D1(a)'s 1.0x is, ROADMAP.md:62), or
  (b) the operator accepts hardware IEEE for speed and pays §2.2's verification bill. There is no (c).
  The measurement that settles the numbers is §7 probe 5 (`kfp`).

---

# 3. STRINGS AS VALUES (zero-allocation form)

## 3.1 (a) What exists today

- `str` is a raw byte pointer: `emit_str` emits `adr x0,<imm>` to literal cells appended after the code
  (bebop.bp:4018-4035; cells are packed 8 bytes per cell, `ceil((len+1)/8)` cells, bebop.bp:6322-6324);
  `str_len` is a NUL scan (bebop.bp:1792-1806); `char` is `ldrb` on the pointer (:1809-1823). "`"..."` is
  only valid as an argument" (docs/LANGUAGE.md:89). `++` exits 96 in `emit_expr` (bebop.bp:3616-3623; the
  table entry :3672; docs/TRAPS.md row 96) and is guarded by neg/c28_plusplus.bp and diag_neg/d03_plusplus.bp.
- **The A7 step 2 handle migration `(off << 32) | len` landed consumers without the producer and broke
  self-hosting; reverted 2026-09-12; owed as a WHOLE re-land** (ROADMAP.md:94: "nothing copies literal
  bytes into the arena ... every generation built from the current source cannot read a single string
  byte"; also the literal table collision `fntab[7000+i]`/`[7016+i]` sixteen apart). The blueprint is
  docs/blueprints/A7-byte-arena-and-str-values.md §3 (byte region inside the x17 reserve, bytes allocated
  as `zeros(ceil(len/8))`, `char` = 3 words, `str_len` = 1 word `and #0xffffffff`, substr as a pure-integer
  idiom, bpref as one bytearray).
- **bpref is ALREADY on the handle**: `('str', v)` returns `((off << 32) | len)` over `self.bytes`
  (tools/bpref.py:574-579), `str_len` = `s & 0xffffffff`, `char` = `bytes[off+i]` (:603-609), `crc32b` over
  the byte range (:620-625). typecheck.py has `str` as a type and flags `str` in arithmetic and `str`
  indexing (tools/typecheck.py:6, :64-67, :93). The Lean model has `Ty.str` (Basic.lean:47-52) but no
  byte memory and `stringConcat -- exit 96` as a trap code (Basic.lean:183).
- **The house already writes strings the addendum's way, by hand, in at least four places:**
  `diag_str(buf, at, m: str)` copies a literal into a caller-owned cell buffer and returns the new end
  (bebop.bp:66-71); `put_num` writes decimal digits into the same buffer (:46-65) and `diag_exit` chains
  them into a 256-cell buffer for one `sys_write` (:72-111); `qdsl_explain*` write bytes into `buf` at
  `bp[0]` one cell per character (selfhost/std/qdsl.bp:328-345, :379-470) and `qdsl_int_to_str` is a
  second copy of `put_num` (:472-492); `str_to_cells` (bebop.bp:6812-6824) and `use_append_str` (:6882)
  are the same loop again; the attic holds a third `int_to_str` written against a `++` that no longer
  exists (selfhost/std/attic/fmt.bp:33-45, attic/README.md:8 reason class **str**: 5 modules). And
  `qdsl_explain(query: str) -> str` returns a FORGED handle `((bp[0]-1) << 32) | (bp[0]-1)` because there
  is no way to return the buffer it built (qdsl.bp:494-508).
- Bytes-in-cells is 8x memory on every IO path (A7 blueprint §0); `c_st_bytes` and `st_bytes`
  (selfhost/prelude/store.bp:48) extract raw bytes from cells for the store's crc.
- The fuzzer never generates strings: "no str literals / ++ (R3.x d)" (bench/fuzz/gen.py:9).

## 3.2 (b) The minimal honest design: a `str` is a VIEW, a `[u8; N]` is the BUFFER

The addendum answers the allocation question: there is no `a ++ b`. What the language adds so the
hand-rolled form stops being hand-rolled:

1. **The view** (A7 step 2, re-landed whole): `str` = `(byte_off_from_x17 << 32) | len`, an ordinary i64
   in an ordinary register. Literals become CONST handles, so `let s = "abc"` is legal (today only an
   argument position is, LANGUAGE.md:89). `str_len` 1 word, `char` 3 words, slicing is the documented
   pure-integer idiom `((s >> 32) + i) << 32 | n` (blueprint §3; c68_strval already asserts the arithmetic,
   bench/parity_constructs/c68_strval.bp:14-20) -- or a 3-word builtin `slice(s, i, n)` if the idiom is
   judged too agent-hostile.
2. **The buffer** (A8 tag 7 + F3 arm ii at width 1): `let b: [u8; 256] = bytes(256)` -- `bytes(n)` is
   `zeros(ceil(n/8))` returning a handle `(off << 32) | n` (A7 blueprint §3 "Allocation of bytes =
   zeros(ceil(len/8)) cells"); `b[i]` on a u8-tagged symbol is `ldrb` (3 words, the same form as `char`),
   `b[i] = c` is `strb`; the DECLARED length 256 goes into the F3 side channel (`fntab[5410+lk]`,
   ROADMAP.md:112) so every literal index is discharged at zero words and the runtime length lives in the
   handle's low 32 bits for the rest. **Where the length lives, answered:** statically in the tag side
   channel when declared, dynamically in the handle always; there is no header cell in the arena (a
   header would be a second place for the same number, and the store's object header already has one,
   LANG-DB-DESIGN:146-152).
3. **Loops write, views read**: `put_num(b, at, v) -> at'`, `put_str(b, at, s) -> at'`, `str_eq(a, b)`,
   `str_cmp(a, b)` in a new `selfhost/prelude/fmt.bp` (the existing `diag_str`/`put_num` bodies moved out of
   bebop.bp and the qdsl copies deleted), and `sys_write(fd, s)` accepting a handle (bpref already does,
   bpref.py:611-618). `str_eq` as a builtin is optional: ~20 words of `ldrb` loop; as a prelude fn it is 0
   compiler words. With A9's `scan` (46 words, already landed for class scans) the parser-side loops get
   NEON later without touching the design.
4. **In the store**: a string column is `[len_cell, ceil(len/8) packed cells]` inside the object payload,
   and a `str` FIELD is an object-relative handle `(obj_rel_byte_off << 32) | len` -- the same "no pointers
   on disk" rule as `ref T` (LANG-DB-DESIGN:155-158); `st_get_str`/`st_put_str` resolve it against the
   object base exactly as `st_get` does `base[obj + 2 + i]` (store.bp:862). **Cost of a string column:**
   `1 + ceil(len/8)` cells, i.e. 8 B + len rounded up to 8 -- against 8*len today with bytes-in-cells, and
   against sqlite's ~1-2 B varint + len; G7's "2.5x size loss" (ROADMAP.md:57) is where the byte-per-cell
   waste shows and this is the row that removes it for text columns.
5. **Refused, deliberately**: `++` (stays exit 96; c28/d03 unchanged), any implicit allocation, ropes,
   interning, a string heap. Building a string means writing into a buffer whose length the author
   declared; running past it is F3's trap (static when the index is literal, runtime when not), which is
   the loud failure the culture wants (AGENTS L1).

The Lean/F4 rule for `char` keeps ONE memory: `char(s, i) = (cells[(off+i) / 8] >>> (8 * ((off+i) % 8))) & 255`
over the same `Arena.cells` (Basic.lean:157-165). No second array, no ByteArray: this is what makes
"persisted objects ARE the in-memory objects" true for text as well.

## 3.3 (c) Cost

| unit | S0 (A7 s2 re-land) | S1 (u8 tag + forms + declared length) | S2 (views: slice/eq/cmp/write) | S3 (store column) | S4 (fmt.bp, hand-rolls retired) |
|---|---|---|---|---|---|
| words in bebop.bin | +2 per `char` site in the compiler's own parser: blueprint §7 expects **+3-5 % (~+1,300-2,100)**; K5 gate <= 1.10x; the A9 `scan` builtin is the named compensation | **ESTIMATE** +150-250 (ldrb/strb forms in `emit_array_index`/set, tag checks) | +40-80 if `slice`/`str_eq` are builtins, 0 if prelude | 0 (store.bp is a library) | **-200..-400** (diag_str/put_num move out; the A17 measurement was -1022 for making 15 texts literals, census_allow.txt `bebop cbz 125`) |
| new b.cond | 0 -- the A7 line says the revert/re-land is branch-neutral (census_allow.txt `bebop bcond 1214`) | +4-6 (impure guarded arms per form; A5 step 1b's index fast path cost +3) | +1-2 per builtin arm | 0 | negative |
| lines of bebop.bp | ~200 (the blueprint's step 1, re-done whole) | ~150-250 | ~60-120 | store.bp ~80 | net negative in bebop.bp, +100 in fmt.bp |
| weeks | 1-2 (it has been done once and reverted; the literal table must be redesigned) | 1-2 (rides A8 step 2) | 1 | 1 | 0.5 |

Total ~5-6 weeks, of which 1-2 (S1) are A8 work that A8 would do anyway for `[u32]`.

## 3.4 (d) What it breaks

- **Constructs**: the 22 string constructs are re-frozen at S0 (WORD_DELTA on every `char`/`str_len`
  site; VALUES unchanged, c68_strval's header says it is written to survive the migration); c28/d03 stay.
  New: `c_u8buf` (write digits, read back, `str_len` of a view), `neg/c_u8_oob` (COMPILEFAIL:65 on a
  literal index past a declared length -- F3's own code), `c_str_store` (roundtrip at two mapping bases,
  the G2 method), `neg/c_str_plusplus` is c28 already.
- **Gates**: hex/base64/morph/c14_string/c50_cas and every std gate that reads argv or a file through
  `char` (the blueprint's parity list §3); the G-store gates are unchanged until S3 adds a byte column.
- **Invariants**: the fixpoint is THE risk -- the last attempt lost it (ROADMAP.md:94). The gate for S0
  is `chain` GREEN with gen3 == gen4 and `fn main(){char("A",0)}` printing 65 from gen2, which is the
  probe that found the defect. Census: 0 for S0 (measured last time), +4-6 for S1 with allow lines.
- **bpref**: 0 change for S0 (already handles) -- and that is itself a finding: today's agreement is
  representation-blind, so a construct that packs a handle by hand (`(128 << 32) | 5`, c68) agrees only
  because both sides never dereference it. typecheck: +1 tag. Lean: one `char` rule over packed cells.
- **A24 step 2 is NOT unblocked by strings alone** (correction to the brief): its blocker is the range-copy
  primitive plus a decimal printer in the GENERATED program; S4's `fmt.bp` supplies the printer via a
  `use` line the rewriter appends, A25 supplies the copy. It is unblocked by A25 + S4 together.
- **The table-driven front end (~2,000 lines, ROADMAP.md:113)**: strings do not make it cheaper, and it
  is still not worth scheduling; what A25's flattener gives instead is ONE program text for the four
  implementations, which removes the part of the "four implementations" problem that matters here (the
  grammar delta of modules) at ~1/6 of the cost.

---

# 4. MODULE CONTENTS (namespace flattening at elaboration)

## 4.1 (a) What exists today

- `module := 'module' NAME '{' '}' -- inert` (docs/LANGUAGE.md:21); 43 files and the compiler itself
  open with `module core { }` (bebop.bp:17). **The compiler does not parse `module` at all**: `grep -n
  module bebop.bp` hits only line 17 and two comments (:6910, :6971); `collect_fns` scans the whole source
  for `fn ` at ANY brace depth, skipping only strings, `//` comments and (since A24 s1) `test` blocks
  (bebop.bp:5978-5999, ROADMAP.md:110); `find_struct`/`find_enum` likewise. **MEASURED:**
  `module m { fn f() -> i64 { 7 } } fn main() -> i64 { f() }` compiles rc=0 on the promoted binary and
  prints 7 -- `f` is GLOBAL, module braces are invisible text. bpref, by contrast, skips the whole block
  (tools/bpref.py:257-258 `skip_block`) and answers `UNSUPPORTED:builtin f` on the same file -- **a
  compiler/oracle disagreement, today, on a form both claim to accept** (`bpref_parity.sh` has not seen it
  because no construct puts a `fn` inside `module { }`). Also MEASURED: `m::f()` exits 101 "unbound
  symbol" at 1:20 -- loud, so `::` is free to take.
- `use "path"`: textual inclusion once, dependencies first, content-hash dedup, nested since T47b, the
  expansion written to `<out>.use` (LANGUAGE.md:31-33; bebop.bp:6857-6870 comment; `use_scan` :6979,
  `use_expand` :7030-7049 builds one byte buffer, exports it, slurps it back so "every later stage sees
  one string"). `use "cas://sha256:<hex>"` resolves to `.bcas/<hex>.bp` and `cas_verify` exits 88 on a
  digest mismatch (bebop.bp:6946-6974; two blobs in `.bcas/` today).
- Names: `read_ident` returns `h = h*131 + c` wrapping (bebop.bp:34, :151-163); fns are `[cnt, names...,
  offsets..., srcpos...]` in fntab (bebop.bp:6270-6273), `fntab_lookup` is a linear name scan with
  "last match wins" and the comment records that two `fn` DEFINITIONS CAN share a spelling today
  (bebop.bp:673-688, :700-704) -- i.e. a cross-file name collision is silent. `module` IS in the T122
  reserved table (hash 4238080694376 present in the `rsv` lines, checked today); `as`, `mod`, `import`,
  `use` are not. The fn cap is 768 (bebop.bp:6356-6360), budget 547 measured (ROADMAP.md:104); the
  planning facts are keyed by SOURCE POSITION precisely because names alias (:700-712).
- The `.use` file is already the text every downstream reads: diagnostics count line:col over it
  (bebop.bp:43-44); A24 s1 made bpref erase `test` the way the compiler does so that the two "agree
  about what a program contains" (ROADMAP.md:110).

## 4.2 (b) Design: `mod_expand`, a textual pass, zero words

Answer to the naming question: **in a one-pass compiler with a hashed, position-keyed fn table, a module
system is qualified-name REWRITING, not a symbol-table redesign.** Nothing in `fntab` changes.

- Syntax: `module m { <fns, structs, enums> }` at top level (one level; no nesting); references `m::f(...)`.
  `::` is lexically free (`:` occurs only in type annotations, LANGUAGE.md:18-22) and bpref already tokenises
  it (`UNSUPPORTED::` today). `.` is NOT available: it is postfix field access (bebop.bp:4356-4361).
  `use "path" as m` (and `use "cas://sha256:..." as m`) wraps the included file as `module m { ... }`.
- `mod_expand` (A25 driver): for each module, collect its fn/struct/enum names (`collect_fns` over the
  range), then (1) strip `module m {` and its `}` (positions preserved by commenting, as `use_comment`
  does, bebop.bp:7039), (2) prefix every definition `fn f` -> `fn m__f`, (3) rewrite every `m::f` anywhere
  to `m__f`, (4) rewrite every UNQUALIFIED call `f(` INSIDE the module range whose hash is in the module's
  table and NOT in the builtin ladder to `m__f(`. Output: the `.use` file. The emitter, bpref, typecheck
  and the Lean harness see a flat program with unique names. Diagnostics keep their positions (the rewrite
  changes lengths; A25 must carry a position map or pad -- `m__f` is exactly as long as `m::f`, which is
  a reason to pick `::` and a two-character prefix separator).
- **The 768 cap**: flattening does not multiply fns (each definition once); the count that matters is
  the CONCATENATION's, which the cap already measures. Monomorphisation (A16 s4) is what multiplies, and
  it is the same driver family, so its instances count against the same 547 budget -- state it in A16 s4's
  gate rather than discovering it.
- **`use_expand` reuse / A16 s4**: one mechanism wearing three hats (modules, mono, test) -- §1.2. They
  should land as A25 + three thin drivers, not as three rows.
- **T122**: `module` already reserved; `as` must be added if it becomes a keyword (one hash, 0 branches --
  the F2-prerequisite measurement found the reserved-table adds cost zero, census_allow.txt `bebop bcond 1194`).
- **F6's tcheck.bp "own 511-fn cap"**: unaffected; it is compiled from flattened text like any program.
- **CAS discipline survives unchanged**: the digest is over the file's bytes and the alias is local to the
  importer, so `use "cas://sha256:<hex>" as m` verifies exactly as today (bebop.bp:6946-6974) and the
  same module imported under two aliases is two prefixes over ONE deduped inclusion -- which the seen
  table must key by content hash (as it does) and the prefix step by alias.

## 4.3 (c) Cost

Modules themselves: **ESTIMATE** 0 words in any program; +30-60 words in the compiler outside A25 (the
`as` parse in `use_scan`, one diag arm for `m::g` unresolved with a position); +2-3 b.cond; ~80-120 lines
on top of A25's 250-400; 1 week on top of A25's 2. Compared with the roadmap's units: A25 + modules is
one capture-inference-closure (~2 weeks) plus a few days.

## 4.4 (d) What it breaks

- Constructs: 9 use `use`; 0 use module contents; c44_use24/c47_usenest (`use` dedup and nesting) are
  the regression net. New: `c_mod2` (two modules each defining `fn f`, distinct results), `c_mod_cas`
  (`use "cas://..." as m`), `neg/c_mod_unresolved` (COMPILEFAIL with a position), and a `.use`
  byte-identity twin.
- Gates: none change value. Invariants: gen3 == gen4 unaffected (the compiler will not use modules
  until generation N+1; the 43 `module core { }` lines are legal under the new grammar as empty
  modules). Census: +2-3 with allow lines.
- bpref/typecheck/Lean: **0 change if they read the `.use`** -- and they should; the alternative is a
  Python flattener (a fourth copy). The one thing to fix regardless is the latent disagreement above:
  either the compiler REFUSES `fn` inside `module { }` until modules land (a neg construct, the F8 step-0
  shape), or bpref stops skipping the block.

---

# 5. Costed ladders, with gate numbers (house style: every step is a number in a committed script)

## 5.1 Floats (fp = typed Q31.32)

| step | lands | buys | gate (the number) |
|---|---|---|---|
| **FP-1 (WAVE 0, now)** | `neg/c_fp_dotlit`: `let x = 1.5` must COMPILEFAIL with a code from the free list | closes a MEASURED silent-garbage acceptance (§2.1) | `neg/c_fp_dotlit` COMPILEFAIL:<code>; `diag: N+1 pass 0 fail`; the same construct is later re-pointed at the fp literal rule (FP1) |
| FP0 | `umulh`/`smulh` builtins (A9 row) + `fp_mul_hw` in fp.bp | the 10-word multiply; F9's proof target becomes the 128-bit product | `fpmul_hw_eq: 1200000/1200000` (fp.bp's own value set, bit-equal to `fp_mul`); `builtin_surface: 38 of 38`; std_golden 117/117 unchanged; census +2 b.cond with allow lines; `.bin` of fp.bp+main shrinks from the measured 1316 B with a word_budget line |
| FP1 | A8 tag 6 on params/returns/lets; literal `1.5`; exit 84 on mixing | typed fp with zero words in programs | `typecheck_gate: 0 findings both sides`; `c_fp_lit` MATCH; `neg/c_fp_mix` COMPILEFAIL:84; `neg/c_fp_dotlit` COMPILEFAIL:<free code>; WORD_DELTA 0 on 115/115 |
| FP2 | `vs_binop` dispatch: `*` inline, `/` -> `bl fp_div`, `+ - cmp` untouched | `a * b` on fp values | `c_fp_ops` MATCH incl. `(-1) * 1 == 0`; fixpoint gen3 == gen4; census +2-3 allow lines |
| FP3 | honest row `kfp` (Q32 axpy + dot, 10^6 elements) vs Rust-Q32 AND vs Rust-f64 | the speed number the operator must see | `kfp <= 1.5x` vs the Q32 twin (the gate); `kfp_f64` REPORTED (report-only, D1(a) shape) |
| FP4 (only if a workload needs 2^248 range) | `selfhost/std/posit.bp` + hand-written oracle | posit as a library | `posit: N/N` ops vs oracle; 0 compiler change |
| F4/F9 | +1 Lean function, +1 bridge theorem | -- | `theorems >= 3` unchanged in statement |

## 5.2 Strings

| step | lands | buys | gate |
|---|---|---|---|
| S0 | A7 step 2 re-landed WHOLE (producer copy, non-colliding literal table, every consumer) | the byte region; the view | `chain` GREEN, gen3 == gen4; `fn main(){char("A",0)}` = 65 from gen2; c68_strval and the 22 string constructs re-frozen with WORD_DELTA lines; `K5 <= 1.10x` |
| S1 | A8 tag 7 `[u8]` + `bytes(n)` + `ldrb/strb` forms + F3 declared length `[u8; N]` | buffers with static bounds | `c_u8buf` MATCH; `neg/c_u8_oob` COMPILEFAIL:65; `bounds_census` counts byte sites; WORD_DELTA 0 on 115/115 |
| S2 | `slice`/`str_eq`/`str_cmp` (prelude or builtin) + `sys_write(fd, s)` on a handle | reading strings without hand loops | `c_str_views` MATCH; `bpref_parity: disagree=0` |
| S3 | store byte column + object-relative `str` field | text in the database | `sstr` G-gate: roundtrip byte-identical at two mapping bases; G7 size row re-run and REPORTED |
| S4 | `selfhost/prelude/fmt.bp` (`put_num`, `put_str`); bebop.bp's `diag_str`/`put_num` moved out; qdsl copies deleted; `qdsl_explain` returns a real view | the hand-rolls retired | `arch_check` dead-function ratchet unchanged; `diag: 19 pass 0 fail`; c70_qdsl re-frozen with a real fold; bebop words DOWN with a word_budget line |

## 5.3 Modules (with A25)

| step | lands | buys | gate |
|---|---|---|---|
| M0 (WAVE 0, now) | neg construct: today's `fn` inside `module { }` REFUSED (or bpref aligned) -- MEASURED disagreement, §7 probe 1 | the disagreement closed before it costs a lane four days the way the `set`-node copy did (ROADMAP.md:108) | `bpref_parity: disagree=0` including the new fixture; a COMPILEFAIL code if refused |
| A25 | `src_copy_range` + `src_rename_idents` + the `.use` driver skeleton | one rewriter | `.use` byte-identity on 3 fixtures; `diag_check` positions correct after expansion |
| M1 | `mod_expand`: `module m { }` with contents, `m::f`, prefixing | modules | `c_mod2` MATCH; `neg/c_mod_unresolved` COMPILEFAIL:<code> with line:col; `.use` twin byte-identical; census +2-3 |
| M2 | `use "path" as m`, `use "cas://..." as m` | aliased imports | `c_mod_cas` MATCH; `neg/c51_casbad` still 88 |
| A16 s4 / A24 s2 | `mono_expand` / `test_expand` as drivers | generics with width, test blocks | their own rows' gates, unchanged, now ~2 days each |

---

# 6. What I would NOT do, with reasons (adding to the roadmap's declined list, L23 form)

1. **IEEE-754 in the language** -- not for hardware chaos (basic ops are deterministic on one ISA, §2.2)
   but because it adds a second register file to a compiler whose whole allocator is x0..x26 (bebop.bp:236-253,
   REGISTER-MODEL-BLUEPRINT §1.1), an axiom between Lean and the FPU that nothing in-tree can check, and a
   bpref that can only agree vacuously. If speed forces it (§2.5 option b), it should be T87's shape --
   at the IO boundary, converting to Q32, no FP arithmetic word emitted (HISTORY.md:2259-2265).
2. **Posit in the compiler** -- ~10x the words per multiply of the Q32 hardware form, 30-100x the cycles
   of f64, closes F6's `ring` route, and no gate has asked for its range. As a library, fine, later.
3. **A quire** (1024-bit accumulator for posit64) -- 16 cells of ripple-carry per accumulate; it exists
   to make dot products exact, and a Q64.64 double-cell accumulator does that in ~4 words if ever needed.
4. **Q32 with a per-declaration scale (`fp<16>`, `fp<48>`)** -- 25 oracles bake `>> 32`; a second scale
   is a second oracle family and a second set of theorems. Decline until one gate needs it.
5. **`++`, ropes, interning, a string heap, string GC** -- the addendum's ban; c28/d03 stay as the guard.
6. **Strings as bytes-in-cells columns in the store** -- 8x the bytes; S3's packed column instead.
7. **A `str` header cell in the arena** -- the length already lives in the handle and, when declared, in
   the F3 side channel; a third copy is the kind of duplicated fact that made the +21 PartTab cell turn
   five gates red (ROADMAP.md:112).
8. **Nested modules, `pub`/private, re-exports, `use m::*`** -- one level of prefixing does what the 43
   `module core { }` files and the fntab collision problem need; each extra feature is another rewrite
   rule the four implementations must agree on.
9. **A Python flattener for bpref** -- read the `.use`; the compiler writes it already.
10. **Runtime linking of any kind** (dlopen, a module table in the `.bin`) -- the flattening produces
    zero words; anything at runtime would need `blr` sites (A16 s1's `call_fn`) and a loader.
11. **The table-driven front end (~2,000 lines)** -- still declined; A25 buys the piece that matters.

---

# 7. Probes: four RUN, two NOT run, with commands and what each settles

Copies of `seed/build/seed`, `bebop.bin` (the promoted 2026-09-13 09:00 binary) and
`selfhost/prelude/fp.bp` plus the probe sources and outputs are in
`/tmp/claude-0/-root/a92ed05c-8ad3-4bf1-9531-0725903df6e2/scratchpad/probe/`; `run.sh` compiles and runs
m1 m2 m3 z0 z1 and was executed as `tools/slot.sh probe-lang-research bash <dir>/run.sh` at 12:59:58
(slot acquired and released in 0 s). Nothing under /root/dowiz was touched.

| # | probe | compiler (MEASURED) | bpref (MEASURED) | settles |
|---|---|---|---|---|
| 1 | `m1.bp` = `module m { fn f() -> i64 { 7 } } fn main() -> i64 { f() }` | rc=0, prints **7**, 752 B | `UNSUPPORTED:builtin f`, rc=3 | module braces are invisible to the compiler; the oracle and the compiler DISAGREE today; M0 is a real neg/alignment item |
| 2 | `m2.bp` = `fn main() -> i64 { let x = 1.5; x }` | rc=0, prints **495807598623**, 744 B | `UNSUPPORTED:.`, rc=3 | silent garbage acceptance -- FP-1 goes in WAVE 0 |
| 3 | `m3.bp` = `fn main() -> i64 { m::f() }` | rc=101 `1:20: error[E101]: unbound symbol` | `UNSUPPORTED::`, rc=3 | `::` is lexically free and loud today |
| 4 | `z1.bp` (fp.bp + `fn main(){ fp_mul(3<<32, 5<<32) >> 32 }`) vs `z0.bp` (`fn main(){ 3 }`) | 1316 B vs 724 B -> **148 words** for `fp_mul` + `isqrt` + the call; prints 15 | -- | the limb form's real size; the hardware form's 8-10 words is ~10x smaller |

NOT run (need code that does not exist yet, or a Lean run off-box):

- 4b. `fp_mul_hw` word count once `umulh` exists (diff the same two files again).
- 5. `kfp`: a Q32 axpy/dot honest row in bench/vs_rust/honest.sh against a Rust twin in TWO arms (i64
  Q32 and f64). **This is the measurement that decides §2.5; nothing in this document has decided it.**
- 6. The `bv_decide` pre-measurement the design study already asked for (RESEARCH-PROOF-DESIGN:285) on
  `fp_mul_hw` vs `fp_mul_spec` -- if it closes in minutes, FP0's bridge theorem is a SAT certificate and
  the `ring` rule is not on FP's path at all.

---

# 8. Open questions for the operator, each with a recommendation

1. **Which twin for the fp honest row?** Rust-f64 (hardware; no software format can reach 2.0x on a
   multiply-dense kernel) or Rust-i64-Q32 (the same arithmetic). **Recommend Rust-Q32 as the gate and
   f64 as a report-only column**, the D1(a) precedent (ROADMAP.md:62). If the answer is f64, the honest
   consequence is §6 item 1's boundary form, not posit.
2. **Posit at all?** **Recommend: not in the compiler; a library only when a gate needs 2^248 range.**
   Fixed point already has the bit structure, the theorem and the oracles.
3. **Rounding of fp `*`**: truncation toward zero (bit-identical to `fp_mul`, F9 unchanged, 25 oracles
   unchanged) vs floor. **Recommend truncation**, at the cost of 4 extra words per multiply.
4. **Fraction literal syntax**: `1.5` only when digit-led. **Recommend yes; refuse `.5` with a code.**
5. **String buffer syntax**: `[u8; N]` (F3's tokens) vs `str<N>`. **Recommend `[u8; N]`** -- it IS F3's
   mechanism and typecheck.py can type it with one entry.
6. **Slicing**: documented integer idiom vs a `slice(s, i, n)` builtin. **Recommend the builtin** (3
   words, one dispatch arm): the idiom is the kind of thing an agent gets wrong by one shift.
7. **Module reference syntax**: `m::f` (lexically free) vs `m.f` (collides with field access). **Recommend
   `::`**, pending probe 3; also `m__f` as the flattened spelling so positions do not shift.
8. **Does every implementation read the `.use`?** **Recommend yes, binding**: bpref, typecheck.py and
   the Lean conformance harness consume the flattened text; the flattener is written once in Bebop.
9. **A7 step 2 re-land position**: head of WAVE 2 (before A8) vs "close A7" at its end. **Recommend the
   head** -- it is the producer for both string rows and A8's tag 2/5/7 forms.
10. **A25 as its own row before A16 s4 / A24 s2 / modules?** **Recommend yes**; without it three
    textual rewriters get written.
11. **Close the two MEASURED silent acceptances now (M0: `fn` inside `module { }` prints 7 while bpref
    refuses; FP-1: `1.5` prints garbage), before any of the three features?** **Recommend yes, in WAVE
    0** -- both are the F8-step-0 shape the execution order already puts first: "every one of them makes
    a FAILURE visible that is currently silent" (ROADMAP.md:113).
12. **Should the compiler itself use fp/strings/modules (two-stage bootstrap)?** **Recommend no for fp
    and modules, yes for S4** (`diag_str`/`put_num` move to the prelude, the A17 direction, bebop words
    go DOWN) -- single-stage for the rest keeps gen3 == gen4 an invariant rather than a goal (ROADMAP.md:52).
