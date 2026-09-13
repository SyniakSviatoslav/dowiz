# Phase F Status Report (2026-09-09)

**Source:** ROADMAP.md §Phase F (lines 181-193 on 2026-09-09; **191-203 at `97895d9`**, re-derived 2026-09-13: F0 194, F1 195, F2 196, F3 197, F4 198, F5 199, F6 200, F7 201, F8 202, F9 203 -- every `line NNN` below is the 2026-09-09 number), commit 7de7922, and all F-phase artifacts in `formal/` + `selfhost/tcheck.bp`.

**Note on blueprints:** The F-phase blueprints named in the ROADMAP (`F1-unrepresentable-zero-word.md`, `F2-bounds-by-type.md`, `F3-lean-semantics.md`, `F4-fragment-validation.md`, `F7-dependent-types.md`, `F8-first-theorems.md`) do **not exist** in `docs/blueprints/`. The ROADMAP §Phase F rows ARE the live specification — the named files were either never written, written-out-then-deleted, or superseded by inline spec. This report uses the ROADMAP rows as the primary source, cross-checked against existing `formal/` files and `tcheck.bp`. (Re-derived 2026-09-13: all six EXIST in `docs/blueprints/` at `97895d9` -- `ls docs/blueprints/F*.md`.)

**Key reality check:** The user context says F4 = "IN PROGRESS (7de7922: 68/121 oracle entries, builtins fixed)" but commit 7de7922 is `fix(formal): F4 builtin stubs — str_len/char/scan implementations` which touches Builtins.lean only. The formal/ scaffold (from d46c752 + 7c03111) reports 86/86 construct entries + 121/121 oracle entries in Conformance.lean. The "68/121" figure is unverified and may be stale; this report records the actual file contents.

---

## F0 — Language surface complete and checkable

**Status:** DONE (LANDED 2026-09-09)

**Evidence:**
- `tools/builtin_surface.py`: 36 arms, 36 resolved, 0 unresolved
- `builtins known to all four: 36 of 36` — compiler (36) / typecheck BUILTIN (36, `scan` added) / bpref (34 — `hvham`/`hvham2` have no case) / LANGUAGE.md (31)
- Gate stays STANDALONE (not wired into std_golden): "34 of 36 today"
- Commit 7de7922 fixes builtin stubs (str_len/char/scan) in formal/ Builtins.lean

**Dependencies:** None (precondition for F1-F9)

---

## F1 — Unrepresentable zero-word traps (planning scan)

**Status:** DONE (LANDED 2026-09-09)

**Evidence:**
- `tools/trap_census.py` derives from `docs/WORKER-CARD.md`, `docs/LANGUAGE.md`, `docs/TRAPS.md`, `bebop.bp`, `bench/parity_constructs/neg/`:
  - `trap_unrep 10/29`
  - `trap_zero_word 16 of 19 open`
  - `trap_needs_words 3 of 19`
  - `undef_census 2`
- Two of four numbers verify EXACTLY (16 zero-word, 2 undefined)
- Open = 19 not 20; rows = 29 not 24 (census names the 3 excluded rows as not-language-defects)
- All three word-costing rows are ONE defect: unchecked array bounds
- **Blocking finding for F2:** exit-code space not partitioned. `vs_mask_take/free` and `vs_cs_take/free` (`bebop.bp:2271-2295`) exit `sys_exit(99/100/119/120 + site)` for caller-chosen site in 1..20, so compiler can emit ANY code in 99..140, aliasing every documented diagnostic. 102 and 104 each mean two things. 103/105/106/107 emitted and documented nowhere.
- F2 assigns 105/106/107 to new user diagnostics — all three taken
- Free codes below 99: 65-79, 84, 85, 86, 93 = 19 against F2's 12
- Builtin shadowing probed: T122 mechanism CORRECT, table short by 7 of 36

