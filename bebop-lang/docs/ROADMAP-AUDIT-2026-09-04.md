Status: 2026-09-04 CURRENT (read-only roadmap audit by the session-8 analyst agent at commit d60b2c0; task counts move with every commit — ROADMAP.md headers are the source of truth)

# Bebop ROADMAP audit — 2026-09-04 (read-only)

Sources read in full: `ROADMAP.md` (2528 lines), `AGENTS.md` (349),
`docs/SESSION-HANDOFF.md` (126), last 60 lines of `docs/exp.journal`,
`bench/vs_rust/std_golden.sh` (702 lines, **91 `gate` lines**, 91 oracle
`.py` files in `bench/oracles/`), 31 construct-parity sources under
`bench/parity_constructs/` (c01-c27, c30, c31 + `neg/` + `frozen/`).
Doc titles/Status lines: ANDROID (CURRENT), BUG-LEDGER-WEEK (CURRENT,
historical), FASTPATH-SPEC (SUPERSEDED-BY ROADMAP), HV_ARCHITECTURE
(SUPERSEDED), LEGACY_BP_ANALYSIS (SUPERSEDED), SESSION-HANDOFF
(SUPERSEDED), TOKEN-ECONOMY (CURRENT).

Line numbers below are `ROADMAP.md` unless prefixed.

---

## 1. TASK LEDGER

