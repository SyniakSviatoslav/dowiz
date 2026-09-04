Status: 2026-09-04 CURRENT (read-only critique by the session-8 critic agent at 7e09005; decisions D-A..D-M pending the operator)

# ROADMAP.md critique — is this a plan that reaches its own terminal goal?

Date 2026-09-04, read-only. Sources: ROADMAP.md @ 704c611 (2670 lines), AGENTS.md, docs/SPEEDUP-ANALYSIS.md,
docs/LANG-DB-DESIGN.md, docs/ROADMAP-AUDIT-2026-09-04.md, docs/SESSION-HANDOFF.md, docs/exp.journal tail,
bench/{substrate_spike,tq_sqlite}/RESULT.md, bench/vs_rust/REPORT-pinned.md, bebop.bp (4059 lines, working
tree has an uncommitted +75 T99 return/break diff + c35/c36), seed/seed.S, std_golden.sh, tools/bpref.py,
bench/fuzz/gen.py, construct_parity.sh, invariants.sh, census.txt. Six 3-line probe programs were compiled
with the SHIPPED bebop.bin (md5 ca404b5f) and run through the seed, then run through bpref.py — no
self-compile, no battery; sources in scratchpad/probes/. "L" = ROADMAP.md line unless prefixed.

## (a) Executive summary

1. The document now holds TWO terminal goals. L17-98 still promise no program counter, no call stack, no
   segfaults, eigentime, SME/SVE2, reversible arena, a typed x9-x13 register bank and "no SQL, no WAL
   locks"; D8-D10 (L1462-1535) and the T55 spike killed or re-scoped every one of those. L93-98 even cites
   a TG-DONE text ("one conditional branch", "substrate-mode fixpoint") that D9 deleted. Nine sentences of
   the goal are dead letters; the plan and the goal no longer describe the same project.
