# REPORT-honest — the D1(a) column (bench/vs_rust/honest.sh, D11-C)

Status: 2026-09-06 CURRENT (first committed run; rerun after every codegen step, replace this file)

# honest twins (D11-C), in-process pinned core 4, R=11, REPS=100 per run, bebop.bin 94e47998
| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | target >= 1.0x (T83) |
|---|---|---|---|---|
| K1H | 2.07 / 2.15 | 1.132 / 1.189 | 1.8x | UNMET |
| K2H | 1.30 / 1.45 | 0.397 / 0.438 | 3.3x | UNMET |
| K3H | 0.65 / 0.72 | 0.194 / 0.272 | 3.4x | UNMET |
| K4 | 5.26 / 5.42 | 3.259 / 3.299 | 1.6x | UNMET |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.67 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

Method (2026-09-06): every kernel runs REPS=100 reps in-process and returns the TOTAL
clock_ms (1 ms coarse) — the table divides by 100, so a row has 0.01 ms resolution; the
Rust twins run the same 100 reps with the carried accumulator through black_box at every
rep boundary and print ms per rep from Instant. K4 got an honest twin (rust_once/k4h.rs,
black_box only on input/output) — 3.26 ms here vs 2.85 ms for the old black_box-in-loop
k4.rs, so the old number was not unfair. K5 is measured cold (the .becache replay removed
before each of 3 runs). Before this change the bebop column had 1 ms resolution and K3h
read "1.0 ms vs 0.124 = 8.1x" — a ceiling, not a measurement.

Run-to-run: an earlier run the same hour at bebop.bin 70cddb59 gave K1H 1.64 / K2H 1.22 /
K3H 0.62 / K4 4.36 ms with Rust 0.879 / 0.356 / 0.170 / 2.629 — both columns move ~20%
together (DVFS on the pinned A78), the ratios stay 1.8-1.9x / 3.3-3.4x / 3.4-3.6x / 1.6-1.7x.
The ratio is the claim; TG-DONE 1 target is <= 2.0x on every row after T101-T104: K1H and K4
are inside, K2H (calls: frame 16 KiB + spills, P4) and K3H (nested loop, no condition fusion
on the inner compare, P3) are the two rows the codegen steps must move.

## B1 row (2026-09-06, session 17): prologue/epilogue/call-site right-sizing, bebop.bin 1a3b2cc2

Status: measured after B1 landed (R=11, pinned core 4, box idle, fuzzd paused). Ratio is the claim: K2H 3.8x -> 2.6x, K3H 4.0x -> 2.4x (D14 item 1 predicted ~2.7x for K2H); bin_words 74804 -> 74222 (-0.8 %), k2h_loopwords 65 -> 51. Both columns sit ~20 % above the session-15 run (DVFS), K1H/K4 unchanged in ratio.

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 2.01 / 2.20 | 1.147 / 1.232 | 1.8x | MET | 1.8x | 16.2 |
| K2H | 1.13 / 1.19 | 0.443 / 0.491 | 2.6x | UNMET | 2.6x | 16.2 |
| K3H | 0.70 / 0.79 | 0.293 / 0.359 | 2.4x | UNMET | 2.4x | 16.2 |
| K4 | 4.70 / 4.77 | 3.284 / 3.383 | 1.4x | MET | 1.4x | 16.2 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.83 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

## B2 row (2026-09-06, session 17): if-expression join convention, bebop.bin a903d33b

Status: measured after B2 landed (R=11, pinned core 4, box idle). K2H 2.6x -> 2.0x (gate MET), bebop K3H 0.70 -> 0.60 ms while the Rust K3H column moved 0.293 -> 0.189 ms between the two runs (DVFS; its ratio 2.4x -> 3.2x is column noise, the bebop side improved). bin_words 74222 -> 67775 (-8.7 %: every if-expression loses the two arm pushes).

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 2.08 / 2.20 | 1.184 / 1.246 | 1.8x | MET | 1.8x | 13.6 |
| K2H | 0.88 / 0.94 | 0.449 / 0.501 | 2.0x | MET | 2.0x | 13.6 |
| K3H | 0.60 / 0.79 | 0.189 / 0.288 | 3.2x | UNMET | 3.2x | 13.6 |
| K4 | 4.57 / 4.76 | 3.247 / 3.374 | 1.4x | MET | 1.4x | 13.6 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.56 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