Status text is quoted from the task header (or, where the header carries
none, marked `(no status)` and the body's claim is given). "Evidence" =
what the roadmap itself cites. T46 does not exist (numbering skips it).

### Milestones / NEO columns / SS layer (pre-T)

| id | goal | status as written | evidence cited | deps / notes |
|---|---|---|---|---|
| M1 | seed loader, k1..k7 via seed.bin | DONE (L163-165) | outputs identical to interpreter | — |
| M3 | self-bootstrap fixpoint | DONE | 67816/67816 words, self_check=0 | M1 |
| M4 | CLI in .bp | DONE | version=1000000, k7 -> 3939697352 | M3 |
| M5 | std twins | DONE | std_golden 15/15 (then) | M4 |
| M7 | Zero-C | DONE | native/ deleted, gates green w/o C | M5 |
| N1 | FWHT | `[CORE DONE]` (L244) | gate wht 85001 | — |
| N2 | reversible logic | `[gate rev added]` (L254) — no DONE mark | gate rev 5092789399242 | runtime use = T59 (open) |
| N3 | Ring-VSA | `**DONE**` | gate ringvsa 1110000000544 | — |
| N4 | bit-level Petri nets | (no status) (L277); gate petri 61678606 exists in std_golden | gate petri | dispatcher use = T14 |
| N5 | LSM/reservoir | (no status) (L285); gate lsm exists | gate lsm | — |
| N6 | holographic non-locality | (no status) (L293); gate holo exists | gate holo | loader use = T60 (open) |
| N7 | multiversal superposition | `**DONE**` | gate msuper 1114056100000 | SME/SVE2 = forward-port |
| N8 | spacetime boundary execution | `**DONE**` | gate spacetime 1111100012240 | — |
| SS-1 | NEON Kalman | `**DONE**` | gate kalman 28327900110011 | NEON tiles forward-port |
| SS-2 | vector-calculus invariants | `**DONE**` | gate vecinv 1111018 | — |
| SS-3 | LC resonance timing | `**DONE (core)**` | gates lcres, lcjit; "<1% wall-clock needs bare metal" | — |
| SS-4 | FIR / BIBO | `**DONE**` | gate fir 11104857722880 | "REJECTS-while-of-unknown-depth half is compiler-internal R3.x work" |
| SS-5 | calculus bounding | `**DONE**` | gate calcbound 1024576000 | — |
| SS-6 | matrix decompositions | `[CORE DONE]` | gates spectral 2038, cache 38876254956 | LU/QR/SVD never gated; only power method |
| SS-7 | QLoRA 4-bit | `**DONE**` | gate qlora 1116506000272 | timing half deferred |
| SS-8 | sinc interpolation | `**DONE**` | gate sinc 6684880500081 | window |x|<=1 only |
| SS-9 | attention on NEON | `**DONE**` | gates attn 2008568201, attnt 11 | 128-token timing deferred |
| SS-10 | stride/norm | `**DONE (core)**` | gate stride 11100128016 | L1 hit-rate needs PMU (T15) |
| SS-11 | generation arena | `**DONE (core)**` | gate genarena 1110300000100 | mmap/mprotect half deferred |
| SS-12 | bit matrices | `**DONE (core)**` | gate bitmat 1000024600 | emit-dispatcher swap = compiler work (T56) |
| SS-13 | PIE cache blocks | `**DONE (core)**` | gate pieblock 1100800001 | mmap save/load deferred |
| SS-14 | direct-threaded code | `DEFERRED` (L523) | gate thr 25000411 exists (silicon-pull map L612) | needs emitter rework (T50) |
| SS-15 | eigen coordinate system | `**DONE**` | gate scoord 2010131 | — |
| SS-16 | eigenvalues as control flow | `**DONE**` | gate sgamma 3550431 | — |
| SS-17 | eigentime | `**DONE**` | gate seigtime 1233012011 | runtime form = T58 (open) |
| SS-18 | spectral self-replication | `**DONE**` | gate srepl 8449214 | — |
| FNO core | neural operator | `**DONE**` (L372) | gate fno 111152971008019 | — |

### T1-T100

| id | goal (one line) | status as written | evidence cited | deps |
|---|---|---|---|---|
| T1 | ternary Clifford basis | `DONE ✓ (fold 8888868889989889; gate wired 2026-09-04 by T37, oracle bench/oracles/tern.py)` | gate tern | bits.bp |
| T2 | packed RNS | `DONE ✓ (fold 1183829339; gate wired … T37 …)` | gate rns | — |
| T3 | in-register SNN | `DONE ✓ (fold 65504516937878; gate wired … T37 …)` | gate snn | T1 |
| T4 | L-system fractal memory | `DONE ✓ (fold 144175882039858; gate wired … T37 …)` | gate lsys | — |
| T5 | fractal LOD zoom | `DONE ✓ (fold 1000088904914; gate wired … T37 …)` | gate lod | T1,T4,genarena |
| T6 | time-phantom networks | `DONE ✓ (fold 8328000021)` | gate phant, journal 1788434834 | T3,T4 |
| T7 | RNS spike rotors | `DONE ✓ (fold 1000088888708)` | gate rnsrot | T1-T3 |
| T8 | VSA delta mesh sync | `DONE ✓ (fold 1168535566021)` | gate deltasync | hv.bp; real mesh = T62/T67 |
| T9 | self-mutating L-rules | `DONE ✓ (fold 44349936263)` | gate mutlsys | T4,qlora,srepl |
| T10 | entropic collapse (GC) | `DONE ✓ (fold 3000021007)` | gate entcol | T4,spectral |
| T11 | morph loop (JIT D-I) | `DONE ✓ (morph gate fold 11, morph_loop.sh 8/8)` | gate morph + morph_loop.sh | F1/F2; mprotect RWX blocked |
| T12 | .becache as only pointer | `DONE ✓ (fold 1118234452261)` | gate ptrless | cache,pieblock,sha256 |
| T13 | register-window emitter | `RE-SCOPED 2026-09-04 (untyped window retired by operator decision; typed bank = T25 …)` — **body L765 still says "NOT YET LANDED. This is the ONE OPEN ROADMAP GAP."** | commits b211451/b4326b5/9d9a2ba; journal 1788530945 (window build NOT fixpoint) | superseded by T25 + T96 |
| T14 | dispatcher as execution substrate | `DONE` | gates dispatcher 81001005, substrate 36750250113; commits b549416, 8a5bc33 | hand-written kernels, not compiler output (F-A L1305) |
| T15 | hardware validation / software PMU | `TERMINAL (sandbox-bound, forward-port only)`; T15a `DONE, commit 4152ec1` | gate swpmu 2001000110000000000 | perf_event_open EACCES; no SVE/SME on A78 |
| T16 | Einstein notation + metric | `DONE ✓ 2026-09-04 (gate tdg 3162519640442167 == bench/oracles/tdg.py)` | gate tdg | bt,matrix |
| T17 | Christoffel + covariant derivative | `DONE ✓ … (gate tdggeo 219599976738721791 …)` | gate tdggeo; journal notes Rust `christoffel()` is a zero stub — oracle is textbook, not Rust | T16 |
| T18 | Riemann/Ricci/scalar | `DONE ✓ … (gate tdgcurv 4262143808388606 …)` | gate tdgcurv (S² K=1) | T17 |
| T19 | forms + exterior d | `DONE ✓ … (gate tdgforms 1000351400006779 …)` | gate tdgforms (d∘d=0) | T16 |
| T20 | tensor query engine | `DONE ✓ … (gate tq 722997760 …)` | gate tq (4 queries, nearest idx) | T16,spectral,bt |
| T21 | Stokes audit | `DONE ✓ … (gate tdgstokes 173698403 …)` | gate tdgstokes | T19,T20 |
| T22 | Grassmann Λ5 | `DONE ✓ … (gate grass 10312435099105887 …)` | gate grass | tern,bits |
| T23 | Cl(4,1) ternary CGA | `DONE ✓ … (gate cl41 1807759285641197332 …)` | gate cl41 | tern,T22 |
| T24 | supercommutator + select-equivalence | `DONE ✓ … (gate zgrade 5676760058329986817 …)` | gate zgrade (256/256 select) | T22,T23 |
| T25 | Z2 bank ABI S1-S3 | (no status) = not started | none; "uncommitted working-tree diff … revert" note L1116 | T22-T24 |
| T26 | register-resident records | (no status) = not started | none | T25,bt,store |
| T27 | cellular sheaf | `DONE ✓ … (gate sheaf 1114020060 == oracle)` | gate sheaf | csr,matrix,vecinv |
| T28 | H^0 query / harmonic iteration | `DONE ✓ … (gate sheafh0 11121890396072 == oracle; it_t=396 it_c=72 recorded)` | gate sheafh0 | T27,spacetime,swpmu |
| T29 | content-addressed sheaf nodes | `DONE ✓ … (gate csheaf 5155430002134088 == oracle)` | gate csheaf | T27,ptrless,cache |
| T30 | string diagrams | `DONE ✓ … (gate sdiag 654345454 …)` | gate sdiag | T24,graph,petri |
| T31 | rewriting to normal form | `DONE ✓ … (gate rewrite 38233233101031 …, 0.2 s)` | gate rewrite; journal 1788535599: hang was a compiler bug (fn named `match`), repro filed, NOT fixed in compiler | T30 |
| T32 | ad-hoc query JIT | (no status) = not started | none | T26,morph,swpmu |
| T33 | MVCC via nilpotent tokens | `DONE ✓ … (gate mvcc 68412663603207 …)` | gate mvcc | T22,T27,genarena,rev,entcol |
| T34 | Z2 STM | `DONE ✓ … (gate stm 871596764015151 …)` | gate stm | T22,T24,T25(!),T28,T33,T21 — **DONE despite dep T25 open**: gate simulates the bank in cells |
| T35 | register wave filter | (no status) = not started | none | T24,T25,T26,substrate |
| T36 | committed oracles | `DONE ✓ 2026-09-04 (run_all ok=82, self-frozen=0; L17 added to AGENTS.md)` | run_all ok=82 (now 91 per journal) | — |
| T37 | wire orphan gates | `DONE ✓ 2026-09-04 (std_golden 82/82; drift 5903978048000947864)` | 6 gate lines | T36 |
| T38 | dead-std triage + prelude | `DONE ✓ 2026-09-04 (44 modules in attic/ … +6 gates bitset dp modular rle search set; std_golden 91/91)` | attic README, invariants (v) | T36 |
| T39 | bpref + fuzzer | `PARTIAL 2026-09-04 (bpref.py landed …; fuzzer in progress)` | fuzz N=450 in T42 text; gen.py recursion bug (135 GENFAIL) | T36 |
| T40 | structural invariants | `DONE ✓ 2026-09-04 (GREEN on HEAD; planted census increase caught RED)` | invariants.sh, check_abi.py, census.py | — |
| T41 | one design corpus | `DONE ✓ 2026-09-04 (banners + index row + seed.S:55 + Status: lines on every doc)` | journal 1788537947 | — |
| T42 | fix R3.x(a)-(e) | `PARTIAL 2026-09-04 (… open: (a) grammar and (b) `>>` under D5 measurement (tools/prec_switch.sh))` | c25-c31 gates, neg/c28,c29, fixpoint ff27a910 | T39 |
| T43 | lift L8 + nesting bans, struct literals | (no status) = not started | none | T39,T40 |
| T44 | self_check honesty | `DONE ✓ 2026-09-04 (c1-c36 re-frozen, c37-c41 + run_program deleted, self_check()==0, fixpoint a03c546…)` | journal 1788537820 | — |
| T45 | retire expr_compile.bp (port clone/futex/LSE) | (no status) = not started; D4 says "sys_clone is root-caused NOW" | none | T40 |
| T47 | `use "path"` | not started | none | T38 |
| T48 | checked types | not started | none | T24/T25 |
| T49 | records = register images (struct) | not started | none | T26,T43 |
| T50 | functions as cells `&f` | not started | none | T48 |
| T51 | branch census gate | `DONE ✓ 2026-09-04 (census.txt frozen: bebop 872 b.cond/133 cbz/0 tbz …)` | census.txt; journal: now 906 b.cond after T42 fixes (increase accepted as "new fns") | T40 |
| T52 | pure if -> csel | not started | none | T24,T51 |
| T53 | sink-predicated stores | not started | none | T52,T43 |
| T54 | bounded loops -> masked | not started | none | T52 |
| T55 | substrate codegen `compile --substrate` | not started ("the terminal move") | none | T50,T52-54,T14,T26 |
| T56 | runtime match branchless | not started | none | T50 |
| T57 | substrate runtime prelude | not started | none | T47,T55 |
| T58 | eigentime scheduler | not started | none | T55 |
| T59 | reversible arena mutation path | not started | none | T55,T57 |
| T60 | holographic artifact loader | not started | none | T57 |
| T61 | threads + cores (affinity) | not started | none | T45; ptrace clone |
| T62 | network syscalls | not started | none | T45; proot net (probe) |
| T63 | benchmark hygiene (pinning) | not started | none | T61 |
| T64 | use CRC32X/SHA256H/CNT | not started | none | T36 |
| T65 | dowiz master index row | `DONE ✓ 2026-09-04` | CORE-ROADMAP-INDEX row | T41 |
| T66 | money/ordfsm Rust-oracle twins | `DONE ✓ 2026-09-04 (gates money 872656672063013 + ordfsm 346243789026198 == cargo-run PRODUCTION …)` | gates money, ordfsm; bench/oracles/rust | T36,T48(!) — DONE with T48 open |
| T67 | bebop2 mesh bridge | not started | none | T62,T66; cross-repo drift |
| T68 | QTT `^0 ^1 ^w` | not started | none | T48 |
| T69 | contracts as gates | not started | none | T68,T40 |
| T70 | effects pure/io | not started | none | T48 |
| T71 | `bit_identical` decl | not started | none | T64,T39 |
| T72 | affinity builtins | not started | none | T45 |
| T73 | snapshot/rollback/on_fail | not started | none | T59 |
| T74 | WFE at quiescence | not started | none | T57 |
| T75 | integer-exact micro-opts | not started | none | T51,T72; T55 |
| T76 | living memory primitive | not started | none | T29,T20,T70 |
| T77 | counterexample shrinker | not started (journal mentions "fuzz DIVERGE-2131 shrunk" — by hand or tool unclear) | none | T39 |
| T78 | token streams `.bt` I/O | not started | none | T26,T35 |
| T79 | hvnav tooling | not started | none | hv,T47 |
| T80 | cas:// imports | not started | none | T47,sha256 |
| T81 | `test` blocks | not started | none | T68,T38 |
| T82 | replay debugger | not started | none | T57,T59 |
| T83 | "faster than Rust" as measured target | not started | none | T63,T72 |
| T84 | glyph canonical surface | not started | none | T39,T47 |
| T85 | proof kernel in .bp | not started ("critical, design-bound") | none | T68,T69,bt |
| T86 | bounded DPLL | `CORE DONE ✓ 2026-09-04 (gate dpll 584168922 == oracle, 20 formulas; T69 hookup pending)` | gate dpll | T69 (open) |
| T87 | f64 at boundary | not started | none | T70 |
| T88 | supervisor cell library | not started | none | T55,T73 |
| T89 | trust chain + DDC | not started | none | T45 |
| T90 | `bebop.bin check` line:col | not started | none | T48 |
| T91 | x86_64 backend | not started | none | T40,T51,T57; x86 host |
| T92 | Verilog from .bt | not started | none | T55; FPGA fwd-port |
| T93 | WGSL export | not started | none | T20; GPU fwd-port |
| T94 | WASM emitter + interp | not started | none | T91 |
| T95 | SPIR-V emitter + sim | not started | none | T93 |
| T96 | register tier for expressions | not started (D2 approved) | none; baseline K1 = 50 words/iter | T42 |
| T97 | RSS memory column | not started | none | — |
| T98 | multi-core substrate gate | not started | none | T45,T72, clone root cause (D4) |
| T99 | unary/hex/return/break | half DONE inside T42 (`unary -/! + 0x hex … landed 2026-09-04 … T99 literal-forms half DONE`); return/break open | c30_unary | T42,T43 |
| T100 | sqlite oracle for tq latency | not started | none | T20,T97 |

### Header/body/journal contradictions

1. **"Verified state" (L129-192) is stale**: says std_golden 82/82,
   run_all ok=82, construct 24/24, bebop.bin md5 88d4cd5d. Journal and
   T38/T42 headers say 91/91, ok=91, construct 31/31, fixpoint md5
   ff27a910 (after a03c546 → b1489f05 → b4a83ebd → 6b0d5343 → 8e318100).
   `std_golden.sh` actually has 91 gate lines. Honest flag 3 (L2218) and
   T36 header also still say 82.
2. **T13**: header `RE-SCOPED`, body L765 `NOT YET LANDED. This is the
   ONE OPEN ROADMAP GAP.`; SESSION-HANDOFF.md L23 still names T13 "THE
   ONE OPEN ROADMAP GAP" (file marked SUPERSEDED at its top, body not
   rewritten). Progress log L2390-2416 repeats the T13 "ACTIVE NEXT"
   plan three times.