**Blueprint referenced:** `docs/blueprints/F1-unrepresentable-zero-word.md` — does NOT exist on disk (re-derived 2026-09-13: EXISTS at `97895d9`)

**Dependencies:** F0

---

## F2 — Zero-word trap implementations ( bebop.bp lane)

**Status:** NOT STARTED

**Evidence:** None on disk. The ROADMAP row at line 186 (195 at `97895d9`) describes the plan only:
- 16 zero-word traps to make representable
- definite assignment bitmask over 64-symbol cap (exit 105)
- loop-literal escape via `loop_alloc_safe` (exit 106; verify A6 per-iteration status first)
- `zeros` in `while` body (exit 107)
- > 8 live symbols across `sys_clone` (exit 108)
- true message for nesting cap at exit 95 sites
- `&&`/`||` rejected or short-circuited (exit 112)
- unresolved call via `fntab_lookup` at end of program (exit 115; deletes `brk #87` word)
- literal `sys_clone(_, 0)` (exit 116)
- `use` off column 0 (exit 118)
- positions on every remaining exit-89 site
- one `neg/` construct per code; gen.py stops avoiding each rejected shape
- ~300 lines, ~3 weeks, bebop.bp lane
- Gate: `trap_unrep >= 12/20`; construct parity 86+12/0; fixpoint gen3 == gen4; bin_words delta <= 0; `gen_avoid` shrinks by 12; battery GREEN

**Blueprint referenced:** `docs/blueprints/F1-unrepresentable-zero-word.md` (referenced from F1 row) — does NOT exist (re-derived 2026-09-13: EXISTS at `97895d9`)