## K8 row (2026-09-06): branchy honest kernel, the B9 falsifier for T52-T54, bebop.bin a903d33b

Status: measured R=11, pinned core 4 (ps -e wc -l was 24-25 throughout, no chain/battery from another
agent running). K8H's branch is a genuine ~50% coin flip (bit = (x >> 60) & 1 of an LCG stream,
x = x*6364136223846793005 + 1442695040888963407 wrapping); the two arms do different real work
(acc+x vs acc-i), 20000 inner iterations x REPS=100 = 2,000,000 branches per run. bebop compiles the
`if` to a real conditional branch (b.ne, confirmed by objdump of bench/vs_rust/kernels/k8h.bp's
40-word inner loop -- b.cond count 2: the loop's own down-counter test (b.le, cheap/predictable) plus
the data-dependent arm-select (b.ne, ~50% mispredict)). rustc -O picked `csel` for the same arm-select
(confirmed by objdump of rust_once/k8h.rs: `csel x17, x15, x14, eq` at the sole branch point in its
inner loop, no data-dependent b.cond at all). Two runs the same session: 5.7x (a standalone R=11
script) and 4.5x (the honest.sh run below) -- both far above the naive 2-3x expectation from A78's
1-cycle csel / ~10-cycle mispredict model (B9's research estimate), because bebop's branch also pays
extra mov/spill overhead per arm beyond the raw mispredict cost (see the 40-word disassembly: a stack
spill/reload of the LCG multiply result, register-shuffle movs before/after the branch). acc parity
verified bit-exact against the Rust twin for seed=1: -7706214503032352720 on both sides.

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 2.03 / 2.18 | 1.118 / 1.230 | 1.8x | MET | 1.8x | 15.5 |
| K2H | 0.87 / 1.04 | 0.390 / 0.494 | 2.2x | UNMET | 2.2x | 15.5 |
| K3H | 0.68 / 0.81 | 0.257 / 0.342 | 2.6x | UNMET | 2.6x | 15.5 |
| K4 | 4.60 / 4.76 | 3.254 / 3.300 | 1.4x | MET | 1.4x | 15.5 |
| K8H | 0.31 / 0.43 | 0.069 / 0.073 | 4.5x | UNMET | 4.5x | 15.5 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.59 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

VERDICT: K8H falsifies the "no target" half of B9's hedge -- a genuinely data-dependent branch at
~50% mispredict costs bebop 4.5-5.7x versus LLVM's csel choice, well outside TG-DONE 1's <= 2.0x gate
and well above the 2-3x A78 csel/mispredict model. T52 (pure `if` -> csel) has a real, large target on
this shape; T53/T54 (sink-predicated stores, masked loops) remain undemonstrated by this row and can
still be dropped per B9's original plan -- this row is only evidence for the simplest case (`if` as a
2-way scalar select), not for predicated stores or masked loops.

Control by the main session (same bebop.bin a903d33b, 5 interleaved pinned runs, B5 chain in flight on the box): the
same kernel with a predictable bit `let bit = (i >> 4) & 1;` runs 0.15 ms/rep against K8's 0.34 ms/rep, so ~55 % of K8's
time is the mispredicted branch itself and the remaining ~2.2x over Rust (0.069 ms) is the 40-word stack-machine loop.
Decision: T52 proceeds -- as a csel on tags in the IR rung (R3+, both arms REG/SYM/CONST), not as a word peephole;
T53/T54 stay deleted.

## Register-model row (2026-09-06, session 18): stack machine retired (docs/REGISTER-MODEL-BLUEPRINT.md), bebop.bin 53d13800