3. **Honest flag 4 of the SILICON pull (L951)**: "Math layers + substrate
   complete" vs Closure Honest flag 5 (L2224): "substrate EXECUTION of
   compiled programs is T55 and is not done." The later flag is right.
4. **T34 DONE with T25 in its DEPS** (L1252) and **T66 DONE with T48 in
   its DEPS** (L1778): the gates were built without the bank/typed
   surface; the deps lines were never edited.
5. **T51 census**: header freezes 872 b.cond; journal records the census
   re-frozen upward to 906/907 after T42 work ("+6 bcond = the new fn").
   The invariant "fails on any INCREASE" is being re-baselined at each
   commit, so it is a ratchet only between commits, not across them.
6. **F-B (L1313)** says T1-T5 headers "now read GATE NOT WIRED -> T37";
   they were later flipped to DONE (correct), but F-B text was not
   updated to past tense. Same for F-C (oracles) — closed by T36.
7. **Predicted speedup table (L2463-2528)** keeps "15-50× vs SQL",
   "24-50MB", "<1ms cold start" rows with no measurement column; T83/
   T97/T100 exist precisely to fix this and are open — TG-DONE 6 is
   violated by the roadmap's own text today.
8. **T17 oracle**: journal 1788530890 notes dowiz-core `christoffel()`
   "is a zero stub; oracle mirrors the textbook formula" — the
   "port-from-reference" claim (L104-115) does not hold for T17; the
   Rust reference was not the oracle.