**Dependencies:** F1 (F1's exit-code partitioning finding must be resolved first — codes 105/106/107 already assigned)

---

## F3 — [T] with length (A8 amendment, bounds-by-type)

**Status:** NOT STARTED

**Evidence:** None on disk. ROADMAP row at line 187 (F2 196 / F3 197 at `97895d9`) describes the plan only:
- Amends A8's gate, does NOT replace A8
- Arm (i): header cell at `data-1`, check `sub;ldr;cmp;b.hs` (4 words reg + 3 literal) → trap 84 with pc in stub text; store-level `st_len` check in `st_get`/`st_put`; cursor check in `st_alloc`; cell stepping made UNREPRESENTABLE (typecheck finding + rewrite of stepping sites)
- Arm (ii): declared lengths `[i64; k]`/`[i64; n]` on params/locals in A8's tag side channel; discharge literal indices below declared length (87.5% of bebop.bp sites) at zero words; actual length checked once at call site
- Arm (iii): hoisted per-loop check when every index is loop counter (pre-measure with `lin_census.py` variant)
- `/0` trap 109 unless proven (row 10); `checked` dialect bit for overflow trap 111 (row 12, money.bp compiles under it); clone stack trap 117
- ~2 weeks arm (i) + ~3 weeks arms (ii)-(iii), bebop.bp lane
- Gate: `bounds_census: checked+proven+hoisted = 100%`; `bin_words <= 48,700` after (i), `<= 43,400` after (ii), `<= 42,600` after (iii); `K5 <= +15%` after (i), `<= +3%` after (ii); `K6 <= 1.2x` with HOISTED check on sgraph2 frontier loop; fuzz OOB → 100% trap 84, TRAP-82 = 0; `stepping_sites: 0`; `trap_unrep >= 18/20`

**Blueprints referenced:** `docs/blueprints/A8-typed-tables-u32.md` (amended §"length") + `docs/blueprints/F2-bounds-by-type.md` — the F2 file does NOT exist; A8 exists in blueprints (re-derived 2026-09-13: `F2-bounds-by-type.md` EXISTS at `97895d9`)

**Dependencies:** F1; A8 (A8 must land first since F3 amends its gate)

---

## F4 — Lean 4 mechanised semantics (whole language)

**Status:** IN PROGRESS (scaffold complete, builtins being fixed)

**Evidence:**
- `formal/` directory exists with 11 files, ~1,150 lines total Lean 4
- `formal/README.md`: "SCAFFOLD — architecture complete, core types and interpreter implemented, builtins/syscalls/traps/conformance as stubs with `sorry`"
- **Compiles without sorry** -- REFUTED 2026-09-13: nothing under `formal/` had elaborated before `23d2d39` (a `/-` inside a Basic.lean docstring opened a nested comment that swallowed the file); at `97895d9` `lake build` is rc=0, 8/8 modules. The 2026-09-09 list follows, written without a build: Basic.lean, Semantics.lean, Traps.lean, Bebop.lean, lakefile.lean
- **Contains sorry:** Builtins.lean (6), Syscalls.lean (26 axioms), Conformance.lean (2) (re-derived 2026-09-13: `sorry` 0 in code, `axiom` 26 + 7, `theorem` 0 -- formal/README.md, counts by `grep -c`)
- File breakdown from README (the per-file COMPILES marks below are the 2026-09-09 claim; see the refutation above):
  - `Bebop/Basic.lean` ~200 lines: core types Val, Expr, Stmt, Program, State, TrapCode, Result — COMPILES
  - `Bebop/Semantics.lean` ~230 lines (actually 607 on disk): fuel-bounded definitional interpreter evalExpr/execStmt/evalProgram — COMPILES
  - `Bebop/Builtins.lean` ~170 lines (actually 324 on disk): 10 executable builtins — 6 sorry
  - `Bebop/Syscalls.lean` ~150 lines (actually 398 on disk): 26 axiomatised sys_* with declared footprints — all sorry
  - `Bebop/Traps.lean` ~180 lines (actually 193 on disk): 24-row trap table (F1 census), static rejection checker — COMPILES
  - `Bebop/Conformance.lean` ~200 lines (actually 609 on disk): 86-construct harness + 121 oracle interface — 2 sorry
  - `Bebop.lean` ~12 lines: root import — COMPILES
  - `harness.lean` 89 lines: OFF-BOX #eval driver, runs 5 sample constructs (re-derived 2026-09-13: runs ON-BOX with `lean --run harness.lean`; 0/5 PASS because the evaluator has no tail-expression rule)
  - `lakefile.lean` 9 lines: Lake project — COMPILES
  - `lean-toolchain`: pinned v4.12.0

- Builtins.lean actual state (from reading the file):
  - **Implemented (no sorry):** zeros, str_len, char, clock_ms, clz, crc32, crc32x, hvham, hvham2, scan — ALL 10 defined
  - char() implementation is approximate: returns 0 for idx > 0 (no byte buffer in model)
  - scan() implementation is approximate: "stop immediately (conservative: no byte matching possible)"
  - hvham2 = hvham (stub: "same as hvham in formal model")
  - Dispatch table complete for all 10

- Commit 7de7922 (`fix(formal): F4 builtin stubs — str_len/char/scan implementations`): fixes str_len, char, scan implementations in Builtins.lean — these were the "F4 builtin stubs" being fixed. The user's "68/121 oracle entries" figure is not corroborated by file inspection; Conformance.lean declares all 121 oracle entries as a data structure regardless of whether the interpreter can run them (the 2 `sorry` are in `runTestCase` parser and `checkResult`).

- Runs OFF-BOX: Lean 4 cannot run under the box's 3GB/32-process caps — first off-box gate in the tree -- **REFUTED 2026-09-13 (`97895d9`): never measured, and false. `lake build` runs ON-BOX, rc=0, 8/8 modules, 5-7 s and ~500 MB per module, at most 2 `lean` processes (commit message of `97895d9`; formal/README.md §Building)**

**Gate:** `lean_conformance: c/86` (landable at any c; end 86/86); `builtin_spec: b/36` (10 exec + 26 axiom; end 36/36); `bpref_diff: 0` over 86 + 121 + >= 10^4 seeds; `lean_results_hash == sha256(formal/*.lean)` checked on-box

**Dependencies:** F0 (rule list and rejection codes). **PARALLEL to F1/F2** — does NOT depend on them.

**Discrepancy:** User context says "68/121 oracle entries" but Conformance.lean declares all 121 as data; the 2 `sorry` blocks are in the harness driver (parser + result checker), not in the oracle entries themselves. The "68" figure may refer to a previous measurement not reflected in current file state.

---

## F5 — Translation validation from one-pass trace

**Status:** NOT STARTED (re-derived 2026-09-13: the instrument exists, the measurement does not yet -- see Evidence)

**Evidence:** None on disk (REFUTED 2026-09-13: `tools/tv_fragments.py` exists, rewritten in `a4c9301` to fail loudly instead of printing PASS over an empty measurement; it has no rc=0 path until a trace zone exists in `bebop.bin`, and none does). ROADMAP row at line 189 (199 at `97895d9`) describes the plan:
- Emitter writes `(pos, first word, last word, window-before digest, window-after digest, patch list)` per construct into side zone of `.bin` (A10's reloc zone; ~30 emitter sites, ~50-100 lines, 0 code words, fixpoint md5 unchanged by zone)
- Lean semantics of emitted AArch64 subset (inventory = check_words.py's allowlist over 532 emission sites; a few dozen forms)
- Validator symbolically executes each fragment (<= ~20 words; loops as diamonds with register model's "cs mask 0 at loop entry" invariant), normalises, sends residual (`vs_try_ubfx`/`and_imm`/`madd`, csel) to QF_BV with certificates through F5
- Runs as chain gate, not at compile time
- 6-10 weeks: decoder/semantics parallel, trace slice bebop.bp lane after F2
- Gate: `tv_fragments: v/t` over 86 constructs (end t/t), then over self-compile; `trace_zone_words: 0` in code; fixpoint gen3 == gen4 unchanged; `bounds_census` re-derived from trace agrees with F2's static census

**Blueprint referenced:** `docs/blueprints/F4-fragment-validation.md` — does NOT exist on disk (despite ROADMAP saying blueprint=F4-fragment-validation.md) (re-derived 2026-09-13: EXISTS at `97895d9`)

**Dependencies:** F2 (checks change the fragments), F3 (rules); F5 for the residual (self-referential: F5 produces certificates consumed by F5 — this is the loop-closing structure)

---

## F6 — Certificate checker in Bebop (`tcheck.bp`)

**Status:** DONE scaffold (commit 302d120), implementation not yet proven

**Evidence:**
- `selfhost/tcheck.bp` exists: 529 lines, standalone program with own 511-fn cap
- `tools/certcheck.py` exists: 358 lines, Python reference checker (NOT in trust root — executable specification)
- Both implement the certificate format: text-hinted LRAT variant with p/c/r/d/x records
- Certificate format decided 2026-09-09: custom minimal format, NOT LRAT
  - 11.3: our own format
  - 12.1: QF_ABV bit-blasted to SAT (carry makes LIA unsound, arrays eliminated in VC generator)
  - 13.3: our own term format for structural layer
  - 14.3: text normative, binary derived cache
  - 15.1: sha256 of obligation inside each certificate
  - 15.3: Merkle root over commit's obligations
- CaDiCaL 2.1.2 ships in Lean toolchain at `<lean>/bin/cadical`, `--lrat=true --binary=false` emits text-hinted LRAT — reachable in-tree
- `tools/certgen.py` (UNTRUSTED, deletable when bebop.bp produces format): Tseitin bit-blast to cadical
- Trust root: `seed/seed.S` (1,480 B), `bebop.bin` (171,320 B), `tcheck.bp` (future)
- Lean demoted from producer to cross-check oracle: "Bebop produces the proof, Lean independently confirms the theorem"

**tcheck.bp capabilities (from reading the file):**
- VC reader: parses p/c/r/d/x records from text certificate
- Bit-blaster: not inside tcheck.bp — the bit-blasted QF_ABV obligations come from outside (certgen.py or future bebop.bp producer)
- Tseitin: not inside — obligations arrive pre-Tseitinned
- Resolution checker: YES — `check_chain` implements hinted resolution as a loop with NO unit propagation, NO watched literals, NO search
- 15.1 binding: `bind_oid` computes sha256 of canonical obligation text, compares to certificate's x line
- 15.3 Merkle root: `certcheck.py` has `--verify-root` and `--merkle`; tcheck.bp has `st` array for counting but Merkle root construction is in certcheck.py only (tcheck.bp reads one cert at a time)
- Line reader, tokenizer, integer parser, clause DB (slot-layered flat arrays), assignment (direct-mapped), canonical text computation — all implemented

**Gate:** Certificate checker is ONE program compiled by ONE compiler; hinted resolution is a loop with no unit propagation, no watched literals, no search — `tcheck.bp` can be tens of functions

**Dependencies:** F5 (produces the certificates tcheck.bp checks). The format is executable TODAY via certcheck.py; tcheck.bp is the trusted Bebop implementation.

---

## F7 — Kernel module of tcheck.bp (structural layer)

**Status:** NOT STARTED (superseded: ROADMAP row F7, line 201 at `97895d9`, LANDED 2026-09-12)

**Evidence:** None on disk (REFUTED 2026-09-13: `selfhost/tcheck_kernel.bp`, `selfhost/tkernel.bp`, `tools/kcheck.py` and `bench/kernel_neg/*.core` exist -- ROADMAP row F7). ROADMAP row at line 191 (201 at `97895d9`) describes the plan:
- Checker for minimal dependent calculus: terms, substitution, whnf, conversion, fixed universe hierarchy, inductives with eliminators
- Bit-level obligations of F6 admitted as certified oracle step
- MINIMAL: research costed Lean-subset kernel at 150-400 fns against bebop.bp's 287 and 511 cap
- Single most expensive item in Phase F — lives in tcheck.bp with its own cap, NOT inside compiler
- **UNIVERSE POLYMORPHISM AND IMPREDICATIVE Prop BOTH REQUIRED** (operator 2026-09-09, binding)
  - Universe polymorphism: quantify statement over levels; ~8 kernel fns of ~170
  - Impredicative Prop: `imax` rule — `pi A B` in Prop whenever B : Prop regardless of A's level
  - Both are SEPARATE losses a concrete-level kernel would take
- Maximum power at maximum risk: impredicative Prop is what Girard's and Hurkens' paradoxes threaten; kernel must get positivity and level rules exactly right or proves False
- `bench/kernel_neg`'s discriminator load-bearing (not decorative)
- `n02` separates the two rules: `Pi (X : Sort 0). X` claimed at Sort 0 rejected predicatively (`max(1,0)=1`), accepted under imax
- Self-verification is a TARGET, not impossibility

**Dependencies:** F6 (kernel re-checks terms F6's format carries). Most expensive item in Phase F.

---

## F8 — Dependent types directly in Bebop

**Status:** NOT STARTED (superseded: ROADMAP row F8, line 202 at `97895d9` -- step 0 landed 2026-09-13: exit 110, `tools/f8_dt.py`, constructs c143/c144)

**Evidence:** None on disk (re-derived 2026-09-13: see the Status clause). ROADMAP row at line 192 (202 at `97895d9`) describes the plan:
- (a) bebop.bp parses `[T; n]`, `{x : T | p}`, `requires`/`ensures`/`invariant`, `theorem` as INERT syntax erased before emission; emits VCs into F4's side zone (~2-3 weeks, bebop.bp lane)
- (b) UNTRUSTED elaborator — standalone `elab.bp` or Lean itself elaborating Bebop annotations to exported terms (cheaper path) — produces terms F6's kernel re-checks (8-16 weeks, parallel)
- (c) checker renames every `let` (Bebop `let` REBINDS fn-scoped register, LANGUAGE.md:41-43); requires loop `invariant` for every rebound variable that a type mentions — Verus/Dafny shape, unavoidable over mutable state
- Full dependent surface types (`Fin n`, length-indexed tensors for B7) after (c)
- Gate: `dt_fns: n/833` selfhost fns with checked types (first: fp.bp 2/2, money.bp 17/17, store.bp 52/52); `elab_neg: 0 accepted of N` ill-typed programs; `K5_typed <= 1.5x K5` (annotation parsing + erasure only; checking is chain gate); fixpoint unchanged; every rejected program carries `<line>:<col>`

**Blueprints referenced:** `docs/blueprints/F7-dependent-types.md` — does NOT exist on disk (re-derived 2026-09-13: EXISTS at `97895d9`)

**Dependencies:** F6; F2 arm (ii) (declared lengths are the first dependent types)

---

## F9 — First theorems, machine-checked and certificate-checked

**Status:** NOT STARTED

**Evidence:** None on disk (re-derived 2026-09-13: `formal/Bebop/Theorems.lean` states the F9 targets as 7 `axiom`s and proves nothing -- 0 `theorem`). ROADMAP row at line 193 (203 at `97895d9`) describes the plan:
- First theorems:
  - `fp_mul(a, b) = sign * floor(|a| * |b| / 2^32)` (tested on 1.2M values, fp.bp comment)
  - `isqrt(s)^2 <= s < (isqrt(s)+1)^2`
  - B8 money laws of money.bp against Rust oracle in `bench/oracles/rust` for all inputs in declared range (eqc_gen exact-equality discipline lifted to quantifier)
  - Store invariants: `st_len`, cursor monotone, crc
- 3-5 weeks after F5-F7
- Gate: `theorems: >= 3` then growing, each with kernel-checked term and LRAT certificate; `theorem-sample.bp`/`theorem-false.bp` retired or re-homed on F6 (their C-era kernel is a stub per bench/VERIFICATION.md)

**Blueprints referenced:** `docs/blueprints/F8-first-theorems.md` — does NOT exist on disk (re-derived 2026-09-13: EXISTS at `97895d9`)

**Dependencies:** F7 (needs kernel to check terms); F5-F7 all need to exist before F9 can produce theorems

---

## Dependency graph (F2-F9)

```
F0 (DONE) ──→ F1 (DONE) ──→ F2 (NOT STARTED)
                │                      │
                │                      ├────────→ F5 (NOT STARTED) ──→ F6 (DONE scaffold)
                │                      │                                     │
                └──────────────────────┼─────────────────────────────────────┤
                                       │                                     │
F4 (IN PROGRESS, parallel) ───────────┘                                     │
                                                                          │
A8 (done, external) ──→ F3 (NOT STARTED) ───────────────────────────────┘
                                                                      │
F6 ──→ F7 (NOT STARTED) ──→ F8 (NOT STARTED) ──→ F9 (NOT STARTED)
         │                      ↑
         └──────────────────────┘
         (F8 also needs F2 arm (ii))
```

**Phase F runs BEFORE B5 and B7** (per ROADMAP line 181; 191 at `97895d9`).

---

## Missing blueprints summary

The ROADMAP §Phase F names these blueprint files that do NOT exist in `docs/blueprints/` (re-derived 2026-09-13: ALL SIX EXIST at `97895d9`; the `Exists?` column is the 2026-09-09 answer, the current one is appended):

| ROADMAP row | Blueprint name in ROADMAP | Exists? |
|---|---|---|
| F1 (line 186; 195 at `97895d9`) | `docs/blueprints/F1-unrepresentable-zero-word.md` | NO on 2026-09-09; YES 2026-09-13 |
| F2/F3 (line 187; F3 197 at `97895d9`) | `docs/blueprints/F2-bounds-by-type.md` | NO on 2026-09-09; YES 2026-09-13 |
| F4 (line 188; 198 at `97895d9`) | `docs/blueprints/F3-lean-semantics.md` | NO on 2026-09-09; YES 2026-09-13 |
| F5 (line 189; 199 at `97895d9`) | `docs/blueprints/F4-fragment-validation.md` | NO on 2026-09-09; YES 2026-09-13 |
| F8 (line 192; 202 at `97895d9`) | `docs/blueprints/F7-dependent-types.md` | NO on 2026-09-09; YES 2026-09-13 |
| F9 (line 193; 203 at `97895d9`) | `docs/blueprints/F8-first-theorems.md` | NO on 2026-09-09; YES 2026-09-13 |

**F3 row at line 187 (197 at `97895d9`) ALSO references `docs/blueprints/A8-typed-tables-u32.md`** which DOES exist (it's an A-phase blueprint).

**Non-F blueprints found in docs/blueprints/ (38 total):** A1-A15, B1-B8, C1-C6, D1-D5, D1-D5b, E1-E4, plus attic/ patches and PARALLEL-LANES. None are F-phase.

The F-phase specification lives entirely in ROADMAP.md §Phase F rows (lines 181-193; 191-203 at `97895d9`) plus `formal/` + `selfhost/tcheck.bp` + `tools/certcheck.py`.

---

## Next priority recommendation

**F2 is the next priority**, for three reasons:

1. **F1 is DONE and F2 depends on it.** F1's exit-code partitioning finding (codes 105/106/107 now assigned, 103/105/106/107 were undocumented) is the gating prerequisite. F2 implements the 16 zero-word trap representations F1 identified.

2. **F2 unblocks F5.** F5's translation validator needs F2's static checks in place because "checks change the fragments" — the bounds checks F2 adds change what the emitted AArch64 fragments look like, and F5 validates those fragments. F5 also needs F2's `bounds_census` as a cross-check against its trace-derived census.

3. **F4 is parallel and can continue independently.** F4 (Lean semantics) does NOT depend on F1/F2 — it only needs F0's rule list and rejection codes, which are DONE. The builtins being fixed in commit 7de7922 are a separate track. F4 can progress in parallel while F2 executes on the bebop.bp lane.

**F3 should follow F2** because F3 amends A8's gate (A8 is done) but also depends on F1's exit-code partitioning (F3 assigns trap 84, 109, 111, 117).

**F6 scaffold is DONE (commit 302d120)** and the format is executable today via `tools/certcheck.py`. F6's remaining work is proving that `tcheck.bp` (the Bebop implementation) agrees with `certcheck.py` (the Python spec) — this can proceed in parallel once F5 starts producing certificates.

**F7 is the critical-path risk.** It's costed as the single most expensive item in Phase F (150-400 fns against 511 cap), requires universe polymorphism AND impredicative Prop, and carries named paradox risk (Girard/Hurkens). It should not be started until F6's format is stable, because F7's kernel re-checks terms in F6's format.

---

## Verdict

**READY** — analysis complete. All evidence gathered from:
- ROADMAP.md §Phase F (lines 181-193; 191-203 at `97895d9`), commit 7de7922
- `formal/` directory: 11 files, ~1,150 lines Lean 4 (README, 6 Lean modules, harness, lakefile, toolchain)
- `selfhost/tcheck.bp`: 529-line standalone certificate checker
- `tools/certcheck.py`: 358-line Python reference checker
- `docs/blueprints/`: 38 files, none F-phase (F-blueprints named in ROADMAP do not exist on disk)
- Git log: 30 recent commits, confirming F4 scaffold (d46c752), F6 scaffold (302d120), F4 builtin fixes (7de7922)
