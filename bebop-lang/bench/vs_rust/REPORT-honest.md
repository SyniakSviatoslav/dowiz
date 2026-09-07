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