9. **T31**: journal shows the compiler mis-parses a user fn named
   `match` (repro `bench/fuzz/repros/T31-match-fn-name.bp`); worked
   around by renaming, no task tracks the compiler defect.
10. **T15 workaround list (L840-856)** documents LPE-exploit paths as
    "documented, not executed"; nothing in the ledger, but note it is a
    roadmap that lists privilege-escalation exploits as options.

---

## 2. REMAINING WORK

Counts over the 99 numbered tasks (T1-T100, no T46):

| bucket | count | ids |
|---|---|---|
| DONE (header says DONE ✓ / DONE) | **38** | T1-T12, T14, T16-T24, T27-T31, T33, T34, T36, T37, T38, T40, T41, T44, T51, T65, T66 |
| CORE DONE | 1 | T86 |
| PARTIAL | 3 | T39, T42, T99 |
| RE-SCOPED / TERMINAL (closed by decision, no more in-sandbox work) | 2 | T13, T15 |
| not started | **55** | everything else |
| **remaining (needs work)** | **59** | 55 + T39 + T42 + T99 + T86 |

Remaining by size (S = commit/hours, M = day, L = several commits/days,
XL = research-grade): **S 13, M 33, L 10, XL 3**.

Kind: C = compiler (edits `bebop.bp`, single-writer per L14/L16),
G = new `.bp` gate + oracle, T = tooling/bench script, D = docs.

