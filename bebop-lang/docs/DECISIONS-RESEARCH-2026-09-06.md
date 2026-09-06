# Bebop — decisions still open, and where a real breakthrough is

Status: 2026-09-06 REPORT (Opus analyst, session 16, read-only at HEAD 94cf3fc; the §5 questions await the operator; nothing here is decided until a D-entry in HISTORY.md says so)

Status: 2026-09-06, read-only research at HEAD 01185e7 (`bebop.bin` 74804 words, PERF row `f86bee7/e14dd55e`).
Method: every claim below is either a file:line in this tree, a number in a committed script, or a
disassembly I produced today by compiling the committed kernels with the committed `bebop.bin` into a
scratch directory (`seed/build/seed ./bebop.bin compile bench/vs_rust/bench630/k{2h,3h,4}t.bp`) and
running `objdump -D -b binary -m aarch64` — the same method `bench_pinned.sh:59-65` uses for its
loop-word column. Nothing in the repo was written, no chain/battery/fuzz was run.
External comparisons are cited inline with URLs.

---

## 0. The three measurements that reorder everything below

I disassembled the three honest kernels. The result contradicts the attribution in
`bench/vs_rust/REPORT-honest.md` (its "Method" paragraph, last 4 lines) and it changes which rung is
worth doing first.

**K4 inner loop — 15 words, zero memory traffic** (`k4t.bin` 0x8c..0xc4):

```
cmp x22,#0 ; b.le exit ; mov x0,x22 ; lsl x1,x0,#3 ; sub x0,x1,x0 ; mov x1,x0 ;
mov x0,x20 ; add x0,x0,x1 ; add x0,x0,x0,lsl #1 ; sub x0,x0,#0xb ; mov x20,x0 ;
mov x0,x22 ; sub x0,x0,#1 ; mov x22,x0 ; b loop
```

Six of the fifteen are `mov` copies to and from x0. The recurrence itself is three ALU ops. K4 is
already 1.15-1.4x of its honest twin and is **latency-bound**, so word removal here buys words, not ms.

**K3H inner loop — 25 words, of which 8 are stack round trips** (`k3ht.bin` 0x9c..0xfc):

```
... add x0,x0,x0,lsl #1 ;  sub sp,sp,#16 ; str x0,[sp] ;  ... ; ldr x0,[sp] ; add sp,sp,#16 ; add x0,x0,x1 ;
                            sub sp,sp,#16 ; str x0,[sp] ;  ... ; ldr x0,[sp] ; add sp,sp,#16 ; add x0,x0,x1 ; ...
```

`a*3 + x*2 + y*3` is left-nested, so the left operand of each `+` is a multi-word expression;
`left_single_begin` (bebop.bp:1813-1826) only slides a **one-word** left producer, and `pop2`
(bebop.bp:1780-1812) only matches `push a; P; push b`. Both bail, and the partial sum goes through
memory twice per iteration — the exact serial-`sp` + store-to-load chain that
`docs/SPEEDUP-ANALYSIS.md` §1.3 measured at 7 cycles. **K3H (4.0x, the worst-but-one row) is a P2
problem and nothing else.**

**K2H (fib) — 41 words executed per call on the else path, 13 of them pure overhead**
(`k2ht.bin` 0x00..0xb0):

| waste | words/call | evidence |
|---|---|---|
| `nop` from the OPT-A post-hoc patch of unused callee-saved pairs | 8 (4 in the prologue at 0x10-0x1c, 4 in the epilogue at 0x94-0xa0) | bebop.bp:3767-3786 patches to NOP *after* emission, so the words stay and execute |
| `add x15,sp,#0x100` although fib never spills (1 symbol) | 1 (0x20) | the OPT-G1 usage scan (bebop.bp:3792-3812) sees register 15 in `emit_bl`'s own `stp x15,x14` word (bebop.bp:538) and therefore never NOPs the setup — a self-fulfilling scan |
| `stp x15,x14` / `ldp x15,x14` around every `bl` | 4 (2 per call site x 2 sites) | `emit_bl` emits them unconditionally; its `has_spills` parameter (bebop.bp:531) is **never read in the body** |
| push/pop of the `if` result at the join | 6 (0x38/0x3c, 0x84/0x88, 0x8c/0x90) | `emit_cond` (bebop.bp:2451) leaves each arm's value on the stack; the caller pops |
| `mov x0,x19; sub x0,x0,#1` instead of `sub x0,x19,#1` | 2 | the x0-centric model |

Measured: K2H 1.09 ms/rep for 242,785 calls x 100 reps = **4.5 ns = ~10.8 cycles per call for 41 words**,
i.e. ~3.8 IPC on a 4-wide core. K2H is **throughput-bound**, so removing words removes time almost
linearly. `REPORT-honest.md` blames "calls: frame 16 KiB + spills, P4" — fib has **zero** spills and the
16 KiB frame is one `sub sp` per call. The doc's attribution is wrong; the cost is NOPs, redundant
saves and copy traffic.

Consequence: **the worst two rows have different causes and only one of them needs the IR.**
K2H (3.8x) is fixed by four S-effort, single-variable emitter edits that do not touch the expression
model at all. K3H (4.0x) needs the register tier. K1H (1.5x) and K4 (1.4x) are already inside the
TG-DONE 1 gate.

---

## 1. Decisions still open

Effort: S = hours, M = a session, L = multi-session.