2. TG-DONE 1 as rewritten is no longer falsifiable: "predication at every site where the measured time
   does not regress" is satisfied by zero sites, and "≥ 1.0x the Rust twins" is measured against K1/K3
   twins that are deliberately crippled (black_box inside the loop, k1.rs:6). TG-DONE 2 ("self-hosting
   stays linear, byte-exact fixpoint") is ALREADY true today — it is a regression invariant, not a goal.
3. The real thesis after D8-D10 is small and honest: a one-pass self-hosting AArch64 compiler whose
   compiled code is ~1-2x Rust, plus an LMDB-style persistent object store benchmarked against sqlite.
   That thesis has ~22 tasks on its critical path (T43/T47/T48 → T101-T108 → T109-T117); the other ~45
   open tasks (glyphs, proof kernel, x86_64, WASM, SPIR-V, Verilog, DDC, hvnav, mesh, sockets, WFE,
   holographic loader, ...) do not touch it.
4. Verification has a systematic blind spot: bebop.bin is memory-unsafe with no diagnostics beyond exit
   codes, and every checker is built so that it cannot see this. Probes today: recursion depth 600 →
   SIGSEGV (16 KiB frames, bpref says 600); out-of-bounds read → silent neighbour value; zeros(40M) →
   SIGSEGV with no trap; an enum ctor inside a loop containing `b[i] = i` → SIGSEGV even after T43's L8
   lift; a user `fn match` compiles and returns 0. gen.py generates none of these shapes by construction.
5. Oracle independence is weaker than "91/91 == oracle" reads: 83 of 91 python mirrors were written by
   the same agent from the same prose; nobody has shown any oracle is non-tautological (no mutation test
   of gate vs oracle exists).
6. "Byte-exact fixpoint" proves self-consistency, not correctness; construct parity after a FREEZE=1
   re-freeze checks VALUES only and accepts any word stream; the branch census "never increases" but is
   re-frozen upward at every commit (872 → 1035 b.cond today). The three structural checks that were
   meant to catch a silent codegen regression each have an escape hatch that is used routinely.
7. Every speed number is one box, one A78 core, under proot, medians of 11, no counters, cycles assumed
   at 2.4 GHz. That is fine for ratios between Bebop versions, not for "≥ 1.0x Rust" as a terminal
   criterion. This shell's affinity mask is 0x7f (CPU 7 excluded) — the "four A78" in T106 may be three.
8. T101-T104 (temporaries in x1-x7/x9-x13, cmp fusion, frames, peephole) are stream-retraction edits of
   the same class that failed five times as T13 and produced two latent holes in T96 step 1 (label cell,
   rd=0 barrier). There is no IR; "LICM and scheduling in .bp" (D8(1)) has nowhere to live.
9. The store spec (D10) is accepted without its own spike, on the same day as its research doc, and it
   has one hole the prior-art matrix itself names: a writer mapping the file RW MAP_SHARED while running
   bounds-check-free code is LMDB's MDB_WRITEMAP hazard — any wild write corrupts the persistent file in
   place, and only the crc on read notices.
10. "Language IS the database" is now measurable but not yet defined: the T100 win is an index (M2) plus
    compiled scans (M1); the sqlite ratio shrinks from 13.8x to ~9x once the ctypes floor is removed
    (LANG-DB §8); there is no real workload, no concurrency under load, no durability proof possible on
    this box (f2fs nobarrier), and no comparison against LMDB/rkyv or a native Rust scan as ceiling.
11. Process: 36 commits and three binding decisions (D8-D10) on 2026-09-04, on numbers measured the same
    day; 17 tasks added the same day; the working tree carries an uncommitted compiler diff (T99);
    ROADMAP.md is 161 KB with a "Verified state" block (L129-140) three fixpoints stale.
12. What is genuinely solid: the gate+oracle discipline, the fuzzer as far as it reaches, the T96 result
    (K1 loop 51 → 14 words with zero memory ops), the sqlite gate, the substrate spike that killed a
    wrong idea with a number, and the honesty of the two analyst reports.
13. Recommended shape of the fix: rewrite the goal to the D8-D10 thesis in ≤ 30 lines; make TG-DONE a
    list of frozen numbers and gates; add three runtime capacity traps and widen gen.py before T101; add
    a mutation test per oracle; put a word budget on construct re-freeze; park ~45 tasks; spike the store
    before binding T109-T117; re-baseline D1(a) against honest Rust twins.
14. Cost of the fix: ~2 days of tooling and docs, no new compiler risk; it removes the four ways the
    project can currently declare victory without having earned it.
15. Thirteen decisions below, five cheap experiments; the first three experiments (honest twins,
    widened fuzz, oracle mutation) take one hour together and each can kill a top-5 risk.

## (b) Findings

Severity: B = blocks TG-DONE (or makes it unfalsifiable), M = distorts measurement, W = wastes effort, C = cosmetic.

| id | area | finding (mechanism) | sev | evidence |
|---|---|---|---|---|
| F1 | coherence | Goal L25-30 "no program counter, no call stack" vs D8(2)/D9(1): the substrate moves INTO the compiler and emits linear code; the T55 spike measured the runtime-cell model 41x/740x against. The sentence is dead. | B | L25-30, L1470-1475, L1756-1780, substrate_spike/RESULT.md |
| F2 | coherence | Goal L31-40 "eliminating dangling pointers and segmentation faults entirely" vs reality: arrays are headerless absolute pointers with no bounds check; probes p1/p2/p5/p6 segfault or read garbage. | B | bebop.bp:104-125 (sym_bind), emit_zeros 3669 (no cmp x27,x28); probes below |
| F3 | coherence | Goal L41-46 eigentime "time is measured not by clock ticks" vs D3 (pinned clock_ms is THE primary column) and D9(3) (T58 eigentime moved to robustness, "must never appear in a speed claim"). | C | L41-46, L1455, L1498-1500, L2587-2608 |
| F4 | coherence | Goal L55-59 SME/SVE2 fusion: the CPU has neither (F-I L1385-1394); every SME/SVE sentence is forward-port by declaration and no task can close it in-sandbox. | C | L55-59, L826-829 |
| F5 | coherence | Goal L60-63 "all arena mutations via reversible gates" vs D10/T73 amendment: the store rolls back by root swap, the XOR journal only governs the volatile arena and only after T59 which sits behind T55 (re-scoped) in ROBUSTNESS. | W | L60-63, L1526-1530, L1936-1948, L1790 |
| F6 | coherence | Goal L85-92 "fixed register file x9-x13 is a TYPED two-sector space" vs D9(2): x9-x13 become compiler temporaries; the bank is a library convention. Two sections (SUPER-SHEAF decisions L1000-1012, T25 body L1073-1121) still read as binding. | C | L85-92, L1490-1493, L1073 header vs body |
| F7 | coherence | Goal L67-84 "no SQL, no relational tables, no WAL locks" vs D10: single-writer lock (atomic cell + futex), superblock A/B, append-only log = a WAL after-image by the design doc's own words (LANG-DB §4c "SQLite's WAL is the same idea"), and every store gate is measured AGAINST sqlite. | C | L67-84, L1507-1535, LANG-DB §4c, §4h |
| F8 | coherence | L93-98 "Terminal criterion ... one conditional branch in the image, substrate-mode self-hosting fixpoint" cites the PRE-D9 TG-DONE text; TG-DONE 1-2 at L1404-1420 say the opposite. Same file, two criteria. | B | L93-98 vs L1404-1420 |
| F9 | TG-DONE | TG-DONE 1 (rewritten) has no falsifier: "predication at every site where the measured pinned time does not regress, with N frozen after measurement" is satisfied vacuously (N = 0 sites); "branch census ... recorded" is bookkeeping. The only number left is D1(a) K1-K4 ≥ 1.0x — see F10. | B | L1408-1416 |
| F10 | measurement | D1(a) compares against twins where K1 and K3 keep `black_box(s + i)` INSIDE the loop (forces the accumulator through memory, ~5x slower than LLVM's honest loop); K2 is against an inlined fib; only K4 is honest. "≥ 1.0x Rust" on K1/K3 is "≥ 1.0x a Rust program compiled to be slow". | B | rust_once/k1.rs:6, k3.rs:7; SPEEDUP §2 lines 112-121 |
| F11 | TG-DONE | TG-DONE 2 "self-hosting stays linear, byte-exact fixpoint" is already satisfied (gen3 == gen4 ca404b5f). A criterion that is true on the day it is written is an invariant, not a goal; TG-DONE therefore contains one vacuous and one already-met "substance" item. | B | L1417-1420, commit 704c611 |
| F12 | TG-DONE | Nothing in TG-DONE names the store (D10) or the query gates (D1(c)); the operator's accepted thesis has no terminal criterion. | B | L1404-1431 vs L1507-1535 |
| F13 | verification | Oracle independence: 91 python mirrors, 8 backed by Rust (6 spectral_golden + money/ordfsm); T17's Rust reference is a zero stub so the mirror is a textbook. No mutation test exists: nobody has shown that changing the .bp changes the fold AND that the oracle was not derived from the same reading of the same prose. | M | AUDIT L404-412, L536-539; bench/oracles/ (91 .py), bench/oracles/rust/ |
| F14 | verification | gen.py never generates: `return`/`break` (docstring L10), allocation inside while bodies (call() `return self.lit()` under loop_depth), recursion deeper than 7 (`& 7` guard), multi-arg or mutual recursion, strings, `char`/`str_len`/`clock_ms`/any `sys_*`, structs, fn names that collide with keywords, arrays > 512, out-of-range indices (every index is masked), loops > 6 iterations, calls in `while` conditions. The fuzzer is green precisely over the surface where the compiler is known to work. | B | gen.py:4-12, 192, 263, 296-298 |
| F15 | verification | bpref has no frame heap (15 KiB), no 16 KiB frames, no arena end, no stack: it computes 600 for a depth-600 recursion where bebop segfaults (probe p1), 5 for zeros(40M) where bebop segfaults (p5), an IndexError for an OOB read where bebop returns a neighbour cell (p2). Div by zero, MIN/-1, shift ≥ 64 DO agree (p3 = 111007 both). | B | probes p1,p2,p3,p5; bpref.py:28 DEPTH_CAP 5000, :419 sys_ stubs |
| F16 | verification | T43's L8 lift is a TEXT heuristic: `let _ = b[i] = i` in a loop body is classified "bare store" → unsafe → no x14 reset → an enum ctor in the same loop leaks 16 B/iteration → SIGSEGV at ~1000 iterations (probe p6; bpref 2999). "Never a wrong value" holds; "no crash" does not, and gen.py cannot see it (F14). | B | bebop.bp:2367 loop_alloc_safe, :2410 store_eq_pos; probe p6 |
| F17 | verification | A user `fn match(a)` compiles to a program returning 0 (probe p4); bpref rejects it. Known since journal 1788535599 (T31), tracked by no task. Keyword detection by characters (`is_br`, hash dispatch) has no reserved-word table. | B | probe p4; bebop.bp:1114 emit_call_or_ctor; AUDIT L200-202 |
| F18 | verification | "Fixpoint byte-exact (gen3 == gen4)" proves the compiler is a fixed point of itself: deterministic and self-consistent. It does not prove any construct is compiled correctly (a miscompile that is idempotent under self-application passes; every historical miscompile c25-c31 coexisted with a green fixpoint). | M | L1417-1420; journal 1788538390-1788545697 (fixpoint green while 56 repros diverged) |
| F19 | verification | construct_parity FREEZE=1 copies the candidate .bin over the frozen one whenever the VALUE matches (construct_parity.sh:74); word deltas are printed, not gated. After a re-freeze, a regression that adds 20 dead words per construct while preserving values is accepted; the only guard is a human reading WORD_DELTA lines. | M | construct_parity.sh:27-30, 74; journal 1788552120 (31 re-freezes in one commit) |
| F20 | verification | census "never increases" is re-frozen with `--freeze` at every commit that adds conditionals: bebop bcond 872 (T51 baseline) → 906 → 922 → 933 → 944 → 971 → 974 → 1035 today. The ratchet exists between commits, never across them; the journal justifies each increase in prose. | M | invariants.sh:34-37; census.txt row 2; journal 1788556374 |
| F21 | measurement | One box, one core (taskset -c 4), proot, R=11 medians, no PMU, cycles = ns × 2.4 assumed; p95 of K4 is 18 ms vs median 13 (40% spread). The pinned in-process spread pinned/unpinned = 1.00x refutes the 2-20x folklore, which is good, but "≥ 1.0x" as a terminal criterion needs a noise floor and a second machine. | M | REPORT-pinned lines 135-142; SPEEDUP §0 method notes |
| F22 | measurement | This shell's affinity mask is 0x7f (`taskset -p $$` → 7f; nproc 7; /proc/cpuinfo 8): CPU 7 is excluded in this sandbox. REPORT-pinned's "usable [0..7]" was measured from python. T106 nn4 "4 A78" may run on 3. | M | shell probe today; REPORT-pinned line 123 |
| F23 | measurement | The "Measured speed" table (L2629-2670) is at T96 step 1 (104b6291); step 2 (K1 14 words) and T43 (+1 word) have no measured row; SESSION-HANDOFF repeats step-1 numbers. The rule "no row without a measurement column" is met, "rows follow the binary" is not. | C | L2637-2646; journal 1788553160 |
| F24 | process | 116 numbered tasks (T1-T117, no T46): 41 DONE + T86/T96 effectively done; 6 re-scoped/terminal; ~67 open (3 partial). 17 added today (T101-T117); 36 commits today closed ~6. Peak pace ≈ 3 compiler tasks/day; ~19 of the 22 critical-path tasks touch bebop.bp under single-writer law. | W | ROADMAP headers (grep `^\*\*T`), git log 2026-09-04 |
| F25 | process | Resume-driven / off-path tasks (no effect on D8-D10 thesis): T84 glyphs, T85 proof kernel (XL), T91 x86_64 (XL), T92 Verilog, T93 WGSL, T94 WASM, T95 SPIR-V, T89 DDC, T79 hvnav, T78 tokens, T82 replay, T88 supervisor, T76 living memory, T67 mesh, T62 sockets, T60 holoload, T57/T58/T74, T59/T73, T56, T50, T49, T32, T35, T68-T71/T81 typing chain, T87, T90, T75 rest, T64. ≈ 45 tasks. | W | task list L1669-2226, L2587-2626 |
| F26 | process | D8, D9, D10 were all made on 2026-09-04 on measurements from 2026-09-04 (T55 spike, T100, SPEEDUP, LANG-DB). D10 binds nine tasks (T109-T117) to a 672-line design with no spike of its own. | W | L1462-1535; LANG-DB status line ("PROPOSAL pending operator decisions") |
| F27 | process | ROADMAP.md = 161 KB / 2670 lines: goal + four superseded pulls + progress log + task ledger + decisions + measured tables. "Verified state" (L129-140) says md5 88d4cd5d, std_golden 82/82, construct 24/24 (actual: ca404b5f, 91/91, 34/34). AUDIT §1 found 10 header/body/journal contradictions; three of the ten are still present (Verified state, F-B/F-C past tense, T25/T26 body text). | C | L129-140; AUDIT L166-205 |
| F28 | process | The working tree is dirty: bebop.bp +75 lines (T99 return/break, emit_return_stmt :2443, emit_break_stmt :2459, trap 98), c35/c36, bpref +21 — while the last commit says "invariants GREEN". Every "green" claim in this critique is about HEAD, not the tree. Single-writer law says nothing about uncommitted state at session end. | C | git status; bebop.bp:2443-2473 |
| F29 | process | "XL" tasks (T85, T91, T55-as-was) have no DONE-CHECK a machine can run in-sandbox ("emitted bytes gated by disassembler diff", "critical, design-bound"). "Done" for them means "an agent stopped". | W | L2106-2125, L2176-2192 |
| F30 | technical | The compiler is a one-pass string scanner with no token stream and no IR: keywords by first characters and a 131-hash; symbol table = 128 (name, reg, srcpos) triples with 8 registers and spills via x15; fntab magic cells 3655-4000; every fn frame is 16 KiB (`sub sp,sp,#0x4000`, emit_prologue :2273); `struct_kill = 1` (:186); types discarded (:127). T101-T104 must be written INSIDE this. | B | bebop.bp:53-125, :127, :186, :1305-1360, :2273 |
| F31 | technical | T101-T104 are stream retractions with a label barrier (fntab[3660]) — the exact mechanism whose predecessor (T13) failed five times and whose T96 step 1 shipped with two holes found only by objdump diffing (rd=0 barrier, label cell leaking across fns). x1-x7/x9-x13 are caller-saved: temporaries must flush across every `bl` (the T13 blocker #3), and x9-x13 are still scratch in 8 sys emitters (71 words; T25 verified facts) — D9(2) hands them to T101 without rehoming. | B | L757-796, L1073-1090; journal 1788552120; bebop.bp:1336 pop, :1499 pop2 |
| F32 | technical | D8(1) "an optimizer pass in .bp (scheduling, LICM, strength reduction)" needs a per-fn op list with explicit labels and def-use; today's stream is words with patched branch offsets (patch_jumps :2474). T104 "peephole over the word stream" can pattern-match `mul` by constant; it cannot hoist across a loop header it cannot see. | B | bebop.bp:2474-2490, L2515 |
| F33 | technical | Runtime capacities with NO trap: arena end (x28 never compared, probe p5), frame heap (x14 vs 15 KiB, p6), recursion (16 KiB × depth vs 8 MiB main stack ≈ 512 frames, p1; thread stacks are 64 KiB carved from the arena, pool.bp:36 → depth 4 overflows silently into arena data), array bounds (p2). Design law L214 "capacity asserts on every fixed table" is applied to compile-time tables only. | B | seed.S:55-68; pool.bp:5,36; bebop.bp:3669-3703; probes |
| F34 | technical | Diagnostics = exit codes 90-94 (seed), 95-98 (compiler) with no message, no position. T90 (line:col) is scheduled after T48. A second human cannot tell "expected `)`" from "17 returns". | W | bebop.bp:1855-1862; seed.S:141-159 |
| F35 | technical | The seed is "frozen" (1496 B, RX file-backed .bin + 256 MB anonymous arena). The store design needs a file-backed MAP_SHARED mapping (available via sys_mmap in .bp — fine) but threads need stacks (today inside the arena, 64 KiB each) and the arena has no guard pages between stacks and data. Nothing in seed.S can change without breaking every golden and the trust chain (T89). | M | seed.S; LANG-DB §4b "seed NOT changed"; pool.bp:5 |
| F36 | security | Executing code: seed runs any .bin it is handed; T80/D10 gate store-named code by sha256 match in `.bcas`, computed by software sha256.bp — sound only if the .bcas directory is trusted. No ASLR story is written down (mmap(0,...) gives kernel randomisation; the .bin is position-dependent via absolute `adr`/entry offsets? — unverified). W^X is clean (file-backed RX). The larger hole is F37. | C | seed.S:31-46; L2009-2020; LANG-DB §4d last row |
| F37 | store | LANG-DB §4b: "the writer maps PROT_READ|WRITE MAP_SHARED". With no bounds checks (F2/F33) any stray store in the writer process lands in the persistent file instantly ("visible to the file at the instant of the store instruction", §4g). This is LMDB's MDB_WRITEMAP hazard, quoted in the design's own prior-art row, and it is absent from §6 risks. crc on read detects, never prevents. | B | LANG-DB §1 LMDB row, §4b, §4g, §6 |
| F38 | store | Language surface the store assumes: struct literals + field access (T43 open, struct_kill), `use` (T47 open), checked `ref T` (T48 open), strings as byte arrays (no string type), i32/packed cells (none: file size loses 2.2x by the design's own estimate). The STORE PULL's DEPS line admits all four. | B | L2582-2585; LANG-DB §4i |
| F39 | store | "Language IS the database" is not defined anywhere as a falsifiable property. The measured content is: (M1) compiled scans, (M2) a grid index, both of which sqlite/Rust also have; the 13.8x is ~9x native (ctypes floor 19 us of 55, LANG-DB §8); no real workload (LCG points), no concurrency under load (mvcc/stm are single-threaded LCG simulations), no durability proof possible (f2fs nobarrier), no schema change in anger, no LMDB/rkyv/Cap'n Proto/native-Rust comparison; file size 2.2x worse; update/reopen ties. | B | LANG-DB §8 lines 419-425, 466-470; SPEEDUP §6.1 |
| F40 | measurement | T100's "scan 9.9x < 10x = FAIL by 1%" is reported honestly but the rule was set (SPEEDUP §4.3) after the mechanism was known; pass thresholds written by the same agent that writes the code (10 us / 3x / 10x) are not independent targets. | M | L2531-2541; SPEEDUP §4.3, tq_sqlite/RESULT.md |
| F41 | missing | No task provides: runtime capacity traps (F33); a language reference (the grammar lives in bpref.py's docstring, no README at repo root, selfhost/readme.md is a banner); overflow/bounds semantics as documented LAW (only div/shift are documented); a string type; packed cells; a benchmark set beyond K1-K4 (self-compile 108.7 s is itself the largest real program and has no kernel row); an ABI/version stamp in .bin (footer is 8 bytes = entry offset only); running without proot (docs/ANDROID.md only); memory limits (arena fixed 256 MB, no MAP_NORESERVE); the A55 cluster; the `fn match` defect; committing T99. | B | this file's probes; bench/vs_rust/rust_once/; seed.S:47-49; AGENTS/ROADMAP grep |
| F42 | process | T15 (L840-856) lists kernel-patching and LPE-exploit "workaround paths" in a roadmap; AUDIT flagged it; still present. | C | L840-856 |
| F43 | coherence | "Invariant policy" L322-334 releases golden determinism "where it conflicts with N1-N8" — a standing licence to change any fold under a new-basis label; unused so far but it contradicts L17 (L17 = no gate without oracle). | C | L322-334, AGENTS.md L17 |

Probe results (shipped bebop.bin, seed; scratchpad/probes/p*.bp). Caveat: a concurrent session promoted
bebop.bin ca404b5f → 4c454e21 (T99 fixpoint) at 23:19 while the probes ran, so each probe used one of the
two; both are green fixpoints and none of the probed shapes is touched by T99 (return/break):

| probe | bebop | bpref | class |
|---|---|---|---|
| p1 `down(600)` recursion | SIGSEGV (rc 139) | 600 | stack: 16 KiB frames × 600 > 8 MiB |
| p2 `a[8]` on zeros(8) with b adjacent | 77 (b[0]) | IndexError | no bounds |
| p3 `7/0`, `MIN/-1`, `1<<64`, `1<<65`, `7%0` | 111007 | 111007 | agree (hardware semantics mirrored) |
| p4 `fn match(a)` called as `match(41)` | 0 | SyntaxError | silent mis-parse |
| p5 `zeros(40000000)` (320 MB > 256 MB arena) | SIGSEGV | 5 | no arena-end trap |
| p6 enum ctor in a 3000-iteration loop with `b[i] = i` | SIGSEGV | 2999 | T43 heuristic says unsafe → leak |

## (c) Decisions for the operator

### D-A · Which terminal goal is binding? (F1-F8, F43)
Question: L17-98 or D8-D10?
Options:
1. Rewrite L17-98 to the D8-D10 thesis in ≤ 30 lines (self-hosting AArch64 compiler at ≥ X× Rust on an
   honest kernel set; the language's object model is the persistent store, measured against sqlite/LMDB;
   cores for parallel scans). Move the nine dead paragraphs to HISTORY.md as "vision 2026-08, superseded".
   Cost: 1 hour. Benefit: one goal; every task can be judged against it.
2. Keep L17-98 as "north star, non-binding" with a banner; TG-DONE is the only binding text.
   Cost: 10 min. Benefit: keeps the prose; risk: the next agent plans against the prose again (it has
   happened twice: T13, T55).
3. Leave as is. Cost: 0. Risk: F8 stays — the file contradicts itself on its terminal criterion.
Recommendation: 1 — a goal that the plan has already abandoned cannot be reached by the plan.

### D-B · Make TG-DONE falsifiable and sufficient (F9, F11, F12)
Question: what replaces TG-DONE 1-2?
Options:
1. TG-DONE 1 := a frozen table {K-set kernel, honest Rust twin, pinned in-process ms, target ratio} with
   the current numbers as the baseline row and the pass rule "every row ≤ target"; TG-DONE 2 := demote to
   AGENTS law (fixpoint per codegen commit); add TG-DONE 7 := store gates G1-G8 green with numbers;
   TG-DONE 8 := widened fuzz (D-D) ≥ 10^5 programs, 0 CRASH/DIVERGE, capacity traps only.
2. Keep TG-DONE 1-2 text, add a footnote defining N ≥ 1 site. (Cheap, still vacuous at N=1.)
3. Delete TG-DONE 1-2, keep 3-6 (honesty floor only). Honest but then the roadmap has no substance goal.
Recommendation: 1 — every criterion becomes a number in a committed script or a gate count.

### D-C · Honest Rust twins before any "≥ 1.0x" claim (F10, F21, F40)
Question: what is the K-set and its twin?
Options:
1. Redefine K1/K3 so LLVM cannot close-form them WITHOUT black_box in the loop (loop-carried nonlinear
   recurrence, e.g. `s = s*3 + i` wrapping; K3 likewise), keep K4, make K2 `#[inline(never)]`; re-baseline
   D1(a); state noise floor (p95/median) per row. Cost: 1 hour. Effect: today's ratios roughly double
   (K1 ~2.5-5x); D1(a) becomes an honest, harder target.
2. Keep the twins, rename the column "vs Rust-with-black_box" everywhere. Honest label, weak target.
3. Both columns. Cost: 2 hours; benefit: the crippled column stays comparable to history.
Recommendation: 3 for one release, then 1 only — history preserved, target honest.

### D-D · What does bebop do at its runtime capacities, and does the fuzzer look? (F14-F17, F33)
Question: UB, trap, or check?
Options:
1. Loud traps for the three capacities: `cmp x27,x28; b.hs trap` in emit_zeros (2 words per zeros), frame-
   heap bound check in emit_array_lit/emit_enum_ctor (2 words per allocation), recursion via a guard page
   (mprotect PROT_NONE below the main stack — one syscall at entry, no per-call cost; threads: guard page
   per 64 KiB stack). Exit codes 80-82 with the fault class. No bounds checks on arrays (stay UB, documented).
   Cost: ~3 commits + re-freeze. Benefit: crashes become deterministic verdicts the fuzzer can classify.
2. Full bounds checks on every array access (`[len]` header): correctness for user programs, ~2 words +
   1 branch per access; census grows; conflicts with "no headers" unless T48 lands first.
3. Document UB, leave gen.py masked. Zero cost; F2's goal sentence must then be struck (D-A).
And in all cases: widen gen.py (return/break, alloc in loops, recursion to 200, multi-arg recursion,
keyword-named fns, unmasked indices under option 2, strings/char/str_len, loops to 10^4) and run 10^5
before T101 starts — this is the T39 DONE-CHECK the roadmap already owes.
Recommendation: 1 now, 2 as part of T48 (`[T]` with length) — the roadmap's own "capacity asserts, never
silent" law, applied to the runtime.

### D-E · Prove the oracles are not tautological (F13)
Question: how independent must an oracle be?
Options:
1. Mutation gate: `tools/mutate_gate.sh` flips one operator/constant in each gate's .bp; std_golden must
   FAIL that gate. Proves fold sensitivity (kills "the fold ignores half the program"). Cost: 1 hour, runs
   in the battery. Does not prove the mirror was written independently.
2. A second oracle for the ~12 gates the D8-D10 path rests on (csr, bt, store, tq/nnidx, mvcc, stm, sha256,
   crc32, sort, rng, money, ordfsm): Rust via the existing cargo generator, or a python written from the
   SPEC without reading the .bp, by a different agent with the .bp withheld. Cost: 1 day.
3. Accept as is; relabel "== oracle" as "== mirror" in the file. Cost: 10 min; honest.
Recommendation: 1 + 3 now, 2 for the store gates as they are written (the store oracle must be built from
the byte layout spec, never from store.bp).

### D-F · Close the three escape hatches in the structural checks (F18-F20)
Question: how does a value-preserving codegen regression get caught?
Options:
1. Word budget per construct: FREEZE=1 accepted only if words(new) ≤ words(frozen) OR the commit carries
   `WORD_BUDGET <construct> +N because ...`; census `--check` accepts an increase only from an ALLOW file
   listing (fn, +bcond) that the commit adds. Cost: 2 hours of shell. Benefit: every growth is a written,
   greppable decision instead of a journal sentence.
2. Freeze disassembly text and require `git diff` review of the .dis per construct. Heavier, human-bound.
3. Keep as is. The hatch stays open at every re-freeze (34 constructs re-frozen twice today).
Recommendation: 1.

### D-G · T101-T104: same mechanism as T13, or an IR first? (F30-F32)
Question: continue stream retractions, or build a per-fn op list?
Options:
1. Continue as planned (retractions + fntab[3660] barrier + objdump diffs). Cost: ~4 commits; risk: the
   T13 class (five failures) and the flush-across-bl problem for x1-x7/x9-x13 temporaries; x9-x13 scratch
   in 8 sys emitters must be rehomed first (T25 S2 was costed at 8 commits).
2. Build a per-fn op list (op, operands, label ids) as the emitter's output, with a final linearise +
   patch step; T101-T104 and any LICM become passes over the list with explicit def-use; the retraction
   tricks (pop/pop2/fold_try) collapse into one peephole. Cost: one large single-variable refactor (the
   fixpoint proves it lossless: byte-identical output before any optimisation), then each pass small.
   Risk: the refactor itself under a 128-symbol/16 KiB-frame compiler written in itself.
3. Stop codegen at T96 (K1 1.24x crippled twin); spend the next month on the store. D1(a) stays UNMET.
Recommendation: 2, with an explicit byte-identical rung first; if the refactor cannot reach byte-identity
in 3 commits, fall back to 1 with x9-x13 excluded from the temporaries (x1-x7 only) until rehomed.

### D-H · Store spec: three holes before T109 (F35, F37, F38)
Question: accept D10 as written, or amend?
Options:
1. Amend: (i) the writer never executes user code while an RW MAP_SHARED mapping exists — writes go
   through store.bp helpers over a MAP_PRIVATE staging view and are published by pwrite/msync of the dirty
   range (or the RW window is opened and closed inside alloc/commit only); (ii) G2 maps at two bases AND
   contains a deliberate OOB write to show it does NOT reach the file; (iii) packed i32 cells scheduled as
   a type feature with a date, not "later". Cost: spec edit + one extra gate row.
2. Accept LMDB's WRITEMAP trade explicitly: document "a wild write corrupts the store; crc detects on read;
   compaction from the last good generation recovers"; add a G5 variant that injects a wild write.
3. Accept D10 unchanged.
Recommendation: 1(i)-(ii) — a persistent store that a bounds-check-free language can scribble on in
place is the one failure sqlite never has; and D-D option 2 is the long-term fix.

### D-I · Define "language IS the database" as something a skeptic can fail (F39, F40)
Question: what is the claim?
Options:
1. Claim := "a Bebop program's persisted objects are its in-memory objects (same layout, offsets not
   pointers), queries are ordinary compiled fns, and on workload W the system is ≥ a× sqlite (C API,
   ctypes floor subtracted) and ≥ b× LMDB on point lookups, within c× of a native Rust scan, with file
   size and durable-commit rows reported." W = one real dataset (e.g. the dowiz-core order log or an
   OpenStreetMap node extract), not LCG points. a,b,c frozen before the gate runs, by the operator.
2. Keep the T100/G7 rows as the definition; skeptic sees LCG data and self-set thresholds.
3. Drop the slogan; keep "persistent object store with compiled queries" as the feature name.
Recommendation: 1 — the numbers already exist for half of it; the workload and the LMDB/Rust rows are
the missing independence.

### D-J · Park the off-path tasks (F24, F25, F29)
Question: what happens to the ~45 open tasks not on the D8-D10 critical path?
Options:
1. Move them to a PARKED section (titles + one line), no numbers used in commits, revisit only after
   TG-DONE 7-8; cap open non-parked tasks at ~25; rule: a new task needs a falsifier AND a named
   critical-path edge. Cost: 1 hour. Benefit: the ledger measures progress toward the goal, not activity.
2. Delete the resume-driven ones outright (T84, T85, T91-T95, T89, T79, T82, T88); keep the rest.
3. Keep all; rely on ordering text. (Today: 17 added, ~6 closed.)
Recommendation: 1.

### D-K · Cooling period and spike-before-bind for decisions (F26)
Question: may a binding decision be made the same day as its measurements?
Options:
1. Rule: a decision that schedules > 3 tasks or changes TG-DONE binds one session later, after a ≤ 1-day
   spike that names its falsifier (as the T55 spike did for D8). D10 gets its spike now: G2-lite (mmap a
   file at two bases from .bp, 10^5 objects by offsets, reopen in a second process) before T109-T117 bind.
2. No rule; trust the operator's same-day judgement (it was right on D8: the spike preceded it).
Recommendation: 1 — D8 was made the right way; D10 was not.

### D-L · ROADMAP.md as a working document (F27, F28)
Question: split or not?
Options:
1. ROADMAP.md ≤ 300 lines (thesis, TG-DONE table, critical path, open decisions, measured table);
   HISTORY.md (progress log, closed pulls, superseded goal text); TASKS.md (ledger, generated counts);
   a `tools/roadmap_check.sh` that fails on stale md5/gate counts. Cost: half a day of docs.
2. Keep one file; fix the stale blocks by hand each session (has not happened three times).
Recommendation: 1; and commit or revert the T99 working-tree diff at every session end (law).

### D-M · Missing tasks (F41, F34, F22)
Question: which of these become tasks now?
(a) runtime capacity traps (D-D) — now; (b) LANGUAGE.md generated from bpref's grammar + a root README
for a second human — now, 2 hours; (c) trap-code table as a doc + T90 pulled before T48 — now;
(d) packed i32 cells — with T48; (e) .bin version word (footer 16 bytes: entry + magic/version; seed
already tolerates size ≥ 16 — verify) — before the store writes .bin digests; (f) K5 = self-compile
time and K6 = nnidx scan as benchmark rows — now (numbers exist); (g) a no-proot run (any Linux
AArch64 VM/host, or qemu-user) as a second measurement column — when available; (h) A55 cluster as a
worker class for T106 — with T106; (i) arena size/MAP_NORESERVE as a seed v4 or env knob — with the store
(F35); (j) `fn match` reserved-word table — now, small; (k) commit T99 — now.
Options: 1. add (a),(b),(c),(f),(j),(k) now, the rest attached to their host task; 2. add all as tasks;
3. add none (fold into existing).
Recommendation: 1.

## (d) Cheap experiments (< 1 hour each) that kill or confirm the top-5 risks

E1 · Honest twins (risk: D1(a) is a paper target). Write k1h.rs/k3h.rs with a loop-carried nonlinear
recurrence and black_box only on inputs/outputs; assert the LLVM loop is present (objdump); run
bench_pinned R=11. Kill condition for "T101-T105 reach 1.0x": the honest K1 twin is < 1.0 ms.
Prediction: K1 honest ≈ 0.5-0.6 ms (v0 shape) → Bebop 3.0 ms = 5-6x, not 1.24x.

E2 · Widened fuzz (risk: the fuzzer is green over the safe subset). Copy gen.py to the scratchpad; allow
allocation in loops (call() at loop_depth), recursion guard `& 255`, a 2-arg recursive fn, fn names from
{match, if, while, let, zeros}, loop bounds to 2000; run 200 seeds with bench/fuzz/shrink.py --classify.
Kill condition for "0 divergences": any CRASH. Prediction: p6-class CRASH within the first 50 seeds and a
p4-class DIVERGE/BPREF-ERROR on the first keyword-named fn.

E3 · Oracle mutation (risk: tautological folds). For each of 91 gates: `sed` the first `+ 1` → `+ 2` (or
first `<` → `<=`) in the std_tests copy, compile with bebop.bin, compare to the frozen fold; count gates
whose fold did NOT change. Kill condition for "every gate proves its algorithm": > 0 insensitive gates.
Prediction: 3-8 gates (the ones whose fold is a small OK-bit sum) survive a one-token mutation.

E4 · Store physics + safety (risk: D10 has no spike; F37). 40-line .bp: sys_open a file, ftruncate 8 MB,
sys_mmap MAP_SHARED at base A, write 10^5 (offset, value) cells by store-relative offsets, sys_mmap again
at base B, fold through B; then do ONE `a[i]` write with i past the mapped object and check whether the
file changed (cmp before/after). Kill condition for "seed stays frozen and the store is safe": the OOB
write lands in the file. Prediction: it does; the two-base fold works.

E5 · Core count (risk: T106's 4-core numbers). `taskset -c 7 ./seed/build/seed k1.bin` and
`taskset -c 4-7 python3 -c 'import os;print(os.sched_getaffinity(0))'` from this shell, then from the
harness's shell. Kill condition for "4 A78": EPERM/EINVAL on CPU 7 or affinity {4,5,6}. 5 minutes.

Bonus (F20/F19): a 20-line script that fails when census.txt's bebop bcond grows without an ALLOW line
in the commit message — one hour, closes the ratchet.