### 2.1 Critical path (D6 L1457: "T42 to completion FIRST, then T96, clone/T45, T52"; then B-ladder L2192-2200)

| order | id | size | kind | blocked on |
|---|---|---|---|---|
| 1 | T42 rest: (a) precedence grammar, (b) `>>` ASR / `>>>` LSR, delete R3.x laws | M | C+T | D5 oracle-switch measurement (`tools/prec_switch.sh`) over 91 gates; std modules audited for abs-before-shift |
| 2 | T96 register tier (3 steps) — target K1 <= 16 words/iter | L | C | T42; construct bins regenerated with asserted delta |
| 3 | T45 retire expr_compile.bp; sys_clone root cause (D4) | L | C | clone under ptrace/proot: `par_tids(4)` returns 0 while Rust threads work — the defect is bebop-side, unlocalized |
| 4 | T52 pure if -> csel | M | C | T24 (done), T51 (done) |
| 5 | T53 sink-predicated stores | M | C | T52, **T43** |
| 6 | T54 bounded loops masked | M | C | T52 |
| 7 | T50 functions as cells | M | C | **T48** |
| 8 | T55 substrate codegen (TG-DONE 1, then 2) | XL | C+G | T50, T52-54, T26; performance unknown (flag 1, L2212) |
| 9 | T57 substrate prelude | L | C+G | **T47**, T55 |
| 10 | T58 eigentime scheduler | M | G | T55 |

### 2.2 Layer C — compiler debt (serialized on bebop.bp)

| id | size | kind | blocked on |
|---|---|---|---|
| T39 rest: gen.py recursion bug, 10^5 programs, FUZZING.md rewrite | M | T | none |
| T43 L8 lift, nested-if/assign bans, struct literals | L | C | T39 |
| T99 rest: `return`/`break` | S | C | T43 |

### 2.3 Layer B (bank + records) — T25/T26/T32/T35 (SUPER-SHEAF ladder L1268)

| id | size | kind | blocked on |
|---|---|---|---|
| T25 S1 ABI (+6 words/fn), S2 rehome 8 emitters (71 words), S3 typed slots + gate `zbank` | L | C+G | none; check_abi allowlists to empty |
| T26 regrec | M | C+G | T25 |
| T32 qjit | M | G | T26, morph |
| T35 wave filter | M | G | T25, T26 |

### 2.4 Layer S — surface

| id | size | kind | blocked on |
|---|---|---|---|
| T47 `use` | M | C | T38 (done) |
| T48 checked types | L | C | T25 for parity |
| T49 struct = bank image | M | C | T26, T43 |
| T56 runtime match | M | C | T50 |

### 2.5 Layer R — runtime (all after T55)

| id | size | kind | blocked on |
|---|---|---|---|
| T59 reversible arena | M | C+G | T55, T57 |
| T60 holoload | M | G | T57 |
| T61 threads/affinity | M | C | T45; honest 5-skip under ptrace |
| T62 sockets | M | C | T45; proot net probe |
| T63 bench pinning | S | T | T61/T72 |

### 2.6 Layer H / D

| id | size | kind | blocked on |
|---|---|---|---|
| T64 CRC32X/SHA256H/CNT | M | C | none (parallel-safe now per L2201) |
| T67 mesh bridge | M | T | T62, T66, cross-repo drift |

### 2.7 Corpus-A carry-over T68-T85 (ordering L2015-2017, L2200-2204)

| id | size | kind | blocked on |
|---|---|---|---|
| T77 shrinker | S | T | now |
| T79 hvnav | S | G+T | now (T47 for `use`) |
| T83 measured-target column | S | D | T63, T72 for numbers |
| T72 affinity builtins | S | C | T45 pattern; proot may EPERM |
| T75 DC ZVA (early) / rest | M | C | T51; rest after T55 |
| T68 QTT annotations | M | C | T48 |
| T70 effects | M | C | T48 |
| T69 contracts | M | C | T68, T40 |
| T81 test blocks | M | C+T | T68, T38 |
| T71 bit_identical | S | C | T64, T39 |
| T80 cas:// | S | C | T47 |
| T73 snapshot/rollback | M | C+G | T59 |
| T74 WFE | S | C | T57 |
| T82 replay | M | T | T57, T59 |
| T76 living memory | M | C+G | T29, T20, T70 |
| T78 tokens | M | C+G | T26, T35 |
| T84 glyph surface G1-G5 | L | C+T | T39, T47 |
| T85 proof kernel | XL | G+C | T68, T69 |

### 2.8 Rejected-list additions T86-T95

| id | size | kind | blocked on |
|---|---|---|---|
| T86 rest (T69 hookup) | S | C | T69 |
| T87 f64 edge | S | C+G | T70 |
| T88 supervisor | M | G | T55, T73 |
| T89 DDC/trust chain | M | T+D | T45 (witness compiler must compile current surface — it does not: F-D) |
| T90 check line:col | M | C | T48 |
| T91 x86_64 backend | XL | C | T57; x86 host for execution |
| T92 Verilog + vsim | L | G | T55 |
| T93 WGSL | M | G | T20 |
| T94 WASM + wasmi | L | C+G | T91 |
| T95 SPIR-V + spvsim | L | C+G | T93 |

### 2.9 SPEED & CORES T96-T100