Status: measured after the register-model landing (R=11, pinned core 4, box idle, fuzzd paused). Every value/expr lives in a register or an x15 frame slot; push_words (str/ldr x0,[sp] + ldr x1,[sp] + sub/add sp,sp,#16 in the code region after stub_words) is now the enforced invariant (`bench/vs_rust/invariants.sh`, `tools/perf.py` EXACT `push_words`) and reads 0. bin_words 68229 -> 36218 (-47 %, well under the "< 55000" report target). Loop words: K1H 10 -> 5 (gate <= 8, MET), K3H 24 -> 8 (gate <= 10, MET), K4 14 -> 7 (gate <= 13, MET), K8H 39 -> 25. K2H's automated `k2h_loopwords` reads 51, unchanged from every prior baseline (B1/B2/B5 rows above all show the same 51): this is a pre-existing quirk of `loop_words()` for a non-looping kernel -- it picks up the entry_stub's SIGTRAP-handler jump-back (`b main`-equivalent, the single smallest backward branch in the whole binary) rather than a real hot loop, since `fib` recurses via `bl` and has no loop of its own. The real per-fn footprint is `bench/perf_fn/latest.txt`'s `fib` entry: fn body shrank from the old stack-machine form to 24 words (prologue-to-ret span at 0x0-0x5c in the k2ht plain disassembly), close to the blueprint's "~21" estimate.

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 0.97 / 1.12 | 0.979 / 1.075 | 1.0x | MET | 1.0x | 15.8 |
| K2H | 0.69 / 0.90 | 0.347 / 0.482 | 2.0x | MET | 2.0x | 15.8 |
| K3H | 0.28 / 0.37 | 0.226 / 0.321 | 1.2x | MET | 1.2x | 15.8 |
| K4 | 3.63 / 3.75 | 2.759 / 2.889 | 1.3x | MET | 1.3x | 15.8 |
| K8H | 0.25 / 0.31 | 0.070 / 0.074 | 3.6x | UNMET | 3.6x | 15.8 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.27 s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

K4's ms gate: `k4_ms 3.63 <= 3.0`? UNMET on the absolute figure, but the blueprint's ratio form (`k4_ms <= 1.15 x` the Rust honest twin in the same run) is the one that governs (D12-B's 3.0 ms absolute predates the honest twins): 3.63 / 2.759 = 1.32x, still above 1.15x -- UNMET on the ratio form too, recorded honestly rather than declared MET.

REGRESSION FOUND, not closed by this row: `bench/vs_rust/parity_driver.sh`'s K7NEON (`bench/vs_rust/kernels/k7neon.bp`, `hvham2` builtin) SIGSEGVs (trap 82) with this bebop.bin; it passes with the previously-committed bebop.bin (e654370993d2, HEAD). Root cause and minimal repro: see docs/exp.journal and the VERDICT of this session's wrap-up task -- `emit_hvham2`'s new `vs_park`/`vs_evict`/`vs_mat` non-sequential register materialisation (a->x0, ao->x6, b->x1, bo->x5, n->x3) loses track of a computed (non-symbol, non-constant) `ao` argument during the evict cascade and a later `vs_mat` overwrites it before it is read. Not patched here (real miscompile, not an exit-89 case) -- left for the main session.

## A2 commit 1 row (2026-09-06/07): csel for pure if-arms (docs/blueprints/A2-csel-and-const-hoist.md §3), bebop.bin 4c53832d (kernel rows measured on e1df4dcc, whose emitted kernel words are identical; 4c53832d only passes the cached source length into the csel scanners)

Status: measured after A2 commit 1 landed and promoted (R=11, pinned core 4, box otherwise idle, fuzzd paused). `emit_cond` now pre-scans both arms of every `if c then a else b` (`arm_is_pure`, text-only, no position mutation, both passes agree); when neither arm can call, `let`, `while`, nest an `if`/`match`, hold an array/string/struct/enum literal, or clobber flags (`<`,`>`,`==`,`!=`,`!`), it emits `csel xd,xn,xm,<cond>` instead of `b.<inv>`/`b`/join. c05_if and c46_andor (impure arms: comparisons, `&&`/`||`) stay byte-identical (0 WORD_DELTA). Census: bcond 1762 -> 1071 (-39%), cbz 124 -> 126 (+2, the 5 new purity-scanner fns' own register-condition branches; census_allow.txt records both this and an intermediate mis-freeze -- see the journal line for the real root-cause story). `k8h_loopwords` 25 -> 22 (docs/PERF.md), a real but modest drop -- K8H's own loop body only has the one `if` (the ~55% branch this kernel exists to falsify) and its arms are pure, so csel does fire there, but the LCG constant materialisation (2 x movz+3movk = 8 words/iter) this row does NOT touch dominates the loop body's word count.

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 1.15 / 1.30 | 1.141 / 1.212 | 1.0x | MET | 1.0x | 13.0 |
| K2H | 0.83 / 0.89 | 0.437 / 0.501 | 1.9x | MET | 1.9x | 13.0 |
| K3H | 0.35 / 0.41 | 0.265 / 0.316 | 1.3x | MET | 1.3x | 13.0 |
| K4 | 4.35 / 4.45 | 3.292 / 3.348 | 1.3x | MET | 1.3x | 13.0 |
| K8H | 0.16 / 0.17 | 0.072 / 0.073 | 2.2x | UNMET | 2.2x | 13.0 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.77 s (was 25.35 s on e1df4dcc: arm_end/ident_call_at called str_len per if/identifier) | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

K8H's blueprint gate is <= 1.2x (docs/blueprints/A2-csel-and-const-hoist.md §0/§7), tighter than this table's generic <= 2.0x column: 2.2x is UNMET against 1.2x, down from the blueprint's stated 4.5x baseline (and from a 3.6x row two sections up) but csel alone does not clear it -- the blueprint's own contingency is commit 2 (hoist the two 64-bit LCG constants out of the loop body), not attempted this round per instructions. K5 self-compile reads 25.35s here vs 1.27s in the register-model row above; not re-measured against an A2-free bebop.bin in this session, so whether that gap is a real regression, a `bench/vs_rust/kernels/k1h.bp` measurement-script change, or box load between sessions is not established here -- flagged, not investigated, out of scope for this row's K8H gate.

## A2 commit 2 row (2026-09-07): constant hoisting (docs/blueprints/A2-csel-and-const-hoist.md §3 "Commit 2"), bebop.bin 9694b780

Status: K8H's blueprint gate stayed UNMET (2.2x/1.9x) after csel alone, so commit 2 landed per the blueprint's own
contingency. `emit_while_stmt` now pre-scans the loop body's text once at loop entry (before the forward `b -> test`
placeholder, so this runs exactly once, never inside the repeated body) for integer literals >= 65536 at brace depth
0 -- K8H's two 64-bit LCG constants (6364136223846793005, 1442695040888963407) -- materialises up to 4 of them with
the existing `mat_const` (movz+3movk) into free cs registers strictly above every register the body's own `let`s
will bind (a text-scan count of NEW names only, via `sym_lookup` against the same `stab` `sym_bind` itself uses --
K8H's body rebinds x/acc/i and only genuinely introduces `bit`, so counting blindly starved the hoist to 1 of 2
constants before this fix), and `vs_push` swaps a matching `CONST` push for a direct register (`SYM`) reference for
the rest of that loop's own compile -- zero new instruction words, zero new window/materialisation kind (kind 3 SYM
already has exactly the right "permanent register, never freed on pop" contract; reusing kind 4 CS would have freed
the register on the constant's first use, corrupting every later iteration). A loop whose body nests another `while`
declines to hoist entirely (`bench630/k1ht.bp`/`k4t.bp`'s `while rep>0 { let i=1000000; while i>0 {...} }` and
`std_tests/ptrless.bp`/`deltasync.bp`'s nested FNV loop all hit this: an outer-hoisted register stays live in the cs
mask across the whole outer body, including the inner loop's own entry, which asserts the cs mask is 0 there) --
K8H/c72_hoist have no nested loop, so this costs nothing for the kernel this commit targets. `k8h_loopwords` 22 -> 14
(docs/PERF.md), exactly the predicted -8 (two 64-bit constants x 4 words each, moved out of the repeated body).
Census: bcond 1113 -> 1116 (+3, the nested-`while` guard's own selector), cbz unchanged at 126; bin_words 38842 ->
38975 (+133, the 6 new fns compiled into the compiler itself: hoist_lookup/hoist_mask/find_body_end/hoist_record/
hoist_scan/hoist_release -- a one-time compiler-size cost, not per-target-program growth); prologue/epilogue grow by
2 words in any hoisting fn (one more callee-saved pair, x23/x24) -- expected, `cs_hi` (fntab[4575]) already tracked
this via the existing `vs_cs_take`. New construct c72_hoist (K8H's own loop shape, EXPECT 5504683299252448320 via
bpref.py).

| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |
|---|---|---|---|---|---|---|
| K1H | 1.22 / 1.27 | 1.212 / 1.275 | 1.0x | MET | 1.0x | 15.6 |
| K2H | 0.83 / 0.91 | 0.451 / 0.526 | 1.8x | MET | 1.8x | 15.6 |
| K3H | 0.31 / 0.40 | 0.229 / 0.345 | 1.4x | MET | 1.4x | 15.6 |
| K4 | 4.37 / 4.47 | 3.294 / 3.400 | 1.3x | MET | 1.3x | 15.6 |
| K8H | 0.14 / 0.14 | 0.072 / 0.073 | 1.9x | MET | 1.9x | 15.6 |
| K5 self-compile of bebop.bp (cold, pinned, median of 3) | 1.92 s (1.914/1.944/1.921, deleting $OUT/k5.bin* between runs) | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |
| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | 18.4 ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |

K8H's blueprint gate is <= 1.2x (docs/blueprints/A2-csel-and-const-hoist.md §0/§7): 1.9x is a real, large drop from
csel-only's 2.2x (loop words halved, wall time roughly halved too, 0.16 -> 0.14 ms/rep against a similarly-noisy
0.072/0.073 ms Rust baseline) but still UNMET against the tight 1.2x target. The remaining gap is the branch itself
(K8H's whole reason to exist: a genuinely ~50% data-dependent bit, `(x >> 60) & 1`, that csel already turned into a
csel rather than a mispredicted branch -- so what is left is real ALU/memory work per iteration, not something
constant hoisting or csel can remove) and possibly REPS-loop/clock_ms overhead LLVM's Rust twin does not pay the
same way; per the task's "one variable" scope (constant hoisting at loop entry only, no other codegen change) this
is not chased further this round -- reported honestly rather than widened.

## Hot vs cold row (2026-10-07, main session 4384f304), bebop.bin 8a0b4325 (self-compile reproduces it byte-for-byte)

Box NOT quiet: three coding lanes + a research agent alive, procs 33/26 at one point (one rc=137 retry); the slot serialised compute.

HOT — in-process ms per rep, pinned core 4, R=11 (honest.sh, two runs; the ratio moved ≤ 0.1x between them):

| kernel | bebop | Rust -O honest twin | bebop / Rust |
|---|---|---|---|
| K1H | 0.920 / 1.030 | 0.980 / 1.074 | 0.9-1.0x |
| K2H (calls, fib) | 0.576 / 0.642 | 0.299 / 0.316 | 1.9-2.0x |
| K3H (nested loop) | 0.161 / 0.160 | 0.118 / 0.119 | 1.3-1.4x |
| K4 | 3.53 / 3.86 | 2.63 / 2.93 | 1.3x |
| K8H | 0.024 / 0.028 | 0.022 / 0.026 | 1.1x |

COLD — source to first result, new process each time, every sidecar removed, pinned core 4, median of 5 (scratchpad c2s.py); outputs equal in both languages:

| kernel | bebop compile | bebop run | bebop total | rustc -O | rust run | rust total | bebop / Rust |
|---|---|---|---|---|---|---|---|
| K1 | 42 ms | 21 ms | 63 ms | 750 ms | 23 ms | 773 ms | 0.08x |
| K2 | 47 ms | 25 ms | 73 ms | 804 ms | 17 ms | 821 ms | 0.09x |
| K3 | 31 ms | 18 ms | 49 ms | 998 ms | 16 ms | 1015 ms | 0.05x |
| K4 | 61 ms | 43 ms | 103 ms | 773 ms | 35 ms | 808 ms | 0.13x |

Process wall (bench_pinned.sh, run only): K0 empty program bebop 9.6 ms vs Rust 25 ms pinned / 10.9 ms unpinned. That row is noise-dominated today: Rust's p95 was 251 ms.
K5 self-compile of bebop.bp: cold 0.84 s (honest.sh) and 0.84-0.97 s (direct), warm via the .dag/.becache memo 0.04 s.

Two instruments were fixed today:
1. honest.sh K5 left the new `.dag` sidecar between runs, so the "cold" row read 0.09 s (a replay); it now removes every `k5.bin*` and refuses a failed compile.
2. bench_pinned.sh compared the one-rep kernels/k*h.bp against the N-rep rust_once/k*h.rs in the process-wall table: MISMATCH and a fake 0.08-0.12x. The h rows are out of that table.

## R2 TRE row (2026-10-07, lane rfast-lsr), candidate 0cc4c5c5 vs control 8a0b4325 (fixpoint gen3 == gen4 0cc4c5c5; NOT promoted)

What changed: tail-recursion elimination for SELF calls in tail position (compiler/tre.bp, emit_bl_call `tk`):
`f(args)` in tail -> args to x0.., `b` back to the param-move block; `f(a) + f(b)` in tail -> the left value
joins an accumulator register (one hidden stab entry), the right call becomes the branch, every return adds the
accumulator. LLVM's TailRecursionElimination shape. R1 (LSR/LICM) was NOT built: STEP 0 census 11.2 % of 1707
loops (A3 rule needs >= 20 %). Coverage of R2: 3 corpus fns (census) / 11 of 439 sweep programs change bytes,
all 11 print the same output and exit code as the control.

Box BUSY (procs 30 before the candidate run, 26 before the control; DVFS cap 1.9 GHz, cpu4 dipped to 0.69-0.96 GHz
between kernels) -- candidate then control back to back in ONE slot, honest.sh R=11, pinned core 4:

| kernel | candidate 0cc4c5c5 bebop / Rust | ratio | control 8a0b4325 bebop / Rust | ratio |
|---|---|---|---|---|
| K1H | 1.120 / 1.083 | 1.0x | 1.080 / 1.081 | 1.0x |
| K2H (fib, TRE on both sides) | 0.464 / 0.327 | 1.4x | 0.648 / 0.340 | 1.9x |
| K2H2 (fib, no TRE either side, new twin) | 0.632 / 0.369 | 1.7x | 0.630 / 0.368 | 1.7x |
| K3H | 0.193 / 0.143 | 1.3x | 0.193 / 0.145 | 1.3x |
| K4 | 4.280 / 3.225 | 1.3x | 4.280 / 3.180 | 1.3x |
| K8H | 0.028 / 0.026 | 1.1x | 0.029 / 0.027 | 1.1x |
| K5 self-compile (cold, median of 3) | 1.10 s | | 1.06 s | |

A quiet-box run of the same kernel code (compiler d6acf93a emits byte-identical kernels; procs 22, 1.9 GHz flat):
K2H 0.464 / 0.323 = 1.4x, control 0.648 / 0.322 = 2.0x. K2H bebop -28 %; the K2H <= 1.1x gate is UNMET.

Why K2H stops at 1.4x (objdump): half of fib's nodes are leaves, and a bebop leaf is 17 instructions against
LLVM's 9 -- `sub sp,#0x60` / `add sp` (frame) and a second saved pair x21/x22 that exists only because every
call inside an if-arm parks the arm's join register (`mov x21, x0`, pre-existing, every program). An internal node
is 11 instructions (mov x19,x0 / cmp / b.ge / sub / mov x21,x0 / mov x0,x1 / bl / sub / add acc / mov / b) against
LLVM's 8. Closing the rest is shrink-wrapping (base case
before the prologue), not TRE. K2H2 (no TRE anywhere) is 1.7x on both compilers: the cost of a real bebop call,
not the 5.7-cycle "LLVM level" the research report inferred from K2H.
Compiler cost: census words 65307 -> 67175 (+1868, +2.9 %), bcond 1962 -> 2040 (+78), cbz 193 -> 193; K5 cold
self-compile 1.060 -> 1.110 s median / 1.059 -> 1.066 s min (interleaved x3).

## R2b shrink-wrap guard row (2026-10-07, lane rfast-lsr), candidate bc2aa59b on top of R2 0cc4c5c5 (NOT promoted)

A fn whose whole body is `if P OP NUM then (P|NUM) else ...` (P a parameter in x0..x7) now starts with
`cmp xP,#NUM; b.<not OP> full; [mov x0,..]; ret` BEFORE the prologue (compiler/tre.bp sw_guard). fib's leaf:
17 instructions -> 3 (`cmp x0,#2; b.ge; ret`; LLVM 9). Coverage: 23 of 2338 distinct fns have that shape (1.0 %;
7 of them also call); 31.7 % have SOME call-free return path (28.6 % no calls at all, 3.0 % mixed). The
if-join park (`mov x21,x0` + the x21/x22 pair) was NOT changed: the guard removes it from every leaf activation.

honest.sh was SIGKILLed by procguard 3 times (procs 30-33) -> NOT MEASURED. SUBSTITUTE, same slot, core 4,
interleaved x7, k2ht total ms / 500 reps (median): control 8a0b4325 0.564, R2 0cc4c5c5 0.406, R2b bc2aa59b 0.334;
Rust k2h.rs 0.284 -> K2H 1.99x / 1.43x / **1.18x**. Spread: R2b 149-193 ms of 500 reps (wider than R2's 199-208).

## R-CF row (2026-10-08, lane rfast-lsr), candidate e1fa8a6f on top of R2b bc2aa59b (NOT promoted)

R-CF eliminates affine `while` loops in closed form (compiler/affine.bp + affine_gen.bp, a text pass in
use_expand_at): a body of `let x = <affine form>;` statements over Z/2^64 with a counted exit becomes v_N = M^N v_0.
Literal start -> P = M^N folded at compile time (nested loops compose: K1H/K3H/K4 become straight-line code);
symbolic start -> a generated helper applies M^N by binary powering when N >= 1024 and exact, else the loop runs.
`BEBOP_NO_CF=1` turns it off. The "loop eliminated" rows are NOT a codegen comparison: Rust keeps its loop. The codegen
claim is carried by the "kept loop" rows, which must equal the R2b numbers.

honest.sh R=11, core 4, procs 26-28 (box busy: p95s and K5 are inflated; the ratios are the claim):

| kernel | bebop med / p95 ms per rep | Rust med / p95 | bebop / Rust |
|---|---|---|---|
| K1H loop eliminated | 0.0000 / 0.0000 (whole 100-rep kernel < 1 ms clock_ms tick) | 1.067 / 1.097 | 0.0000x |
| K3H loop eliminated | 0.0000 / 0.0000 | 0.145 / 0.238 | 0.0000x |
| K4 loop eliminated | 0.0000 / 0.0000 | 3.776 / 10.791 | 0.0000x |
| K1H kept loop | 1.130 / 2.170 | 1.127 / 1.812 | 1.0x (R2b control 1.0x) |
| K3H kept loop | 0.292 / 0.445 | 0.215 / 0.241 | 1.4x (R2b control 1.3x; Rust row inflated too, box busy) |
| K4 kept loop | 4.370 / 6.710 | 3.271 / 5.485 | 1.3x (R2b control 1.3x) |
| K2H / K2H2 / K8H (untouched) | 0.384 / 0.642 / 0.0383 | 0.325 / 0.559 / 0.031 | 1.18x / 1.1x / 1.24x |

Bit-exactness against the R-SPIKE C values: K1H 1M x 20 reps -12017413204599424 (= 18434726660504952192),
K4 10 reps -1871081701536519807 (= 16575662372173031809), K3H 200 reps 2875824312501384256 -- folded, kept and
bc2aa59b binaries all print them. Crossover (symbolic start, 40M iterations, cf / kept ms): N=256 43/44,
512 44/43, 1023 43/43, 1024 7/44, 2048 2/43, 4096 1/43 -- the 1024 guard holds and is conservative.
Compiler cost: census words 67771 -> 74817 (+7046), bcond 2058 -> 2345 (+287), cbz 193 -> 224; K5 interleaved x7
1.400 -> 1.418 s median (+1.3 %), 1.379 -> 1.410 s min.