| # | task | decision the operator still has to make | recommended answer | evidence | effort |
|---|---|---|---|---|---|
| 1 | D12-B / T96 P2 | Does the IR rung come first, or the four P4 micro-items (prologue sizing, join convention, conditional x15/x14 save, loop rotation)? | **P4 micro-items first**, then the IR rung. They move the *worst* honest row, each is a single-variable diff with an existing probe, and they shrink the emitter the IR rung has to replace | §0 K2H table; c53_param9 / c23_spillcall / c24_ifspill already exist | S each |
| 2 | T96 P2 shape | Per-fn **op-list IR** (D12-B's wording) or a Liftoff-style **operand-tag stack** (compile-time value stack of {reg, const, spill} tags, no op list)? | **Tag stack.** It is the same information with none of the "refactor every emit_*" cost T101 refused, and it *deletes* pop2 / left_single_* / writes_producer / count_masked | §3; T101's own note (HISTORY.md:2670); Liftoff `VarState::Location = {kStack,kRegister,kIntConst}` https://chromium.googlesource.com/v8/v8/+/master/src/wasm/baseline/liftoff-assembler.h | M |
| 3 | window size | How many scratch registers does the operand window need — x1-x7, or fewer? | **Declare x1-x7; the measured requirement is 4.** Over 111 files (bebop.bp + all of selfhost/std + the kernels) and 11,854 binary-operator trees the Sethi-Ullman number is 2 for 95%, 3 for 4.8%, **4 for 14 trees, and never 5**. A 7-register window provably never spills on any committed program | my analysis over `tools/bpref.py`'s AST (§3.2); SU recurrence: Sethi & Ullman JACM 17(4):715-728 https://en.wikipedia.org/wiki/Sethi%E2%80%93Ullman_algorithm | — |
| 4 | P4 frame size | Keep the flat 16 KiB frame (`sub sp,sp,#0x4,lsl #12`, bebop.bp:2857) or compute it per fn? | **Compute it**: `80 + 8*while_marks + 8*spill_slots`, plus the 15.6 KiB heap only when the body contains `mov x0,x14`. It also closes the fuzzer's TRAP-82 "deep recursion" class, which D12-C requires to be 0 | prologue decode (0xd14013ff = `sub sp,sp,#16384`); TRAPS.md row 82; PERF `fuzz_trap_unpredicted` 7 -> 15 | M |
| 5 | calling convention | Args are evaluated onto the stack then popped into x0..x7 (`emit_bl_call`, bebop.bp:694-698). Change now, or after the tag stack? | **After.** With the tag stack the arg list is just "materialise tag i into x{i}", which is where the win is; doing it against the push/pop model duplicates work | bebop.bp:666-706 | — |
| 6 | x15/x14 call save | Unconditional (today) or only when the caller uses them? | **Conditional**, and stop feeding the OPT-G1 scan with the save words themselves | bebop.bp:538 vs 3792-3812; `has_spills` is dead | S |
| 7 | TG-DONE 1 target | Does D1(a) "<= 1.0x Rust" stay a long target, or is it retired? | **Keep it reported, state the prior in ROADMAP**: baseline (one-pass) tiers land at **1.1x-1.5x** of an optimising tier on Sightglass; that is the honest ceiling for this compiler shape, and the <= 2.0x gate is the real target | https://github.com/bytecodealliance/rfcs/blob/main/accepted/wasmtime-baseline-compilation.md ; D12-G | — |
| 8 | T52-T54 | Write K8 (branchy honest kernel) and then decide, or decide now? | **Write K8, expect "no", then delete T53/T54.** A78: `csel` is 1-cycle, mispredict ~10 cycles (13-stage pipe); a well-predicted branch measured **2.9x faster** than cmov, and cmov only wins near coin-flip mispredict rates | https://en.wikichip.org/wiki/arm_holdings/microarchitectures/cortex-a78 ; https://questdb.com/blog/cmov-vs-branch-perf/ ; D12-H | S (K8) |
| 9 | store threshold b | ROADMAP's thesis says the store is gated "against sqlite, **LMDB** and native Rust"; D11-I defined b as ">= b x LMDB on point lookups"; D12-F redefined b as the sqlite window scan. LMDB and native Rust have never been measured | **Delete LMDB and "native Rust" from the thesis sentence**, or measure LMDB once through ctypes (the sqlite oracle already does exactly this). Do not leave a thesis clause with no script | ROADMAP thesis §2; HISTORY.md:1544-1555 (D11-I); D12-F | S |
| 10 | the real workload W | Still open (ROADMAP "Open decisions") | **dowiz-core order log** — T66 already committed `money.bp` + `ordfsm.bp` with byte-exact Rust oracles; it is the only candidate workload in this tree with a runnable Rust twin | TASKS T66; HISTORY.md T66 body | M |
| 11 | durable commit 0.15x | Chase it, or mark it unprovable here? | **Mark unprovable on this box and stop.** `msync` of one page costs 101.5 us under proot and f2fs runs `fsync_mode=nobarrier`; the row measures proot, not the design | LANG-DB-DESIGN appendix (`msync 1 page 101.5 us`, `rename 267 us`); ANALYSIS §4D | — |
| 12 | store size loss c=2.5x | Optimise (packed cells, D11-H) or accept? | **Accept and say why**: fixed-width in-memory layout *is* "persisted objects are the in-memory objects". Only the 16-byte per-object header (`h0` len+digest, `h1` crc+generation, store.bp:3-5) is negotiable; folding it to 8 bytes takes 85.2 MB -> ~77 MB (~2.2x). Packing i64 fields to i32 would break the thesis | selfhost/prelude/store.bp:1-15; ROADMAP Measured (85.2/72.4 vs 34.1 MB) | — |
| 13 | T48 rest | D12-H says move the type check into bebop.bp (M, zero speed); the session-15 analyst said no | **Do it as a rider on the IR rung, not before.** The tag stack forces a per-symbol table in the planning pass anyway; attaching a type to that table drops the cost from M to S. Before the IR rung it is M for nothing | bebop.bp:181-203 (`sym_bind` has no type field); invariants.sh:44-48 | S after §3 |
| 14 | TG-DONE 8 / T39 | The seed counter resets on every codegen promotion (D12-C). 10^5 on one binary at 1.58 prog/s = **17.6 h of uninterrupted fuzzing**. Is that compatible with continuing codegen work? | **No — schedule it.** Land the IR rung, freeze codegen for one 24 h window, then run to 10^5. Otherwise the counter never converges | PERF `fuzz_seeds_on_bin` 3000, `fuzz_rate` 1.58; D12-C | — (wall clock) |
| 15 | TRAP-82 reporting | `tools/perf.py` still reports the *combined* `fuzz_trap_unpredicted` (7, then 15), but D12-C split 81 from 82 | **Split the perf row into `fuzz_trap_81` / `fuzz_trap_82`** so the "0 tolerated" class is visible per commit | docs/PERF.md rows | S |
| 16 | `--codegen` ergonomics | `tools/chain.sh` still does not export `FREEZE=1` on `--codegen` (RETRO §5 item 4, unapplied) | **Apply it.** One line, kills the most frequent RED-by-construction | tools/chain.sh:9-10 (`CG=1` set, `FREEZE` never exported); battery.sh reads `${FREEZE:-0}` | S |
| 17 | ledger | TASKS.md still shows `T96 PARTIAL`, `T101-T108 OPEN` (7 of 8 done), `T77 OPEN` (ladder.py exists), `T83 OPEN` (the ratio column exists) | **Edit the HISTORY headers** — the pre-commit hook regenerates from them, so the hook cannot fix stale headers | TASKS.md; ANALYSIS §4A.7 | S |
| 18 | parked work | D11-J says "all tasks stay, no PARKED section". 24 of 37 OPEN tasks are separate projects with zero speed and no gate | **Reverse D11-J**: one `## PARKED (reopened only by decision)` heading in TASKS.md. The 37-OPEN count is the single most misleading number in the repo | ANALYSIS §2 bucket table | S |

---

## 2. Breakthrough candidates, ranked by expected gain / effort

Ranked by (expected number) / (effort x risk). Every row names the gate that proves or kills it.

### B1 — Right-size the prologue, epilogue and the call-site x15/x14 save  ★ best ratio

- **Mechanism.** `compile_fn_at` (bebop.bp:3673) already runs **twice per function** — the planning pass
  for sizes, the emission pass for words (`compile_program_offs`, bebop.bp:3882). Have the planning pass
  publish four facts per fn in `fntab` — {symbols bound, uses x15, uses x14, while-nesting depth} — and
  have the emission pass *emit only what is needed* instead of emitting everything and NOPing it
  afterwards (bebop.bp:3767-3786) and instead of deciding x15/x14 by scanning its own output
  (bebop.bp:3792-3812). Make `emit_bl`'s `stp x15,x14` conditional on the caller's facts (its
  `has_spills` parameter, bebop.bp:531, already exists and is dead).
- **Expected number.** fib: 13 of 41 executed words per call disappear -> **K2H 1.09 ms -> ~0.78 ms
  (3.8x -> ~2.7x)**. Every fn in the tree loses 0-8 prologue/epilogue words: `bin_words` 74804 -> ~72500
  (-3%). No effect on K1H/K3H/K4 loops (they are inside one fn).
- **Gate.** `k2h_ms`, `k2h_loopwords`, `bin_words` (E2/E3 rows already in bench/perf.csv); fixpoint
  gen3==gen4; constructs re-frozen; `c53_param9`, `c23_spillcall`, `c24_ifspill`, `c26_selfrec` unchanged.
- **Effort** S/M. **Risk** low-medium: this is the DIVERGE-42122 neighbourhood (the OPT-G1 scan bug that
  wrote a 9th param into the caller's spill slot), but the fix *removes* the scan rather than patching it.
- **Deletes.** OPT-A post-hoc NOP patching (20 lines), the OPT-G1 word scan and its DIVERGE-42122 special
  case (25 lines), and the dead `has_spills` parameter.

### B2 — Value-in-x0 at `if` / `while` joins

- **Mechanism.** `emit_cond` (bebop.bp:2451-2486) makes both arms end with a `push` and the consumer pop;
  `emit_while_stmt` (bebop.bp:3113-3125) already retracts the special case of a trailing literal `0`
  (T96 s2b). Generalise: an arm's result stays in x0, the join is a barrier, no push/pop.
- **Expected number.** fib: 6 of 41 words (-15%). Every `if`-expression in the tree loses 6 words —
  `c05_if` 37 -> ~31, `c24_ifspill` 169 -> ~150, and the compiler is full of `if`-expressions
  (`bebop.bp` is written in the branchless-flag style: `let x = if c then a else b`).
- **Gate.** `cw:c05_if`, `cw:c24_ifspill`, `cw:c16_compound` word rows; `bin_words`; fixpoint.
- **Effort** S. **Risk** low — the barrier discipline (`fntab[3660]`) already exists for exactly this.
- Combined with B1: **K2H ~0.60 ms, ~2.1x** — one point outside the gate, which B3/B4 then close.

### B3 — The operand-tag stack (the T96 P2 / T101 IR rung)  ★ biggest single number

- **Mechanism.** §3. Replace the emitted `push`/`pop` pair with a compile-time stack of *tags*
  ({register r, constant c, spill slot s}); operators consume tags and emit one 3-address instruction.
- **Expected numbers.** K3H inner loop **25 -> 6-9 words**; K4 **15 -> 7-9**; K1H **11 -> 8**. K3H is
  throughput/memory-bound (8 of its 25 words are the sp-serial store-to-load chain), so
  **K3H 0.52 ms -> ~0.22-0.28 ms (4.0x -> ~1.4-1.8x)**. K4 is latency-bound on
  `add;add-shifted;sub` and will move little in ms (3.75 -> ~3.5) while meeting the `<= 13` word gate
  comfortably. K1H ~1.59 -> ~1.35 ms.
- **Gate.** D12-B's own: K4 loop `<= 13` words **and** `<= 3.0 ms`; add `k3h_loopwords <= 10`; fixpoint;
  all 52 constructs re-frozen under FREEZE=1; census `b.cond` non-increasing.
- **Effort** M (not L — see §3.4 for why the "multi-session refactor of every emit_*" estimate was
  pessimistic: only `push`/`pop`/`pop2` and 4 fold helpers need to change semantics; the other 60
  call sites keep calling `push`/`pop`, which become tag operations).
- **Risk** medium: it is the fixpoint-critical path. Mitigation is D11-G's own rule — the first rung
  must be **byte-identical** (tags that always materialise into x0 reproduce today's stream exactly),
  and only then are the lazy paths switched on one at a time.
- **Deletes.** `pop2` (33 lines), `left_single_begin`/`left_single_finish`/`writes_producer`/
  `count_masked` (78 lines), and turns `madd_try`/`shl_try`/`mulc_try`/`addshift_try`/`cmp_try`
  (185 lines of word-pattern matching with label-barrier guards) into tag matches of a few lines each.
  Net: **~300 lines of the most defect-dense code in the compiler go away.**

### B4 — Per-fn frame size (P4)

- **Mechanism.** `emit_prologue` hard-codes `sub sp,sp,#16384` (bebop.bp:2857, word `0xd14013ff`); the
  layout is [0..80] callee-saved, [80..256] T43 while-marks, [256..768] x15 spill slots,
  [768..16384] x14 frame heap. All four sizes are known to the planning pass.
- **Expected numbers.** Recursion depth before exit 82 goes from ~4,000 frames to ~500,000;
  `selfcompile_maxrss` (23 MB today) drops by the stack share; the dTLB footprint of a 25-deep
  recursion falls from 25 pages to 1. **The real prize is honesty, not speed**: the fuzzer's
  TRAP-82 class ("SIGSEGV/SIGBUS = stack overflow or wild access", TRAPS.md) is mostly deep recursion
  at 16 KiB/frame; D12-C demands TRAP-82 = 0, and today the classifier cannot tell a miscompile from
  a deep-recursion program. Shrinking the frame converts most of that class into ordinary programs.
- **Gate.** a new construct `c54_deeprec` (recursion depth 100k returns the right fold);
  `fuzz_trap_82 == 0` over a 500-seed batch; `selfcompile_maxrss`; c33/c34/c43 (frame-heap constructs)
  unchanged; exit 81 still raised by `bench/parity_constructs/neg`.
- **Effort** M. **Risk** medium — an under-estimated heap turns a working program into exit 81.
  Mitigation: keep the full 15.6 KiB heap whenever the emitted body contains `mov x0,x14`
  (`count_word(insns, ..., 2853045216)` already exists, bebop.bp:3127).

### B5 — Loop rotation (bottom test)

- **Mechanism.** `emit_while_stmt` emits `[cond][b.cond exit][body][b loop_start]` (bebop.bp:3093-3131):
  **two branches per iteration, one of them always taken**. Emit `[b .test][body][.test: cond]
  [b.cond .body]` instead — one branch per iteration in steady state. The condition source position is
  already saved (`pcond`, bebop.bp:3078), and re-parsing text is something this compiler already does
  (`emit_call` re-parses callee bodies, bebop.bp:3450).
- **Expected number.** -1 word and -1 taken branch per iteration on **every** loop in the tree:
  K4 15->14, K3H 25->24, K1H 11->10 before B3, and it composes with B3 (K4 -> 7, K3H -> 6).
  A78 predicts up to 2 taken branches/cycle, so the ms gain is small but non-zero on short loops.
- **Gate.** all four `k*_loopwords` rows; census `b` count decreases, `b.cond` unchanged; fixpoint.
- **Effort** S/M. **Risk** low; `c07_while`, `c33`, `c34`, `c36_break` are the probes.
- Note: this makes `while` and `if` share one shape, which helps B2.

### B6 — Live-range reuse of x19-x26 (the compiler's own speed, K5)

- **Mechanism.** `sym_bind` (bebop.bp:181-203) is a **monotone counter**: the first 8 distinct names get
  x19-x26, the 9th onwards live in `[x15,#slot*8]` **for the whole function**, with no liveness and no
  reuse. Measured over bebop.bp's own 199 functions (via `tools/bpref.py`'s AST):
  **139 of 199 fns (70%) bind more than 8 distinct symbols**; `emit_body` binds 84. With last-use
  liveness (loop-extended, conservative) the *simultaneously live* count is <= 8 in **99 of 199** fns
  and `emit_body` needs 28, not 84.
- **Expected number.** ~40 functions become spill-free and the rest lose most of their spill traffic ->
  `selfcompile_wall` 1.57 s -> ~1.2-1.3 s, and every std gate compile with it (`becache_cold_ms`,
  `gate_run_ms`). No effect on K1-K4 (single-symbol loops).
- **Gate.** `selfcompile_wall`/`selfcompile_utime` (E1), `becache_cold_ms`, `gate_run_ms` (E4),
  all folds unchanged, fixpoint.
- **Effort** M (the planning pass must record per-symbol last use). **Risk** medium: an over-eager reuse
  is a silent wrong-value bug. Mitigation: the first rung reuses **only** names whose last textual use
  precedes the next binding *and* that are not touched inside any enclosing `while`
  (`sym_is_outer`/`loop_alloc_safe`, bebop.bp:2894/2948, are the same kind of conservative text scan
  and are already gated by c33/c34).
- Note this is the only item here that speeds up the *toolchain loop* rather than generated code.

### B7 — Store: {u,v} record pairs + Morton-ordered CSR for the point/window query

- **Mechanism.** `docs/LANG-DB-DESIGN.md` §8 already did the arithmetic: bebop's 4.0 us window query is
  ~35 random DRAM lines (9 bucket entries + 8.5 points x 3 arrays) at ~100 ns each; storing
  `{u,v}` as adjacent cells removes ~8 lines and a Morton (Z-order) layout that puts the 3x3 window into
  one run of lines removes ~20. **4.0 us -> 1.0-1.5 us**, i.e. G7 row b goes from 30.7x to ~80x of sqlite
  and the PK row from 22.8x toward the same class.
- **Gate.** G7 rows a and b (already frozen at 4x / 10x by D12-F) plus a new "DRAM lines per query"
  column derived from the layout, so the claim is falsifiable without a PMU.
- **Effort** M. **Risk** low (a layout change inside `sbench.bp`/`nnidx.bp`, oracle folds unchanged).
- **Caution.** This is a *layout* win, available to sqlite too; it is not a language claim. Say so.

### B8 — Profile the 45-90 s CSR build **before** doing anything else in the store  ★ best information/hour

- **The anomaly.** `bench/vs_rust/RESULT-sgraph.md:5` records `build (ms) 44793` and
  `RESULT-sgraph.md:17/31` records 39,982 ms and **90,081 ms** for the same phase. `csr_build`
  (selfhost/std/sgraph2.bp:24-52) is a three-pass counting sort, O(m+n). For m = 10-20M edge slots:
  the DRAM floor is ~0.2 s (12 GB/s, ROADMAP platform table), a words-only estimate with today's
  codegen is ~1 s, and a first-touch-write estimate for the whole 512 MB store file is ~1 s
  (7.27 us/page, LANG-DB-DESIGN appendix). **Nothing in any committed doc explains 45-90 s, and the
  2x spread between two committed runs of the same phase is itself unexplained.**
- **Why it matters.** It is the single largest number in the whole repo and it is already on the
  critical path (ROADMAP step 3, "the 45-90 s CSR build profile"). If it is codegen (per-element `bl`
  into `st_get`/`st_put` with today's 41-word call), B1+B3+B4 buy 3-5x on it for free. If it is
  writeback/msync, no codegen rung will ever touch it.
- **Gate.** E9 rows per phase in `bench/perf.csv` + a stated expected value at issue time (L10).
- **Effort** S (measurement only). **Risk** none.

### B9 — K8, the branchy honest kernel, as the *falsifier* for T52-T54

- **Mechanism.** One kernel whose branch is genuinely data-dependent (e.g. `if (lcg(x) & 1) then a else b`
  accumulated over 2M iterations), plus its Rust honest twin. Report the row; write no csel code first.
- **Expected number.** A78 `csel` is 1-cycle and mispredict is ~10 cycles on a 13-stage pipe, so csel wins
  only above roughly a 10-30% mispredict rate; the one published head-to-head measured a *predicted*
  branch at **2.9x faster** than cmov and cmov at 2.9x faster only near a coin flip. The likely honest
  outcome is "T52 pays on one shape, T53/T54 have no target" — the T104b precedent.
- **Gate.** a new `k8h_ms` row; `census` unchanged until the row says otherwise.
- **Effort** S. **Risk** none. **Deletes** T53 and T54 (see §4).

### B10 — Persistent parked workers instead of spawn-per-task (T61 + P8)

- **Mechanism.** The builtins exist (`sys_clone` bebop.bp:993, `sys_futex_wait_guard` 1060,
  `sys_futex_wake` 1094, `sys_atomic_add` 1122, `sys_setaffinity` 1170) and `pool_parity` is 5/5.
  What is missing is a library + a gate.
- **Expected number, bounded honestly.** Thread spawn under proot is ~0.7 ms (SPEEDUP §3); waking a
  parked futex worker is single-digit microseconds uncontended, ~10 us under contention — **1-2 orders
  of magnitude cheaper** than `clone` (~50 us even natively). But three A78 give **1.0-1.4x on
  DRAM-bound streams** and 2.21x measured on the one compute-bound scan (`nn4`, ROADMAP). So: worth
  <= 3x, on compute-bound work only, and never on K1-K4 (serial recurrences).
- **Gate.** T61's own: an affinity probe plus `nn4`-style ns/point on 1 vs 3 cores, with the
  memory-bound row reported next to it.
- **Effort** S/M. **Risk** low, but it competes with `fuzzd` and `boxguard` for the same 3 cores.

### B11 — Things the numbers say **not** to do

| candidate | why not | evidence |
|---|---|---|
| DC ZVA / wider stores for `zeros()` (T75 item 1) | today's loop is `str xzr,[x2],#8` + cmp + b.ne (bebop.bp:4212-4218) = 8 B/cycle = ~19 GB/s, already **above** the 12 GB/s DRAM bus. For arrays that fit L2 the win is real but they are not the bottleneck | ROADMAP platform table; bebop.bp:4212-4218 |
| NEON for i64 work (T64 remainder) | AArch64 Advanced SIMD has **no 64-bit-lane multiply**; only add/sub/shift/compare are 2-wide. `sdot`/`udot` exist on A78 but need 8-bit data. Real only for the 1-2-bit HV path (`hvham`, bebop.bp:720-802), which is already done | https://en.wikichip.org/wiki/arm_holdings/microarchitectures/cortex-a78 ; SPEEDUP §3 M6 |
| Parallel per-fn emission (dev-speed item 7) | ideal 3.0x on 3 A78 against a **1.5 s** baseline, at high risk (per-worker arenas, fntab mutation, determinism) — and boxguard/fuzzd already eat part of the 3 cores | SPEEDUP §7.4 |
| Modules / separate parser-IR-backend binaries (item 2) | there is no IR to hand over and an emitter edit rebuilds everything anyway | SPEEDUP §7.4 |
| Instruction scheduling | 160-entry ROB, 4-wide decode, 10-wide issue — the A78 schedules the 3-8 word loops itself | SPEEDUP §1.4 |
| Chasing the durable-commit 0.15x row | `msync` of one page is 101.5 us under proot; f2fs runs nobarrier. The row measures the sandbox | LANG-DB-DESIGN appendix |
| Packed i32 store cells for the 2.5x size loss | it breaks "persisted objects **are** the in-memory objects", which is the thesis sentence | ROADMAP thesis; store.bp:1-15 |

### Ranking

| rank | item | expected | effort | risk | deletes |
|---|---|---|---|---|---|
| 1 | B8 profile CSR build | information on the repo's largest number | S | none | possibly a whole store rung |
| 2 | B1 prologue/epilogue/x15 | K2H 3.8x -> 2.7x, bin_words -3% | S/M | low-med | 45 lines + a dead param |
| 3 | B2 join convention | K2H -> ~2.1x, every `if` -6 words | S | low | — |
| 4 | B3 tag stack | K3H 4.0x -> ~1.5x, K4 15->8 words | M | med | ~300 lines |
| 5 | B5 loop rotation | -1 word/-1 taken branch per iteration everywhere | S/M | low | — |
| 6 | B4 frame size | TRAP-82 class closed, recursion x100 | M | med | a fuzz-honesty hole |
| 7 | B9 K8 | kills or scopes T52-T54 | S | none | 2 tasks |
| 8 | B6 live ranges | self-compile 1.57 -> ~1.25 s | M | med | — |
| 9 | B7 store layout | window query 4.0 -> ~1.2 us | M | low | — |
| 10 | B10 parked workers | <= 3x, compute-bound only | S/M | low | — |

After B1+B2+B3+B5 the four honest rows should read approximately
**K1H ~1.2x, K2H ~1.6-2.0x, K3H ~1.4-1.8x, K4 ~1.1-1.3x** — every row inside TG-DONE 1's <= 2.0x for the
first time, and within the 1.1x-1.5x band that baseline (one-pass) tiers occupy against optimising
compilers generally. That is the honest end state of the codegen story; **1.0x is not reachable without
inlining and closed forms, and the ROADMAP should say so.**

---

## 3. The IR rung: concrete design

### 3.1 What the survey says about one-pass register allocation

| system | structure | what transfers |
|---|---|---|
| **V8 Liftoff** | `CacheState` = a vector of `VarState`, each `{kStack, kRegister, kIntConst}`, plus `used_registers` and `register_use_count[]`; arm64 pool = 24 GP. Spill picks a round-robin register avoiding `last_spilled_regs`. Joins do a **parallel move** (`MergeFullStackWith`), not a blind spill | **the whole shape.** This is a compile-time value stack with kinds — no op list, no IR, no liveness. Bebop's `fold_try` model (`fntab[3655..3659]`) is already a 2-deep degenerate version of it |
| **SpiderMonkey Rabaldr** | `Stk` with `MemI32 / LocalI32 / RegisterI32 / ConstI32` kinds; `sync()` = spill everything at joins and when a register request fails; the code itself flags this as blunt (bug 1316802) | the **cheap** join rule: "sync at every label". Bebop already has the label barrier (`fntab[3660]`) |
| **V8 Sparkplug** | *no* register allocation at all — mirrors the interpreter frame | the reminder that "no allocation" is a legitimate design point; bebop is past it |
| **LuaJIT** | reverse linear scan over SSA IR: walking **backwards**, the first sighting of a value is its last use | elegant, but needs an IR and a backward walk — the one thing a text-driven one-pass compiler cannot do cheaply |
| **Cranelift `fastalloc`** | single-pass reverse LSRA; compile 1.07-5x faster, **runtime 1.06-7.50x slower** than backtracking; later disabled in wasmtime | the warning: a *general* single-pass allocator can produce far worse code. Bebop does not need one (see 3.2) |
| **QBE** | RPO block order, backward within a block, hint-based coalescing (`sethint`), no interference graph; "70% of the performance in 10% of the code", <8 kloc | proof that hint-biased coalescing without a graph is enough at this quality level |
| **Sethi-Ullman / Ershov** | `SU(leaf)=1`; `SU(n) = l+1` if `l==r` else `max(l,r)`; evaluate the higher child first; spill when `SU > K` | the sizing argument, below |
| **Davidson-Fraser PO**, LLVM `DAGCombiner` vs `PeepholeOptimizer` | production compilers run peepholes **both** on IR and on machine instructions | do not move the fusions off the word stream — move their *matching* onto the tags and keep emitting words |

Sources: https://chromium.googlesource.com/v8/v8/+/master/src/wasm/baseline/liftoff-assembler.h ,
.../liftoff-assembler-defs.h , .../liftoff-assembler.cc , https://v8.dev/blog/liftoff ,
https://searchfox.org/mozilla-central/source/js/src/wasm/WasmBCStk.h ,
https://bugzilla.mozilla.org/show_bug.cgi?id=1316802 , https://v8.dev/blog/sparkplug ,
https://github.com/LuaJIT/LuaJIT/blob/v2.1/src/lj_asm.c ,
https://github.com/bytecodealliance/regalloc2/blob/main/doc/FASTALLOC.md ,
https://github.com/bytecodealliance/wasmtime/pull/10554 , https://github.com/8l/qbe/blob/master/rega.c ,
https://c9x.me/compile/ , https://en.wikipedia.org/wiki/Sethi%E2%80%93Ullman_algorithm ,
https://llvm.org/doxygen/PeepholeOptimizer_8cpp.html , Davidson & Fraser TOPLAS 2(2):191-202, 1980.

### 3.2 The sizing argument — bebop needs 4 registers, not an allocator

I ran the Sethi-Ullman recurrence over every binary-operator tree in **111 files** (`bebop.bp` after
`use` expansion + all 103 `selfhost/std/*.bp` + the `bench630` kernels), using `tools/bpref.py`'s parser
as the AST source:

| SU number (= registers needed to evaluate with no store) | trees | share |
|---|---|---|
| 2 | 11,266 | 95.0% |
| 3 | 574 | 4.8% |
| 4 | 14 | 0.12% |
| >= 5 | **0** | 0% |

The 14 SU-4 trees are in `spectral_profile_fp` (5 copies), `csheaf.check`, `sheaf.delta`,
`base64.b64_decode_char4`. **No committed Bebop program anywhere in this tree needs a fifth scratch
register.**

That is the decisive design fact. It means:

- **No linear scan, no live intervals, no interference graph, no backtracking, no splitting.** Every
  allocator in the survey exists to handle pressure that Bebop's expression grammar cannot generate.
- **A fixed window of x1-x7 with a "spill the deepest tag" fallback provably never spills** on the
  committed corpus. The fallback must still exist (fuzz can generate deeper trees) and must be exercised
  by exactly one synthetic construct.
- Cranelift's fastalloc warning (1.06-7.50x runtime loss) does not apply: that loss comes from
  *general* pressure handling, which is not on Bebop's path.

The **other** register class — the 8 callee-saved symbol registers x19-x26 — *does* have pressure
(§B6: 139 of 199 compiler fns exceed 8 symbols), but that is a separate, later, and lower-value rung.
Keep the two problems apart.

### 3.3 The design

Replace the emitted push/pop pair with a **compile-time operand-tag stack**, held in `fntab` exactly where
the depth model already lives (`fntab[3700]` depth, `fntab[3701+d]` slot tags, bebop.bp:1598-1615 — the
slots exist and are described as "bookkeeping only" today).

```
tag = (kind, payload)
  kind 1 = CONST c          (nothing emitted yet)
  kind 2 = REG r            (value lives in x r, r in 1..7)
  kind 3 = SYM r            (value lives in a bound symbol register x19..x26 — no copy needed)
  kind 4 = SPILL s          (value lives at [x15,#s*8])
```

Five primitives replace `push`/`pop`/`pop2`/`left_single_*`:

- `vs_push_const(c)` / `vs_push_sym(r)` — **emit nothing**. This alone kills the `mov x0,x22` /
  `movz x0,#imm` + push pairs that make up 6 of K4's 15 words and 6 of K3H's 25.
- `vs_alloc()` — take the lowest free register from a 7-bit free mask in `fntab`; if the mask is empty,
  spill the *deepest* tag to `[x15,#s*8]` (Rabaldr's `sync` in its cheapest form; never fires on the
  committed corpus, §3.2).
- `vs_materialise(tag, want)` — emit the one word that puts the tag in a named register
  (`mov`, `movz/movk`, `ldr`), used only when the consumer needs a specific register (x0 for a return,
  x0..x7 for a call, x0 for a syscall builtin).
- `vs_binop(op)` — pop two tags, allocate a destination, emit **one** 3-address instruction
  `op xd, xa, xb`. Constant right operands take the immediate form; power-of-two multipliers take
  `lsl`/`add-shifted` directly (this is `shl_try`/`mulc_try`/`addshift_try` without any word retraction);
  `xB + xA*c` matches `madd xd,xa,xc,xb` on the tags (this is `madd_try` without `writes_producer`).
- `vs_sync()` — materialise every tag into its canonical home. Called at exactly the places that set
  `fntab[3660]` today: before a label, before a `bl`, at a loop back-edge, at an `if` join. This is
  Rabaldr's blunt rule and it is correct by construction; Liftoff's parallel-move refinement is a later
  option, not a requirement.

Everything else in the emitter keeps calling `push`/`pop` — they simply become tag operations.
That is why the effort is M, not L (§3.4).

**The fusions stop being word-pattern matches.** `madd_try`, `shl_try`, `mulc_try`, `addshift_try` and
`cmp_try` currently decode previously-emitted words with bit masks and retract them under a label-barrier
guard (bebop.bp:1910-2020, 2184-2207). On tags the same fusions are two-line predicates on the operand
kinds, with **no retraction and no barrier**, which removes the entire class of bug that
`fntab[3660]`, `writes_producer`, `count_masked` and the `has_adr` check exist to prevent. Peephole on
the word stream stays available for what tags cannot see (production compilers run both levels —
LLVM's `DAGCombiner` and `PeepholeOptimizer`); nothing that exists today needs it.

**Copy coalescing** falls out for free and needs no interference graph: a `SYM r` tag is consumed
directly as an operand register, so the `mov x0,x19` chains never exist; where a destination must be a
symbol register, `vs_binop` writes straight into it (`sub x22,x22,#1` instead of
`mov x0,x22; sub x0,x0,#1; mov x22,x0`). This is QBE's hint bias in its simplest form.

### 3.4 Why "a multi-session refactor of every emit_*" was pessimistic

`grep`: `push(insns` appears **51** times, `pop(insns` **102** times, `pop2` **3** times, spread over 68
functions. But 39 of those functions are `emit_sys_*` builtins whose entire interaction is
"pop N args into x0..xN, emit the syscall, push the result" — they keep their source unchanged and get
correct behaviour from `vs_materialise` inside `pop`. The functions whose **semantics** change are:
`push`, `pop`, `pop2` (replaced), `emit_binop_plain`/`emit_binop_regs`/`emit_binop_regs_plain`,
`emit_cmp_op`/`emit_cmp_regs`, `emit_var`, `emit_lit`, `bind_reg`, `emit_cond`, `emit_while_stmt`,
`emit_let_stmt`, `emit_bl_call`, and the five `*_try` fusions. **Sixteen functions.**

### 3.5 Migration path from today's retractions (D11-G's byte-identity rule)

D11-G requires the first rung to be **byte-identical** so the fixpoint proves losslessness. That is
achievable exactly as follows:

- **Rung 0 (byte-identical, the fixpoint proves it).** Introduce the tag stack with **every tag
  materialised immediately** — `vs_push_sym(r)` emits `mov x0,xr` + push, `vs_binop` pops to x0/x1.
  All existing retraction helpers (`pop2`, `left_single_*`, the five `*_try`s) stay and keep working on
  the word stream. Output must be byte-identical to today's `bebop.bin`; `tools/chain.sh` (non-codegen)
  must report `gen2 == gen3 == gen4` and the battery must be GREEN with no FREEZE. **This rung is the
  whole risk; if it does not go byte-identical in one commit, stop and reconsider.**
- **Rung 1 (lazy consts and symbols).** `vs_push_const` / `vs_push_sym` stop emitting. `pop2` and
  `left_single_*` become dead — delete them in the same commit, because their inputs no longer occur.
  Codegen change: `--codegen` chain, FREEZE=1, all 52 constructs re-frozen, census ALLOW line if the
  emitter's own `b.cond` count grows.
- **Rung 2 (3-address destinations).** `vs_binop` allocates a destination from x1-x7 and writes
  directly into the symbol register when the consumer is a `let` of that symbol. This is where
  K3H's stack round trips and K4's six `mov`s disappear. Gate: `k3h_loopwords <= 10`,
  `k4_loopwords <= 13` **and** `k4_ms <= 3.0`.
- **Rung 3 (fusions on tags).** Rewrite `madd_try`/`shl_try`/`mulc_try`/`addshift_try`/`cmp_try` as tag
  predicates and delete `writes_producer`, `count_masked` and the expression half of the
  `fntab[3660]` barrier. Word budgets must not grow anywhere.
- **Rung 4 (rider, optional).** Attach a type to each symbol-table entry and move the T48 census into
  the compiler (decision 13).

Each rung is one commit, one variable, one gate — the L14 form. B1/B2/B5 (§2) are independent of all of
this and should land **before** rung 0, because they shrink what rung 0 has to reproduce byte-for-byte.

### 3.6 Expected word counts

| kernel | today | after B1+B2+B5 | after rung 2 | after rung 3 | Rust honest twin (shape) |
|---|---|---|---|---|---|
| K4 inner loop | 15 | 14 | 9 | **7-8** | ~5-6 |
| K3H inner loop | 25 | 24 | 9-11 | **6-9** | ~5 |
| K1H inner loop | 11 | 10 | 8 | **6-7** | ~4 |
| K2H per call (executed, else path) | 41 | **~22** | ~19 | ~17 | ~10 (`#[inline(never)]`) |
| gate | — | — | K4 <= 13 (D12-B) | — | — |

ms projections (from today's `k*_ms`, at the measured IPC of each loop):
K4 3.75 -> ~3.4 (latency-bound, small change); K3H 0.52 -> **~0.22-0.28**;
K1H 1.59 -> ~1.35; K2H 1.09 -> **~0.60** after B1+B2, ~0.55 after rung 2.

---

## 4. Tasks to delete or merge

| task(s) | action | reason |
|---|---|---|
| **T53, T54** (sink-predicated stores, masked loops) | **delete** after B9's K8 row exists | Design-only, no gate script, no oracle. A78 evidence is against predication except at high mispredict rates, and `T104b` already set the precedent ("no target on any measured program"). T52 survives only if K8's row says a branch costs |
| **T101-T108** group row in TASKS.md | **delete the row**; T101/102/103/105/106/107/108 DONE, T104 CLOSED | 7 of 8 done; the group row alone inflates the OPEN count and hides that only P2 remains, which is already T96 |
| **T77** (shrinker) | **close DONE** | `bench/fuzz/ladder.py` (b37e1c0) shipped; ROADMAP step 1c already says DONE |
| **T83** ("faster than Rust as a MEASURED TARGET") | **close DONE** | It *is* the ratio column of `REPORT-honest.md` and TG-DONE 1 |
| **T90** | **close DONE** in the HISTORY header | Steps 1/2a/2b/2c all landed; only the header keeps it PARTIAL |
| **T63** (benchmark hygiene) | **merge into D12-A evals** | E7 (thermal/freq/load validity) + pinning + the perf.csv rows are strictly more than T63 asked for |
| **T49** (records = register images) | **close as DONE-by-T43**, delete the "bank image" half | It depends on T26, which is RE-SCOPED away (HISTORY.md:1061). Fixed-offset field access exists (`emit_field_access`, bebop.bp:429) |
| **T50 + T56** (`&f`/`call_cell`, runtime `match`) | **merge into one task** | T56's DONE-CHECK is a table-driven dispatcher, which *is* T50. One feature: a code-offset table plus `blr` |
| **T59 + T73** (reversible arena, snapshot/rollback) | **delete** | D10 already amended both to "the volatile arena only"; the store rolls back by the previous root and that path is gated (G4/G5). Two mechanisms for one job, one of them ungated |
| **T68, T71** (QTT annotations, `bit_identical`) | **delete** | No gate, no number, no dependent. T69 (contracts) survives only as a rider on the T48 checker |
| **T76** (living memory as ONE primitive) | **re-scope to a library over the store, no emitter change** | Three new builtins for a sheaf demo; the store already provides the persistence half |
| **T75 + T64** (integer-exact micro-ops, use the silicon) | **merge; delete the DC ZVA item** | `crc32x` and `hvham` landed; `zeros()` is already above the DRAM bus (§B11); what remains is one S task: apply `hvham2` to `hv.bp`/`deltasync`/`attn` |
| **T92-T95, T84, T85, T87, T88, T89, T62, T67** | **move under one `## PARKED` heading** (reverses D11-J) | Each is a separate project with zero speed and no gate; together they are 65% of the OPEN count and they make the ledger unreadable |
| `docs/FASTPATH-SPEC.md`, `docs/HV_ARCHITECTURE.md`, `docs/LEGACY_BP_ANALYSIS.md`, `bench/FUZZING.md`, `bench/SELFHOST.md`, `bench/VERIFICATION.md` | **move to an attic directory** | All six are marked SUPERSEDED and describe deleted C code; they cost reading time on every "read everything" pass (ANALYSIS §4C, still unapplied) |

### What the numbers refute in the docs

1. `REPORT-honest.md` (Method, final paragraph): "K2H (calls: frame 16 KiB + spills, P4)". **fib has zero
   spills** (one symbol, so `add x15,sp,#256` is dead weight, not a spill) and the frame is one `sub sp`.
   The cost is 8 NOPs + 3 stack round trips + 2 x15/x14 saves + copy traffic (§0).
2. Same paragraph: "K3H (nested loop, no condition fusion on the inner compare, P3)". **P3 landed** —
   `cmp x23,#0 ; b.le` is fused at `k3ht.bin` 0x9c-0xa0. K3H's cost is two push/pop round trips from the
   left-nested sum (§0).
3. `docs/SPEEDUP-ANALYSIS.md` §1.4 predicted K4 at ~24 words after step 3 and 20 after "temps in
   registers"; the retraction path reached **15** with no register tier. The table under-predicted what
   stream retractions could do and over-predicted what remains for the IR.
4. `docs/LANG-DB-DESIGN.md` §5 "Expected G7 shape": PK point lookup "~0.1-0.2 us" (measured **450 ns**,
   2-4x worse) and file size "40 MB" (measured **85.2 MB**, 2.1x worse). Both derivations should be
   corrected in place rather than left as expectations the measurement already contradicted.
5. `docs/LANGUAGE.md` still carries **both** frame-heap paragraphs (the T43 "released at the back-edge"
   text and the pre-T43 "die at return … reset per iteration when the compiler can prove" text).
   ANALYSIS §4A.8 flagged it; D12-D's hygiene commit fixed LANGUAGE.md:9 and the `emit_var` comment but
   not this.
6. ROADMAP's Measured table still leads with the pre-T96 / post-T96 columns whose Rust twins carry an
   in-loop `black_box` (K2 "5.4x"), next to the honest-twin table that says 3.3-3.8x for the same kernel.
   Keep one table.
7. ROADMAP thesis §2 promises the store is gated "against sqlite, **LMDB** and native Rust". Neither LMDB
   nor a native-Rust store twin exists anywhere in the tree (decision 9).
8. "No C anywhere in the toolchain" is true of the runtime path only: GNU `as`/`objcopy` build the entry
   stub and `seed/build/seed`, `honest.sh:19` invokes `rustc`, and the oracle floor is 133 python files
   (ANALYSIS §4A.5/A.6). The sentence needs the qualifier it already has in ANALYSIS.

---

## 5. Questions for the operator (recommended option first)

1. **Order.** Land B1 (prologue/epilogue/x15 sizing), B2 (join convention) and B5 (loop rotation) as
   three single-variable commits **before** the IR rung — or keep D12-B's order and do P2 first?
   *(Recommended: micro-items first; they move K2H, the worst row, and they shrink what the IR rung must
   reproduce byte-for-byte.)*
2. **IR shape.** Operand-**tag stack** (Liftoff `CacheState` shape, no op list) — or the per-fn op-list IR
   as D12-B words it? *(Recommended: tag stack; same information, deletes ~300 lines, M not L.)*
3. **Window.** Declare the window as x1-x7 with a never-firing spill path — or size it to the measured
   requirement of 4? *(Recommended: x1-x7, with one synthetic construct that forces the spill so the
   path is not dead code.)*
4. **Frame.** Shrink the per-call frame to a computed size (B4), accepting that a mis-estimate turns a
   working program into exit 81 — or keep 16 KiB and leave the TRAP-82 fuzz class open?
   *(Recommended: shrink; D12-C's "TRAP-82 = 0" is otherwise not reachable.)*
5. **T52-T54.** Write K8 and delete T53/T54 now, keeping T52 conditional on K8's row — or keep all three
   open? *(Recommended: write K8, delete T53/T54.)*
6. **Store, first move.** Profile the 45-90 s CSR build (B8) before any store code changes — or go
   straight to the Morton/{u,v} layout (B7)? *(Recommended: profile first; it is the largest unexplained
   number in the repo and it may be a codegen problem the IR rung already fixes.)*
7. **LMDB.** Delete LMDB and "native Rust" from the ROADMAP thesis sentence — or measure LMDB once
   through ctypes as the sqlite oracle already does? *(Recommended: delete; the sentence has no script.)*
8. **Workload W.** Is W the dowiz-core order log (T66's `ordfsm.bp`/`money.bp`, which already have
   byte-exact Rust oracles) — or an OSM node extract? *(Recommended: the order log; it is the only
   candidate in-tree with a runnable twin.)*
9. **Size loss.** Report c = 2.5x as the price of the thesis and stop — or spend an M on folding the
   16-byte object header to 8 bytes (85.2 -> ~77 MB, 2.5x -> ~2.2x)? *(Recommended: report and stop;
   the header carries the crc and the layout digest that G1/G4/G5 depend on.)*
10. **1.0x.** Add one line to ROADMAP saying that one-pass tiers land at 1.1-1.5x of an optimising
    compiler and that <= 2.0x is therefore the real target — or leave D1(a) as written?
    *(Recommended: add the line; D12-G already made 1.0x report-only.)*
11. **PARKED.** Reverse D11-J and put the 13 project-sized tasks under one `## PARKED` heading —
    or keep the flat 37-OPEN ledger? *(Recommended: reverse it; the OPEN count overstates remaining core
    work by ~24 items and every new session re-derives that.)*
12. **Fuzz window.** After the IR rung lands, freeze codegen for one 24 h window so `fuzz_seeds_on_bin`
    can reach 10^5 on a single md5 (17.6 h at 1.58 prog/s) — or accept that TG-DONE 8 stays partial while
    codegen work continues? *(Recommended: schedule the window; the counter otherwise resets forever.)*

---

## VERDICT

- I disassembled the three honest kernels with the committed compiler. **K2H's 3.8x is not the 16 KiB
  frame and not spills** (fib has none): 13 of its 41 executed words per call are 8 post-hoc NOPs, a dead
  `add x15,sp,#256` kept alive by `emit_bl`'s own save word, 2 x15/x14 saves per call site, and a 6-word
  push/pop at the `if` join. **K3H's 4.0x is entirely two stack round trips per iteration** because
  `a*3 + x*2 + y*3` is left-nested and `left_single_begin` only slides a one-word left operand.
  `REPORT-honest.md`'s attribution of both rows is wrong.
- Therefore the order should change: three S-effort single-variable edits (prologue sizing, join
  convention, conditional x15/x14 save) take K2H from 3.8x to ~2.1x **without touching the expression
  model**, and they shrink what the IR rung must reproduce byte-for-byte.
- The IR rung should be a **Liftoff-style operand-tag stack, not an op-list IR**: over 111 committed
  files and 11,854 operator trees the Sethi-Ullman number never exceeds **4**, so no allocator, no live
  intervals and no interference graph are needed — and the tag stack **deletes** `pop2`,
  `left_single_*`, `writes_producer`, `count_masked` and the retraction guards of all five fusions
  (~300 lines, the most defect-dense code in the compiler). Only 16 functions change semantics.
  Expected: K3H 25 -> 6-9 loop words (4.0x -> ~1.5x), K4 15 -> 7-8 (gate <= 13 met with margin).
- The largest unexplained number in the repo is the **45-90 s CSR build** (4.5 us per edge for a counting
  sort whose memory floor is ~0.2 s, with a 2x spread between two committed runs). Profile it before any
  store code changes; it may be the same call-overhead problem the codegen rungs already fix.
- Delete rather than add: T53/T54 (A78 says a predicted branch beats `csel` ~2.9x), T59/T73 (two rollback
  mechanisms for one job), T68/T71, T49's bank half, the `T101-T108` group row, and the six SUPERSEDED
  reports — and put the 13 project-sized tasks under one PARKED heading, which reverses D11-J and is the
  single change that makes the ledger tell the truth.