| id | size | kind | blocked on |
|---|---|---|---|
| T96 | L | C | T42 (see 2.1) |
| T97 RSS column | S | T | none — parallel-safe now |
| T98 substrate4 | M | G | T45, T72, clone |
| T100 sqlite oracle | S | T | T20 (done), T97 |

### Totals

- Done: 38 (+1 CORE DONE, +2 closed by decision) = 41 closed.
- Remaining: 59 = S 13 / M 33 / L 10 / XL 3.
- Of the 59, **~38 touch `bebop.bp`** (single-writer queue); ~21 are gates/tooling/docs that can run in parallel.
- Parallel-safe right now with zero compiler edits: T97, T100, T77, T79, T83(draft), T64, T39-rest, T93.

---

## 3. TERMINAL GOAL

### As stated (L17-98)

"Bebop is a post-von-Neumann self-hosting agent language — a single living
mathematical structure that maps directly to silicon. It erases the
boundary between memory, compiler, text and processor architecture. There
are no traditional instruction lines, no syntactic sugar, no virtual
machines, no garbage collectors, no intermediate interpreters." (L19-23)

"What the language IS" enumerates nine properties (L25-98):
1. Post-von-Neumann substrate: no PC, no call stack; an async event
   dispatcher over dense activity bit arrays via tzcnt/popcnt + SVE2 with
   threshold accumulation; "Code lives only where a spike fires."
2. Holographic memory + ranked arenas: one immutable linear arena, mmap,
   64B-aligned, rank-4 `.bt` tensors + CSR; FWHT spectral smearing so
   deletion "smoothly redistributes spectral fingerprints", eliminating
   dangling pointers and segfaults.
3. Spectral engine + eigentime: time = Hotelling deflation iterations;
   spectral-gap violation prunes branches before execution.
4. Multi-tier spectral stack: FWHT/Haar (micro), NTT (meso), KLT (macro);
   DFT/FFT/DCT/DST/Z/DHT "rejected forever".
5. Hardware fusion on ARM SME/SVE2.
6. Reversible logic: all arena mutations via Toffoli/Fredkin XOR masks;
   instant rollback without snapshots.
7. Multiversal superposition of hv4096 states with deterministic collapse.
8. Tensor database as the language (T16-T21): data = tensor fields,
   queries = Einstein summation, integrity = Stokes, schema = metric
   tensor/Jacobian; "memory, persistence and the compiler are ONE type
   system"; dowiz-core Rust is the reference geometry.
9. Z2-graded register bank + cellular sheaf store (T22-T35): x9-x10 even
   / x11-x13 odd typed register file; storage = cellular sheaf; queries =
   H^0; records = register images; CRUD = CoW + nilpotent tokens.

Terminal criterion sentence (L90-98): "'done' is defined falsifiably in
TERMINAL-GOAL CLOSURE §TG-DONE … Audit 2026-09-04: every N/SS/T 'DONE'
is a standalone gate demo (class a); none is yet integrated into emitted
code or the runtime (class b) except foldx/whileb/r3x/morph; none is
hardware-validated (class c)."

### TG-DONE criteria verbatim (L1404-1431) and their status

> 1. **Substrate execution of compiled programs.** `bebop.bin compile
>    --substrate` turns a `.bp` program into (a) branch-free cell kernels
>    and (b) an incidence/activity `.bt` tensor; the runtime sweep
>    (`activity != 0`) is the ONLY conditional branch in the executable
>    image. Gate: branch census of the image == 1 conditional branch; fold
>    == the linear-mode fold for every std gate and for K1-K4.

Closed by: T50, T52, T53, T54, T55 (+T57 runtime, T26 cell state).
Progress: **~5%** — only the measuring instrument exists (T51 census
baseline: 906 b.cond, 135 cbz in bebop.bin; every `if`/`while` is a real
branch, F-E L1341). No csel path, no cell cutting, no `--substrate` flag.
T14's substrate.bp is hand-written and runs on a PC (L1311-1313).

> 2. **Self-hosting on the substrate.** `bebop.bp` compiled in substrate
>    mode compiles itself to a byte-exact fixpoint (bb2 == bb3) in
>    substrate mode.

Closed by: T55 terminal rung. Progress: **0%**; the linear-mode fixpoint
exists (ff27a910) and is the only self-hosting evidence. Depends on 1.

> 3. **Every gate has a committed independent oracle** (python or Rust)
>    that reproduces the frozen fold from scratch; a gate without one is
>    labelled `self-frozen` in this file, never "proven".

Closed by: T36, T37 — **DONE (100%)**: 91 gate lines, 91 `bench/oracles/
*.py`, run_all ok=91 self-frozen=0 (journal 1788537947), L17 in AGENTS.md.
Caveat: "independent" is weak for gates whose python mirror was written
from the same author's definition (T17 explicitly not Rust-backed); only
8 are backed by Rust (6 spectral_golden + money/ordfsm).

> 4. **Zero tolerated miscompiles**: the R3.x(a)-(e) laws are deleted
>    because the defects are fixed and regression-gated; no ban list
>    remains in "Design laws" except capacity limits with loud traps.

Closed by: T42, T43, T39. Progress: **~55%** — (c),(d),(e) fixed with
construct gates c25-c31 + neg/c28,c29; (a) precedence and (b) `>>` open
pending D5 measurement; L8 (no alloc in while) and the nested-if/assign
bans still in Design laws (L212-217); fuzzer at N=450 with a generator
bug, not 10^5; new defect found and not fixed (fn named `match`, T31
journal). Design laws block still lists bans.

> 5. **Single compiler, single language**: `selfhost/expr_compile.bp` is
>    retired; every construct the language accepts is in construct_parity;
>    every std module is gated or in an explicit attic.

Closed by: T45, T38, T42/T43. Progress: **~45%** — attic done (44
modules), 91 gated; expr_compile.bp (3128 lines) still the only owner of
sys_clone/futex/atomics and the only thing that builds pool_*.bp;
`struct` literals disabled, `module core {}` inert, types discarded
(F-F) — "every construct accepted is in construct_parity" is not
checkable while the accepted surface is undocumented.

> 6. **Hardware claims are measured or labelled forward-port**, never
>    projected in a table without a measurement column.

Closed by: T83, T97, T63, T72, T100. Progress: **~40%** — T15 forward-
port list exists and swpmu gives deterministic steps; but the "Predicted
speedup and memory" section (L2463-2528) still projects 15-50× vs SQL,
24-50MB RSS, <1ms cold start with no measurement column, and no bench
is core-pinned (F-I: big.LITTLE 4xA55+4xA78, 2-20× noise).

### D1 breakthrough metrics (L1449-1452)

"D1. Breakthrough metric = ALL of (a) K1-K4 >= 1.0x Rust single core
pinned wall-clock, (b) throughput on N cores through the substrate,
(c) tensor-query latency vs SQL. Each gets its own measured gate."
Status: (a) today 2.6-10.6× SLOWER (L2378-2379, L1985), no pinned gate;
(b) no multi-core anything (clone broken, T98 open); (c) no sqlite
oracle (T100 open). **0 of 3 metrics have a gate.**

---

## 4. ASSESSMENT — the post-von-Neumann thesis

### What is claimed vs what is proven

| claim | roadmap location | gate evidence (class a: standalone demo) | class b (in emitted code/runtime) | class c (hardware) |
|---|---|---|---|---|
| Dispatcher substrate, no PC | T14 L797-808; flag 4 L951 | `dispatcher` 81001005, `substrate` 36750250113: k1 chain->36 (9 sweeps), fib(25) (25 sweeps), hand-written cells | none — the substrate loop is itself `while activity != 0` compiled to cmp+b.eq by a stack-machine emitter (F-A L1311) | none |
| Branch elimination (supercommutator/select) | T24 L1060; T52-T56 | `zgrade` 256/256 select-equivalence, `bitmat` 256 patterns | none: 906 b.cond in bebop.bin, `if` = 2 branches (F-E L1341-1349) | none |
| Substrate runtime (eigentime scheduler, reversible arena, holographic loader) | T57-T60 | `seigtime`, `rev`, `holo` as arithmetic demos | none | none |
| Tensor DB = the language | T16-T21 L862-940; L67-84 | tdg/tdggeo/tdgcurv/tdgforms/tq/tdgstokes: 2x2 metrics, S² curvature, 4-query nearest, Stokes on squares | none: no builtin, no storage, no persistence path; tq is 4 points, not "1M-point manifold" | none |
| Sheaf store / H^0 query | T27-T29 | sheaf/sheafh0/csheaf: one edge table, tree + cycle, 5 inserts | none | none |
| MVCC / STM without WAL | T33/T34 | mvcc (64 LCG steps, 2 slots), stm (2 interleaved slots) | none; T34 "bank" is simulated in cells since T25 is open | none |
| Register-resident records (zero deserialization) | T25/T26 | none — T25 not started; x9-x13 currently scratch in 8 emitters (71 words) | none | none |
| Holographic artifact | T60 / N6 | `holo` WHT smear demo | none | none |
| "Language IS the database" | L67-84, T20, T100 | tq gate | no query path from source to store; no latency number | none |
| Multi-core / "thousands of connections per µs" | D1(b), T98, Honest flag 2 L946 | `fiber` cooperative scheduler in ONE process | sys_clone returns 0 (bebop-side defect, D4) | none |
| SME/SVE2 fusion | L55-59, N7, SS-1/9/10 | nothing — CPU has no SVE/SME (F-I L1394) | none | impossible on this box |

Summary: the mathematics is demonstrated in 91 small, deterministic,
oracle-backed folds. That is real and non-trivial. But **every one of
the thesis's operational claims — no program counter, no branches,
memory = database = compiler, zero deserialization, reversible arena,
holographic loading — is class (a) only**: a program that itself runs
as ordinary AArch64 with a call stack and 900+ conditional branches
demonstrates the arithmetic of a model in which those things would not
exist. The roadmap says this itself (L90-98, F-A L1305-1313, flag 5
L2224); the ledger merely confirms nothing changed since that audit
except T36-T44/T51/T65/T66 (truth floor) and T22-T24/T27-T31/T33/T34
(more class-a gates).

### The hard blockers, in order of severity

1. **The stack-machine emitter is the real bottleneck, and every
   substrate step is behind it.** K1 hot loop = 50 words/iteration, 8
   `sub sp/str/ldr/add sp` quartets, 6 of them push-immediately-pop
   (L1436-1441); Rust = 3 words. R6.2 folding fires only for const-const
   and var+imm12. The one attempt to fix this (T13 register window,
   five R4 attempts + S1-S3 landed/disabled/reverted, L757-796, journal
   1788530945 "NOT fixpoint") failed on a layout fact the roadmap only
   verified afterwards (x9-x13 used as scratch by 8 emitters). T96 is
   the third design for the same problem. Until it lands, T52-T55 would
   be emitting csel/cells around a 15× instruction bloat — flag 1
   (L2212) admits substrate mode "may be slower than straight-line
   code".

2. **sys_clone defect** (D4, L1442-1445): `par_tids(4)` through
   expr_compile.bp returns 0 while Rust threads work under the same
   proot. Multi-core (D1 b), T45, T61, T63, T72, T98 all sit behind an
   unlocalized bebop-side bug in a 3128-line second compiler whose
   builtins have not been ported. Nothing in the journal tail shows a
   hypothesis for it yet.

3. **Sandbox/proot**: no mprotect RWX (morph uses file-backed RX),
   perf_event_open EACCES, ptrace makes pool an "honest 5-skip", 2-20×
   timing noise on unpinned big.LITTLE, no SVE/SME silicon. Every
   hardware sentence of the terminal goal (L55-59, "2.4GHz", "L1 hit
   >95%", "<5ms cold start", "nanosecond CRUD") is forward-port by
   declaration (T15 L810-868). The roadmap's T15 "workaround paths" (root
   patching, LPE exploit, QEMU TCG, ptrace PMU virtualization, SIGILL
   SVE emulator) are listed but "not executed"; two of the five are
   privilege escalation.

4. **TG-DONE 2 (self-hosting on the substrate)** requires compiling a
   3400-line compiler with 123 fns, recursion, syscalls and string
   scanning into branch-free cells with a fixed recursion cap (flag 2
   L2214: "unbounded recursion is a von Neumann call stack by
   definition"). Nothing in the tree — not T14, not any gate — shows a
   single *compiled* function running on the substrate. The step from
   "k1 chain hand-encoded as 9 cells" to "bebop.bp as cells" is not a
   ladder rung; it is the entire project. No estimate of cell count,
   sweep count or memory for the compiler exists.

5. **Compiler surface debt underneath everything** (F-F L1360-1376):
   types discarded, struct literals disabled, `module` inert, no
   `return`/`break`, `>>` semantics undecided, second compiler still
   needed for pool, a fn named `match` mis-parses (T31 journal), fuzzer
   generator broken at 135/450. TG-DONE 4-5 must close before 1-2 can
   be trusted, and D6 correctly orders it that way — which means the
   substrate work is realistically months out, not "next session".

6. **"Port-from-reference" is partly fictional**: T17's Rust reference
   `christoffel()` is a zero stub (journal 1788530890); the oracle is a
   textbook mirror. The de-risking argument at L104-115 rests on Rust
   code that in at least one case does not implement the math.

### Fraction of the terminal goal reached

- Honesty floor (TG-DONE 3-6): 3 is done; 4/5/6 roughly half. Weighted
  ~55% of the floor.
- Substance (TG-DONE 1-2): ~5% and 0%.
- D1 breakthrough metrics: 0/3 have gates; (a) is currently 2.6-10×
  the wrong way.

Honest overall estimate: **~15-20% of the terminal goal as the roadmap
itself defines it**, most of it the truth floor. If one counts "the
mathematics of every layer exists as a gate" as the goal (the framing
of L602-606 "~70% of its mathematics is ALREADY gate-proven"), the
number is higher, but that framing is exactly what F-A and TG-DONE were
written to reject.

### Realistic path and risk

Path (matches D6/L2192-2205): T42 close (M) → T96 (L, the third try at
register residency; risk: same heisenbug class as T13, R4 x5) → T45 +
clone root cause (L, unknown depth) → T43/T47/T48/T50 (surface, ~4
weeks single-writer) → T52-T54 (csel/masked loops, M each; risk: the
FASTPATH R4#3 lesson that a branch-per-op made K1 6× slower — selects
are not free) → T55 k1/k2 rung (XL; first real class-b evidence) →
T57 prelude → K3/K4 → std gates → bebop.bp (TG-DONE 2).

Risks, in order:
- **Serialization**: ~38 of 59 remaining tasks edit `bebop.bp` under a
  one-commit-per-variable law with a ~60s/gen fixpoint and a 91-gate +
  31-construct battery. Throughput is bounded by one writer.
- **Performance inversion**: every rung to TG-DONE 1 is allowed to be
  slower ("ship whatever the numbers are"). D1(a) >= 1.0× Rust and
  TG-DONE 1 (one conditional branch) may be mutually exclusive on a
  Cortex-A78 with a branch predictor; the roadmap has no experiment
  scheduled to find out before T55 (T52's "N frozen after measurement"
  is the closest).
- **Scope creep still active**: T68-T95 added 28 tasks on 2026-09-04
  (glyph surface, proof kernel, x86_64, WASM, SPIR-V, Verilog) on the
  same day the audit found the compiler discards types. Three of them
  are XL. None advances TG-DONE 1-2.
- **Oracle independence**: python mirrors written by the same agent from
  the same prose; a systematic misreading survives both sides. Only 8/91
  gates have a foreign oracle.
- **Environment**: any bare-metal/root number (T15) requires leaving the
  sandbox; the roadmap's listed escape routes include kernel exploits,
  which should be struck.

What would move the needle fastest (no new tasks needed): T96 + T45
with a clone repro under `bpref`, then a T55 spike that compiles ONE
straight-line fn to cells and measures sweeps vs linear mode — before
the 25 surface/typing tasks, to learn whether TG-DONE 1 is worth its
cost on this ISA.
