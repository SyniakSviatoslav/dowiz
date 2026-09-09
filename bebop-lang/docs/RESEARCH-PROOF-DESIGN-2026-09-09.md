Status: 2026-09-09 -- read-only design analysis by a research agent (Fable) over /root/dowiz/bebop-lang at today's tree (bebop.bp 7,474 lines / 291 fns; bebop.bin 171,320 bytes; seed 1,480 bytes; 76 positive + 13 `neg/` constructs = 89 files; selfhost 837 fns), written against the operator's decisions as they stood at the END of the day: full CIC (dependent types over an infinite universe hierarchy with higher-order abstraction), closures/generics/HOFs required (ROADMAP A16), ONE language with elaboration as a compiler PHASE inside `bebop.bp`, our own certificate and term formats (11.3, 13.3), QF_ABV bit-blasted to SAT (12.1), text normative + binary cache (14.3), sha256 per obligation + Merkle root per commit (15.1, 15.3), Lean 4 a cross-check oracle only. **Every repository number below was counted today with grep/wc/python3 and the file:line is given; every literature figure is (cited) where a venue is named from knowledge or (from memory) where a size or date is recalled without a checkable source -- no paper, manual or kernel source was fetched; every cost is an ESTIMATE anchored to a count.** Nothing was compiled, run, benchmarked or written inside the repository. Where the brief's own numbers are stale against the tree, section 1 says so. Where the honest answer is that a part of the thesis is not reachable on the horizon, section 13 says so with the argument.

# A CIC-class kernel in Bebop that is small because it is not Lean: the calculus to CHECK, the cap arithmetic for one language, the certificate format, the bootstrap, and the cost

## 0. The finding

1. **The one-language decision puts the elaborator inside `bebop.bp`, and the 512 cap then binds -- but the cheap answer is to raise it, not to restructure.** A minimal dependent elaborator is 115-170 fns (estimate, section 5.2): 291 + 115..170 = **406-461**, under 512 by 51-106 with the compiler still growing 1-3 fns per row. The raise to 1024 costs **~30 edit sites and, by the arithmetic of section 2.2, ~0 `bin_words`**: append a second 512-entry window list ABOVE `fntab[5000]` (the highest zone constant today; `fntab = zeros(8192)` has 3,191 free cells) so that the 272 sites at 35 distinct constants above the current window list do not move; only the 11 window-list sites, the 17 `zeros(512)` arrays and the `>= 512` check at `bebop.bp:2703` change. A1b did the same job once (256 -> 512, ROADMAP:87) as one fixpoint commit.
2. **Elaboration has an in-tree precedent for its SHAPE: `use` expansion (T47).** `use_expand` (`bebop.bp:6679`) is a whole-program textual pass inside the compiler that emits no machine word, writes `<out>.use` and hands the expanded text to the unchanged one-pass emitter (`:6510`). Monomorphisation and defunctionalisation are the same shape: they append generated fn TEXT (specialisations, lifted lambdas, one `apply` per arrow type) to that buffer. The emitter stays one-pass and IR-free. **But the 2026-09-09 thesis amendment does not cover it as worded**: it permits a second pass whose output is "a rejection with a position or a certificate"; elaboration also outputs generated source, which is expressiveness. The amendment must be WIDENED (section 2.4 has the sentence), not stretched.
3. **Closures pack into one i64 and the pointer-free model survives: 9 bits of fn index (cap 512) + 29 bits of environment cell index (A5's 2^29-cell reserve, blueprint A5 §"Address space") = 38 of 64 bits.** Application needs no indirect call: the emitter has **0 `blr` sites** (`grep -c` today; the only mention is a comment at `bebop.bp:7378` about `sys_run`), so `apply_T` is a generated fn with an `if` chain over that arrow type's lambdas -- ~3 words per arm plus two `bl`. The collision is not with the register model but with the arena: an ESCAPING closure's environment is a `zeros` object that is never freed, and a closure created in a `while` body is exactly trap-census row 5 (`zeros` in a loop). Section 3.
4. **The kernel at full CIC strength is 135-200 fns / ~3,500-5,100 lines in a standalone `tcheck.bp` (estimate, section 5.1) -- under its own 512 cap by 2.5x -- because seven things Lean's kernel does are Lean CONVENTIONS the operator's wording does not require, and each is dropped:** level variables and `imax` (levels are concrete naturals; the elaborator instantiates), impredicative `Prop` and definitional proof irrelevance, quotients, nested and mutual inductives (the elaborator encodes them into single indexed families), K-like reduction and structure eta, and -- the largest -- **definitional-equality SEARCH**: the kernel does no unfolding on its own; every conversion is an explicit step or a `reduce k` with the fuel k IN the certificate, so checking is linear in the certificate and `maxHeartbeats` cannot exist. Section 4 has the required/conventional verdict per feature.
5. **The i64 machine word is the kernel's primitive number type, and that is the gift, not a limitation.** Lean special-cases `Nat` literals with GMP inside the kernel for speed -- a trusted extension. Here `I64 : Type 0` with the ~20 machine operators as primitive constants whose evaluation rule IS the instruction the hardware executes (already trusted via the seed), and the SAME operator table defines the bit-blaster (12.1). No bignum, no encoding gap, and the statement discipline `money.bp` already uses (explicit `addov`/`subov`/`mulov`, `:28-41`) is the statement discipline of the logic.
6. **The three flagship theorems (F9's `fp_mul`, `isqrt`, and `money.bp`'s `mulov`) are NOT safe SAT problems.** Each contains a 64x64 multiplication or a 64-bit division compared against a different arithmetic decomposition of the same value; multiplier-equivalence miters are the known-hard case for CDCL/resolution (Kaufmann, Biere, Kauers, FMCAD 2019 -- from memory on the bit-width where SAT alone stops, ~16-32). The bounds, overflow, FSM and store obligations (adders, comparators, constant divisions) are the easy case and 12.1 is right for them. The design therefore needs ONE word-level rule in the kernel -- polynomial normal form over Z/2^64 (`ring`, ~8-12 fns) -- plus two floor-division lemmas, and the cheapest pre-measurement is Lean's `bv_decide` on the `fp_mul` miter (it is the cross-check oracle anyway; lane C measured `bv_decide` at +7 processes, so it is a slot-class batch job). Section 10.
7. **The extraction gap is the single clearest advantage and it is quantifiable: Lean's gap is ~10^6 lines of unverified pipeline (Lean compiler -> C -> clang -> runtime + GMP, from memory on the orders of magnitude); Bebop's is two in-tree phases under 8,000 lines with a per-build certificate.** The proved artifact is the elaborated term; the running artifact is ~42,000 words produced by a 7,474-line one-pass emitter from fragments of <= ~20 words at 532 emission sites; F5 validates every fragment with QF_BV obligations through the same checker. What remains open is the lowering phase (defunctionalisation + monomorphisation, est. 30-45 fns): trusted today, validated per program later by a logical-relation certificate the elaborator can emit mechanically (section 3.5). Not zero; two orders of magnitude smaller and checkable.
8. **Theorem number one needs NO elaborator.** With the SAT oracle a QF_BV theorem's proof term is one constant application, and with fuelled reduction a ground fact's proof is `refl k`. So `kernel_checked >= 1` is reachable from `tcheck.bp` alone plus a hand-written statement in the text format and a certificate from an untrusted producer (`tools/prove.py`, a ~300-line CDCL with hint logging -- Python is already a dev tool via bpref). Candidates in order: `ordfsm.decide` on the 12x12 matrix (`refl` with fuel; no producer at all), then `addov` (`money.bp:28`) by the oracle. The serial elaborator work in `bebop.bp` is NOT on the path to theorem one. Section 9.
9. **A half-built CIC kernel has no number until `infer` + `reduce` + inductives + conversion all exist, and that is unlike every other row in the tree -- said here as the second independent source.** The mitigation is to build the refutation instrument FIRST: a Python twin `tools/kcheck.py` (the kernel's bpref, ~1,000 lines, days) and a negative corpus written before the kernel -- `Type : Type` (Girard), Hurkens' paradox shape, a non-positive inductive, an ill-typed application, a level overflow -- so `kernel_neg: 0 accepted of N` and `kernel_parity: n/N` are numbers from the first `infer` commit. Section 9.4.
10. **Metamath Zero is the artifact to copy for the FORMAT and the verifier discipline; Milawa for the BOOTSTRAP; HOL Light for the size benchmark of the logical core.** MM0's split -- a human-readable statement file that is the trusted claim and a binary proof stream verified by a ~1-2k-line C verifier with a stack machine (from memory) -- is 14.3 and 15.1 already; its logic (first-order) cannot be copied under the CIC decision. Milawa's ladder (a small level-1 checker, higher levels proven in the level below, the whole re-checked from level 1, on a verified Lisp runtime -- Davis 2009; Myreen & Davis ITP 2011, cited) is the shape of `tcheck.bp` -> `prove.bp` -> (later) admitting `prove.bp`. HOL Light's ~700-line kernel (from memory) is beaten by the RULE core of `tcheck.bp` (35-45 fns) and exceeded by the whole because a CIC kernel must generate recursors and this one carries a bit-blaster; both excesses are named and priced. Section 6.
11. **Lean's holes, each with the mechanism that makes it impossible here rather than discouraged** (section 7): no `native_decide` because the kernel's only computation rule is its own fuelled reducer and `tcheck.bp` has no `sys_run`/`sys_clone`/`sys_mmap` (a grep gate); no `implemented_by` because a Bebop fn's definition is read from its BODY by the kernel's own reader, never from a claimed replacement; no `axiom` keyword in the certificate grammar (a fixed axiom table -- the 26 syscall footprints -- compiled into the checker); no `#print axioms` because every certificate carries a `deps` line the checker RECOMPUTES and compares, and the Merkle root covers it; no `maxHeartbeats` because fuel is per step in the certificate; no universe-constraint solver in the trusted base because levels are concrete.
12. **Cost in the project's units: ~95-100 roadmap steps, of which 35 are SERIAL in `bebop.bp` (the elaborator) and ~60 are lane-parallel new files (`tcheck.bp` 35, `prove.bp` 10, Lean cross-check 3, first theorems 9-13, docs 2).** At the measured ~1 step/h that is ~95-100 h of continuous operation and ~160-170 commits at 1.7/h; wall ~45-50 h with a kernel lane and a compiler lane staffed, ~95 h single-lane. The slot is not the constraint (~100 landings x 3.5 min = ~6 h). The honest caveat: the 1 step/h velocity was measured on rows where every step had a number; the kernel's first four modules have none until the twin exists, so expect 1-3 h/step there -- which is why finding 9 puts the twin first. Section 12.
13. **Not reachable on the horizon, stated once:** the checker's own correctness proved in its own logic (MM0's and Milawa's end state; multi-year for both, from memory); the compiler's correctness as a theorem (Piton-class, years -- F5's per-build validation is the substitute); any theorem across a `sys_clone`, a crash, or a syscall beyond its declared footprint; and function extensionality or classical reasoning, which the predicative, axiom-free kernel does not provide and which nothing in the named claims needs. Section 13.

---

## 1. What was counted today, and where the brief is stale

| object | measured today | brief / prior document | how |
|---|---|---|---|
| `bebop.bp` | **7,474 lines, 291 top-level fns**, 25.68 lines/fn, median fn 19 lines, p90 47, max 204; prefixes `emit_*` 103, `vs_*` 70 | brief: 287 fns; TRUST-CHAIN §1: 7,356 lines / 287 | `grep -c '^fn '`, python |
| fn cap | `fntab[4547] >= 512` -> exit 89 at `bebop.bp:2703`; TRAPS.md:29 "cap 512"; 17 `zeros(512)` tables | brief: 511 | grep; whether index 0 is usable was not verified |
| `fntab` layout | `zeros(8192)` (4 sites); 326 literal `fntab[k]` sites at 44 distinct k, max 5000; window list `fntab[2800..4335]` (512 x 3 cells, 11 sites); **272 sites at 35 distinct constants >= 4336** | -- | python over the source |
| `bebop.bin` | **171,320 bytes** | TRUST-CHAIN §1: 170,312 (2026-09-09 morning) | `wc -c` |
| `bin_words` | not measured (needs a compile); `docs/PERF.md:11` last row 41,161 -> 41,217; the 09-09 report used 42,078; the coordinator's message 42,330 | three inherited numbers, none mine | read only |
| constructs | **76 positive + 13 `neg/` = 89 files**; `construct_parity.sh` has 92 `EXPECT=` lines | brief: 86 | `ls`, grep |
| `docs/LANGUAGE.md` | 133 lines; `:132` lists closures and generics as NOT in the language (now a defect per A16) | brief: 130 | wc |
| selfhost | 837 fns across `std/` + `prelude/` | prior report: 833 | grep |
| theorem targets | `fp.bp` 44 lines / 2 fns / 1 loop; `money.bp` 261 / 17 / 1 loop; `ordfsm.bp` 327 / 15 / 24 loops; `store.bp` 583 / 52 / 20 loops; `sha256.bp` 133 / 5 / 6 loops | -- | wc, grep `while ` (each loop = one invariant an author must write) |
| existing SAT solver | `dpll.bp` 106 lines / 3 fns, <= 16 vars, bitmask clauses, 2,000-node budget | -- | read |
| indirect calls in the emitter | **0** `blr` emission sites (one comment at `:7378`) | -- | grep |
| `use` expansion | `use_path_hash/append_str/comment/scan/expand`, `bebop.bp:6513-6679`; output re-slurped at `:6510` | -- | grep |
| box | Lean 4.33.1 runs: 3.0 GB, cold 20.3 s, warm 0.4 s, ~735 MB; `bv_decide` +7 processes (lane C, today) | -- | reported, not re-measured |
| solvers on box | cadical/kissat/minisat/z3/cvc5/lean: none on PATH; python3, as, ld present | -- | `command -v` |

---

## 2. One language: where elaboration lives, the cap, and the amendment

### 2.1 The distinction the operator drew, made operational

A separate PROGRAM is a `.bp` file compiled by `bebop.bin` whose inputs and outputs are files and whose existence adds no grammar rule: `gb_run.bp`, the D5b witness, and `tcheck.bp`. A separate LANGUAGE is anything that would need its own line in `docs/LANGUAGE.md`. The test: **if deleting the program leaves every `.bp` in the tree meaning the same thing, it is a program.** Deleting `tcheck.bp` leaves the meaning of every program intact (fewer things are PROVED); deleting an elaborator would change what `fn map<T>(...)` means. So elaboration is a phase of `bebop.bp`, and `tcheck.bp`, `prove.bp` and the Lean cross-check tooling are programs.

### 2.2 The cap arithmetic

`bebop.bp` at 291 + a minimal elaborator at 115-170 fns (section 5.2) = **406-461**. The compiler has gained ~40 fns since A1b's "~250" (ROADMAP:87) in three days; F2's diagnostics, F3's bounds forms and A16's constructs all add fns. 51-106 of headroom is one lane-week of growth. The raise should be done at the START of the elaborator work, as one fixpoint commit, the A1b way.

Two ways to raise to 1024, priced against the counts:

| way | sites that change | `bin_words` (estimate) | why |
|---|---|---|---|
| shift: grow the window list in place (`2800..4335` -> `2800..5871`) | 11 window-list sites + **272 sites at 35 constants >= 4336** + 17 `zeros(512)` + the check | ~0 (indices already exceed 4095 today, so no immediate-class changes) | every zone above the list moves by 1,536 |
| **append**: a second 512-entry list at `5001..6536` selected by `k >= 512` | 11 window-list sites (each gains a select) + 17 `zeros(512)` -> 1024 + the check at `:2703` = **~30 sites** | ~+20-40 words for the selects in the compiler's own binary; 0 for compiled programs (their `bl`s are direct; no runtime fn table) | `zeros(8192)` has 3,191 cells free above 5000 |

Recommend append. Runtime memory: 17 x 512 extra cells = 8,704 cells (~70 KB of arena) per compile -- nothing against 256 MB. K5 effect: nil in principle (tables are `zeros`, never scanned by size). The gate is A1b's: fixpoint md5 unchanged in shape (gen3 == gen4), a 600-fn synthetic program compiles, exit 89 above 1024.

The alternative -- restructuring elaboration to fit under 512 by moving parts to programs -- is available for exactly one part: **proof AUTOMATION** (rewriting, induction search, CDCL) is a producer and belongs in `prove.bp` regardless (section 5.3). Type inference, unification, level solving, monomorphisation and defunctionalisation define what a program MEANS and cannot leave the compiler without becoming a second language. So the split is fixed by the operator's own criterion, and the cap must rise.

### 2.3 Elaboration as a T47-shaped phase

`bebop.bp` has no AST: the emitter is text-directed and its "internal representation" is the per-fn fact word, the symbol table and the register window (the 09-09 report §4: "one pass in this tree has always meant the EMITTER has no IR, not that the source is read once"). A dependent elaborator needs terms. The phase therefore adds a term store (hash-consed `(tag, a, b, c)` cells in a `zeros` arena) and a second reader from the grammar into terms, checks and lowers, and then hands the emitter TEXT: the original fns with type annotations erased plus generated fns (specialisations, lifted lambdas, `apply_T`s) appended to the `.use`-style buffer. The emitter compiles that text unchanged. Positions: the emitter's `<line>:<col>` will point into the expanded buffer, exactly as it does for `.use` today; since the elaborator type-checks first, a Core-level rejection of its own output is an elaborator BUG and gets a gate: `core_reject: 0` over the corpus.

### 2.4 The amendment, as worded, does not cover this -- widen it

The 2026-09-09 amendment (09-09 report §4): a second pass is justified "where its output is a REJECTION with a position or a CERTIFICATE; it is not justified where its only output is a faster or smaller binary". Elaboration's outputs are (a) rejections with positions -- covered; (b) a proof term -- a certificate, covered; (c) generated source for monomorphisation and defunctionalisation -- NOT covered: its purpose is expressiveness, not impossibility of error. Proposed wording, to be added rather than read into the old sentence: *"or a source-to-source expansion that the emitter compiles unchanged, whose correspondence to the source is stated as obligations the kernel checks (the `use` precedent, T47)"*. Under that sentence the lowering phase is admitted on the same terms as `use`: a textual pass, zero words, with its correctness a checkable claim rather than an assumption (section 3.5).

---

## 3. Closures, generics and HOFs compiled away -- the compiled artifact stays first-order

Re-scoped per A16: this no longer shrinks the LOGIC (the kernel is CIC and carries lambdas natively); it keeps the MACHINE WORDS first-order and monomorphic, which is what F5's translation validation reasons about, and it keeps A5's pointer-free model.

### 3.1 The packed closure, checked against the tree

A closure value = one i64: `fn_index` in the low 9 bits (cap 512 today, 10 bits after the raise) and `env` (a cell index into the 2^29-cell reserve, A5 blueprint §"Address space": "R = 4 GiB = 2^29 cells") in bits 9..38 (or 10..39). 38-40 bits of 64. Since A5 step 1b every `[i64]` value is already an x17-relative cell index, the environment is an ordinary arena object and `env[i]` is the existing `ldr [x17, xt, lsl #3]` form. **Holds.** Recursive closures (a lambda referring to itself) put their own packed word in their environment -- fine. The fallback reserve of 1 GiB (2^27 cells) shrinks the env field to 27 bits; still fine.

### 3.2 Application without an indirect call

Reynolds' defunctionalisation (1972, cited) is total for whole programs: every lambda of arrow type T gets a tag; `apply_T(c, args...)` dispatches on `c & 511` to the lambda bodies with `env = c >> 9`. Bebop's `match` is compile-time only (LANGUAGE.md:80-81, the scrutinee must be a literal ctor), so the dispatch is an `if` chain: ~3 words per arm (compare, branch, `bl`) -- O(k) per call where k = lambdas of that type in the WHOLE program, typically a handful per type. A computed function argument (A16's tension iv) is just a tag flowing as data; the logic reasons about `apply_T` as an ordinary first-order fn with an `if` chain. Bebop compiles whole programs already (`use` inclusion), so the whole-program requirement adds nothing new. What is genuinely absent: separate compilation of a HOF against unknown lambdas -- not a Bebop feature today either.

### 3.3 The arena collision, named before it is discovered

An escaping closure's environment must outlive the creating call, so it is a `zeros` object in the monotone arena (never freed, LANGUAGE.md "Memory model"). A closure created inside a `while` body is trap-census row 5 (`zeros` inside a loop: monotone arena growth, trap 80 far from the cause) -- a whole class of ordinary functional code (`map` with a closure over the loop variable) lands on it. Mitigations, in order: non-escaping closures on the frame heap (x14, 16 KiB, dies at return; `loop_alloc_safe` already computes escape for array literals); an elaborator DIAGNOSTIC for an escaping closure allocated in a loop (the agentic rule: a rejection with a position beats a silent leak); A6 step 2's mark/reset on the back-edge for the non-escaping case. There is no garbage collector and the thesis does not want one.

### 3.4 Monomorphisation against the cap and `bin_words`

Every generic instantiation is a new Core fn; every lambda is a lifted fn; every arrow type used as a value is one `apply_T`. Per PROGRAM (the cap is per compilation unit): the compiler's own 291 leaves 221 today (733 after the raise) for generated fns. `bin_words` grows by the generated bodies plus ~3 words per `apply` arm; no number is possible before a construct exists -- the A16 gate ("the word cost stated against 42,330") is the right one and should be a `word_budget.txt` line per closure construct. Polymorphic recursion (an instantiation that demands another at a larger type) makes monomorphisation diverge; Rust rejects it and so should the elaborator (exit code with position; one `neg/` construct).

### 3.5 What the lowering link costs in trust, now and later

The proof term is about the ELABORATED term (with lambdas); the emitter compiles the LOWERED text (with tags). The relation is a logical relation (`[[packed word]] = lambda-term`), not a definitional equality -- so it is not a `refl` step, and until a per-program certificate exists the lowering phase (est. 30-45 fns, section 5.2) is a TRUSTED item, listed in section 11. The elaborator can emit the certificate mechanically (one lemma per lambda: `apply_T(tag_k, env, x) = body_k[env]`, each a few unfolding steps, plus one simulation lemma per HOF call); that is a later step, priced in section 12 as part of F5's remit. Alternatively -- and this is what F9 should do -- state theorems about the LOWERED fns directly: `money.bp`, `ordfsm.bp`, `fp.bp` and `store.bp` contain no closures, so for them the two coincide and the link costs nothing.

---

## 4. The calculus at full CIC strength, chosen to minimise what is CHECKED

The rule applied: a feature is kept if the operator's wording ("dependent types over an infinite universe hierarchy with higher-order abstraction; closures, generics, HOFs") requires it or if the named claims need it; a feature that is merely how Lean does it is dropped, and the drop is priced.

| feature | required by the wording? | Lean's mechanism and its cost | decision here | kernel cost (fns, est.) |
|---|---|---|---|---|
| infinite universe hierarchy | YES | level TERMS `0, succ, max, imax, u` with normalisation and a `<=` decision procedure, universe-polymorphic constants instantiated at use (`level.cpp`, from memory ~500 lines C++); the `imax` rules exist only because of `Prop` | **concrete natural-number levels only.** `Sort n`, `Pi` at `max(i, j)` computed on integers; universe-POLYMORPHIC surface definitions are instantiated by the elaborator at each concrete level used -- "level monomorphisation", the same move as generics. The hierarchy is infinite (any n); nothing in the trusted base solves a constraint | 1-2 |
| cumulativity (`Type i <= Type j`) | no | Coq has it (subtyping in conversion); Lean does not (explicit `ULift`) -- a place Lean is simpler | **none** (Lean's choice); the elaborator inserts lifts | 0 |
| higher-order abstraction, Pi/lambda/app | YES | de Bruijn with locally-nameless + caches | de Bruijn indices; alpha-equivalence is syntactic equality | in infer/subst |
| impredicative `Prop`, definitional proof irrelevance | **NO** -- not in the wording | `Prop` at the bottom, proofs definitionally equal, elimination RESTRICTIONS (which `Prop` inductives may eliminate into `Type`: subsingleton/K rules) -- the most subtle part of `type_checker.cpp` and the home of the `Acc.rec` reduction troubles (from memory) | **dropped.** A predicative hierarchy with proofs in `Type 0`; large elimination is then the unrestricted DEFAULT and costs nothing; no proof irrelevance code. Consequence to state: no impredicative quantification, no `propext`, no `Classical.choice` -- none of the named claims uses any (all are decidable statements over i64) | -(30-50) relative to Lean |
| quotients as a primitive | no | `Quot`, `Quot.mk/lift/ind` + `Quot.sound` axiom + a kernel reduction rule | **dropped**; explicit equivalence relations in libraries | 0 |
| inductive families | YES (the claims need `Eq`, `Bool`, sigma, user `enum`s/`struct`s, and lists once generics exist) | general scheme with positivity, nested->mutual translation, mutual blocks, structure eta, K-like reduction, recursor generation (`inductive.cpp`, from memory ~1,300 lines C++) | **single indexed families, strict positivity, generated recursors + iota.** Nested and mutual are ENCODED by the elaborator (mutual = one family indexed by `Fin k`; nested = the standard unnesting) so the kernel never sees them; no K; no structure eta | 25-40 |
| eta for functions | no | one clause in conversion | **dropped at first**; add if the elaborator needs it (<20 lines); not a soundness surface | 0 (+1) |
| definitional equality | needed (type checking needs conversion) | lazy delta, whnf with caches, `Nat`/`String` literal extensions, proof irrelevance, eta, unfolding heuristics; unpredictable cost; `maxRecDepth` | **no search.** Conversion = syntactic equality after EXPLICIT steps: `beta`, `delta c` (unfold one named constant), `iota`, `prim` (one i64 op), or `reduce k` (the fixed deterministic strategy for at most k steps, k from the certificate). A certificate that needs more fuel is REJECTED, never a hang. Proof terms are larger; the producer is untrusted and pays | 10-14 (whnf-step + fuelled reduce) |
| number type | the claims need i64 | `Nat` unary + GMP-accelerated literal ops inside the kernel (trusted for speed) | **`I64 : Type 0` primitive** with literals and the machine operators (`+ - * / % & \| ^ << >> >>> == != < > <= >=`, `clz`, unary `-`/`!`) as primitive constants; evaluation rule = the hardware instruction; same table feeds the bit-blaster | 6-10 |
| arrays | the claims need cells | -- | `Cells = I64 -> I64` (a Pi type); `sel M i = M i`, `upd` by `if eq`; no array theory in the kernel; 12.1's read-over-write elimination in the VC generator | 0 |
| axioms | none required | `axiom` anywhere; `#print axioms` to discover | **no `axiom` form in the grammar.** A FIXED table compiled into `tcheck.bp`: the 26 syscall footprints (declared state transformers) and nothing else; `deps` recomputed per certificate | 2-3 |
| SAT oracle step | 12.1 | `bv_decide` = verified bit-blaster + external CaDiCaL + LRAT check, all OUTSIDE the kernel (a reflection proof) | `sat(phi, cert#n) : phi` for QF_BV `phi`; the bit-blaster and hinted-resolution checker are INSIDE the trusted checker (decided F6) | 40-60 |

**The logic that results** is a predicative Martin-Löf type theory with an infinite concrete-level hierarchy, single indexed inductive families with recursors, a primitive machine-word type, and an oracle rule -- Agda's core minus universe polymorphism plus i64, or "CIC minus `Prop`, quotients, level terms and eta". It satisfies every clause of the operator's wording. It is proof-theoretically weaker than Lean's (no impredicativity, no choice): the report does not argue from that, only records that the named claims do not touch the difference.

---

## 5. Kernel and elaborator cost against the tree's numbers

### 5.1 `tcheck.bp` (standalone program, trusted)

| module | job | fns (est.) | lines at 25.7/fn | Lean's counterpart (from memory) |
|---|---|---|---|---|
| term store | hash-consed `(tag, a, b, c)` cells; tags Var/Sort/Pi/Lam/App/Const/Ctor/Rec/Lit/Prim | 6-8 | 150-200 | `expr.cpp` + caches |
| text reader / printer | certificate and statement grammar (section 8); uses `scan` | 12-16 | 300-400 | -- |
| lift / instantiate | de Bruijn shifting and substitution (first-order binders only, no names) | 4-6 | 100-150 | `instantiate.cpp`, `abstract.cpp` |
| reduction | `beta`, `delta`, `iota`, `prim`, `reduce k` with fuel | 10-14 | 250-350 | `type_checker.cpp` whnf + lazy delta + literal extension |
| conversion | syntactic equality after explicit steps | 3-5 | 80-130 | `is_def_eq` family |
| infer / check | Var, Sort n, Pi (level max), Lam, App, Const, Lit, Ctor, Rec | 10-14 | 250-350 | `infer_type` family |
| inductives | declaration well-formedness, strict positivity, ctor universe check, recursor type generation, iota rule | 25-40 | 650-1,000 | `inductive.cpp` |
| environment | constants with type/value/digest; the fixed axiom table; `deps` recomputation | 5-8 | 130-200 | `environment.cpp`, `declaration.cpp` |
| i64 primitives | operator table + evaluator, shared with the bit-blaster | 6-10 | 150-250 | GMP `Nat` extension |
| SAT oracle | bit-blaster (64/128-bit add/sub/mul/sdiv/srem/shifts/cmp/logic/mux), Tseitin, clause DB, hinted resolution with deletion | 40-60 | 1,000-1,550 | outside Lean's kernel (`bv_decide` + CaDiCaL + LRAT) |
| `ring` | polynomial normal form over Z/2^64 (section 10) | 8-12 | 200-300 | `ring` tactic (untrusted, outside) |
| binding | sha256 of the obligation (via `use "selfhost/prelude/sha256.bp"`, 5 fns), Merkle root, `deps` line | 3-5 (+5) | 80-130 | -- |
| driver / gates | CLI, `kernel_checked`, `cert_checked`, `checker_neg`, `tcheck_ms` | 5-8 | 130-200 | -- |
| **total** | | **137-206** (+5) | **~3,500-5,200** | order 10^4 lines C++ + GMP + C++ runtime |

Under its own 512 cap by ~2.5x; **no cap change needed for `tcheck.bp`.** The 09-09 report's "150-400 fns for a Lean-subset kernel" was for a kernel with Lean's conventions; the drops of section 4 are what bring the top of the range down, and the bit-blaster + `ring` are what keep the bottom from going lower. The LOGICAL core alone (term store, lift, reduction, conversion, infer, environment) is **38-55 fns / ~950-1,400 lines** -- HOL Light's class (section 6). The rest is (i) inductives, the unavoidable price of the operator's requirement, and (ii) decision procedures that in an LCF system live outside the kernel and here live inside it because certificates for every adder bit would be 100x larger (the F6 argument for hints, applied to gates).

### 5.2 The elaboration phase inside `bebop.bp` (untrusted for proofs, trusted for what the program MEANS -- as the compiler already is)

| part | fns (est.) | note |
|---|---|---|
| term store (a second copy; `bebop.bp` cannot `use` `tcheck.bp`'s without sharing a module, and sharing would couple the trusted program to the compiler) | 6-8 | the two readers/stores are deliberately DIVERSE: gate `def_parity` (section 8.4) compares their digests over the 837 selfhost fns |
| grammar reader into terms: Pi, lambda, `Sort n`, generics `fn f<T>`, closures, dependent `[T; n]`, `requires/ensures/invariant`, `theorem` | 25-40 | the current parser is emission-directed and cannot be reused for this |
| infer / unify (pattern + first-order) / implicit args / metavariables / postponement | 30-40 | a near-duplicate of the kernel's `infer` plus unification |
| universe level inference: level variables, `max` constraints, solving to concrete levels | 8-12 | the solver lives HERE, not in the kernel |
| monomorphisation | 8-12 | instantiation cache keyed by (fn, types) |
| defunctionalisation + closure conversion + lambda lifting + `apply_T` generation + escape analysis for the frame heap | 12-18 | section 3 |
| text generation of lowered fns into the `.use` buffer; annotation erasure | 8-12 | `use_append_str` (`:6525`) is the existing primitive |
| obligation and proof-term writer (text format); position map | 8-12 | section 8 |
| **total** | **105-154**; with margin **115-170** | 291 + 115..170 = 406-461 |

### 5.3 `prove.bp` (standalone program, untrusted producer)

CDCL with watched literals and hint logging (the successor of `dpll.bp`'s 3 fns at 16 vars): 15-25 fns; a rewriter that turns tactic scripts (`unfold`, `rw lemma`, `induct x`, `sat`, `ring`, `reduce`) into primitive certificate steps: 15-25; ~40-70 fns / 1,000-1,800 lines. Its bootstrap twin `tools/prove.py` is ~300 lines (a CDCL whose learned clauses come with their propagation trail as hints). Neither is trusted: a wrong producer yields a rejected certificate.

---

## 6. Prior art, weighed by trusted base and by what to copy

All sizes below are (from memory) unless a venue is named; none was re-measured.

| system | logic | trusted base | what it bought | copy / avoid |
|---|---|---|---|---|
| **Metamath Zero** (Carneiro, "Metamath Zero: The Cartesian Theorem Prover", CADE 2019 / arXiv:1910.10703, cited) | multi-sorted first-order with definitions; HOL/PA/x86 specified as theories inside it | the `.mm0` statement file (human-readable, THE claim) + `mm0-c`, a C verifier of ~1-2k lines that executes the `.mmb` binary proof stream on a stack machine with backreferences; checks the translation of set.mm in about a second (from memory) | a verifier small enough to be verified itself, aimed at verified compilers/kernels; the verified-verifier proof (x86) was in progress as of the last I know (from memory) | **COPY**: statement/proof split = 14.3 + 15.1; opcode stream + stack + backrefs = the binary cache; the size discipline (the verifier under ~1.5k lines); the end-state "the verifier verified". **CANNOT COPY**: its logic (first-order) under the CIC decision |
| **Milawa** (Davis, PhD UT Austin 2009; Myreen & Davis, "A verified runtime for a verified theorem prover", ITP 2011, cited) | first-order with induction, ACL2-shaped | a level-1 proof checker of a few thousand lines of Lisp accepting explicit (enormous) proofs; 11 levels, each level's prover PROVEN sound in the level below and then admitted; the whole tower re-checked from level 1; runs on Jitawa, a Lisp runtime verified in HOL4 to x86-64 machine code | "the prover verifies itself" done for real; the level-1 proofs of the tower were huge (from memory, gigabytes) | **COPY**: the ladder -- `tcheck.bp` (L0) checks; `prove.bp` (L1) produces L0 certificates; admitting L1 after proving it in L0 is the far end. **LEARN**: L0 certificates without compression are enormous -- hence the SAT oracle, `reduce k` and `ring` as kernel-level compression |
| **HOL Light** (Harrison) | simple type theory, 10 primitive rules, 3 axioms | `fusion.ml` ~700 lines OCaml (often quoted as "a few hundred", from memory) + the OCaml runtime; Candle (Abrahamsson, Myreen, Kumar, Sewell, ITP 2022, cited from memory) verified it in CakeML | Flyspeck, Jordan curve theorem (from memory) | the BENCHMARK: `tcheck.bp`'s rule core (38-55 fns) is in its class; the whole is 3-4x larger for two named reasons (recursor generation; in-kernel bit-blaster). HOL Light has no inductives in the kernel (derived via choice + set encoding) and no computation rule -- CIC's recursors and iota are the price of the requirement |
| **ACL2** (Kaufmann, Moore) | quantifier-free first-order with induction; executable definitions ARE the logic's functions | monolithic: the whole prover (order 10^5 lines Lisp) + the host Common Lisp; trust tags (`defttag`) as escape hatches -- an `implemented_by` analogue | AMD K5 FDIV, Rockwell AAMP7, Centaur x86 units, Piton's descendants (from memory) -- decades on money-and-machine claims | **COPY** the executable-definition discipline (section 4's `I64` primitive with machine evaluation). **AVOID** the monolithic trust and the trust tags |
| **Metamath** (Megill) | any, via substitution; set.mm ~40k theorems | `mmverify.py` ~400 lines Python; the algorithm fits on a page | the smallest checker in wide use | the "explicit substitution steps, no search" discipline -- already the conversion design of section 4 |
| **Nqthm** (Boyer, Moore) | first-order + induction | the prover | Piton: a verified compiler to the FM9001 (Moore, ~1989-96, from memory) | the compiler-correctness precedent: reachable in FOL+induction (so in CIC), at multi-year cost -- section 13 |
| **CakeML / HOL4** (Kumar, Myreen, Norrish, Owens, POPL 2014; Tan et al. ICFP 2016, cited) | HOL | HOL4's LCF kernel (~1-2k lines SML) + ISA models + the in-logic bootstrap of the compiler binary | the strongest end-to-end story: compiler correct down to machine code, bootstrapped inside the logic | the end-state for the COMPILER theorem; person-decades cumulative (from memory) |
| **seL4 / Isabelle** (Klein et al. SOSP 2009; Sewell, Myreen, Klein PLDI 2013, cited) | HOL | Isabelle/Pure kernel (several thousand lines SML) + the C parser/semantics + (for the binary) the decompiler and SMT | ~200k lines of proof for ~10k lines of C (from memory); binary verification by decompilation per function | F5's shape: validate the BINARY against the source semantics per build, do not prove the compiler |
| **Lean 4** | CIC + `Prop` + quotients + universe polymorphism | kernel of order 10^4 lines C++ + GMP + C++ runtime + `#print axioms` (propext, `Quot.sound`, `Classical.choice`, `Lean.ofReduceBool` when `native_decide` is used); re-checkers lean4checker/nanoda/lean4lean of the order 5-10k lines each (from memory) | Mathlib; `bv_decide` | section 7, feature by feature |

---

## 7. Lean's holes, and the mechanism that closes each by construction

| Lean hole | concrete cost there | exposed here? | mechanism that makes it impossible (not discouraged) |
|---|---|---|---|
| `native_decide` (`Lean.ofReduceBool` axiom), `implemented_by`, `@[extern]`, `unsafe` | the kernel accepts the COMPILED evaluation of a `Bool`; `implemented_by` is unchecked, so a mismatched implementation plus `native_decide` proves `False` (documented behaviour; the axiom appears in `#print axioms` -- from memory on the demonstrations) | the temptation is "evaluate with `bebop.bin`" | the kernel has ONE computation rule, its own fuelled reducer over its own term language; a Bebop fn enters the logic only through the kernel's READER of its body (`fn f(...) { body }` -> a definition term), so there is no way to declare a replacement; `tcheck.bp` contains no `sys_run`, `sys_clone`, `sys_mmap`, `sys_mprotect` -- **gate `tcheck_sys_census: {open, read, write, close, exit}` only** (a grep over `tcheck.bp`, 0 code) |
| the extraction gap | the proved term and the running code are different artifacts joined by an unverified compiler (~10^5 lines Lean/C++, from memory), a C compiler (~10^6) and a runtime + GMP (~10^5) | yes, but two orders smaller | the running artifact is ~42,000 words produced by a 7,474-line one-pass emitter from <= ~20-word fragments at 532 sites; F5 validates each fragment per build with a QF_BV obligation through `tcheck.bp`; the lowering phase (est. 30-45 fns) is the remaining trusted link until section 3.5's certificate lands. **Quantified: 10^6 unverified lines vs < 8,000 lines with a per-build certificate** |
| definitional equality and whnf: lazy delta, `Nat.rec` unfolding blowups, `decide` timeouts, `maxHeartbeats`/`maxRecDepth` | checking time unpredictable; kernel defeq semi-decidable in practice; the whnf/literal/eta interplay has been a bug source (from memory) | would be, with Lean's design | no search in the kernel; explicit `beta`/`delta`/`iota`/`prim` steps or `reduce k` with k in the certificate; check time linear in certificate size; `tcheck_ms` committed per obligation; a certificate short of fuel is REJECTED with the step number |
| universe polymorphism and level constraints | `max`/`imax` normalisation and `<=` decision in the trusted base | no | concrete levels; the elaborator (untrusted for proofs) solves and instantiates; the kernel computes `max` on two integers |
| `Prop`, definitional proof irrelevance, elimination restrictions, K-like reduction | the subtlest code in the checker; `Acc.rec` reduction troubles; K | no | no `Prop`; predicative hierarchy; large elimination unrestricted by default; no K |
| quotients as a kernel primitive + `Quot.sound` | a reduction rule and an axiom in the trusted base | no | none |
| nested / mutual inductives, structure eta | nested->mutual translation and mutual blocks in the kernel | no | single indexed families in the kernel; the elaborator encodes |
| GMP-accelerated `Nat` literals | a performance extension of the trust root | no -- the gift | `I64` IS the machine word; evaluation IS the instruction; no bignum; theorems about unbounded integers are stated with explicit overflow predicates (`money.bp:28-41` already does) or a 128-bit sort in the bit-blaster (`fp_mul`'s spec) |
| axiom leakage (`#print axioms`) | you must ASK; a theorem silently depends on `propext`/`choice`/`ofReduceBool`; `sorry` warns but compiles | would be | every certificate carries `deps` (axiom-table entries and oracle steps used); the checker RECOMPUTES the set while checking and rejects a mismatch; the Merkle root (15.3) hashes `deps`; there is no admit/`sorry` rule and no `axiom` form |
| kernel size and auditability | 10^4 lines C++ + GMP; three independent re-checkers exist because the kernel is too big to read | -- | 137-206 fns / ~3,500-5,200 lines of Bebop, readable in a day, with a Python twin (section 9.4) as the second implementation from day one |

---

## 8. The term and certificate formats under 11.3 / 12.1 / 13.3 / 14.3 / 15.1 / 15.3

### 8.1 Terms (13.3, ours)

Text, s-expression-shaped, one token class set already served by `scan` (A9, 46 words): `(var k)`, `(sort n)`, `(pi A B)`, `(lam A b)`, `(app f a)`, `(const name)`, `(ctor ind k)`, `(rec ind)`, `(lit 123)`, `(prim add)` etc.; de Bruijn indices, no names inside terms. A definition: `(def name TYPE VALUE)`. An inductive: `(ind name PARAMS INDICES-TYPE (ctor ...)*)`. A Bebop fn entering the logic: `(fn name)` is a definition the kernel's READER produced from the `.bp` source (section 8.4), and its digest is the sha256 of the source text of that fn, so a theorem about `addov` is bound to the bytes of `money.bp:28`.

### 8.2 Obligations and certificates (11.3, 15.1)

```
obl v1                      ; the STATEMENT file -- normative, human-read (MM0's .mm0 role)
  src money.bp addov 7f3a...  ; every referenced fn by file, name, sha256 of its text
  claim (pi (I64) (pi (I64) (eq ...)))
cert v1                     ; the PROOF file
  obl-sha256 <64 hex>       ; 15.1: this certificate discharges exactly that statement
  rules-sha256 <64 hex>     ; digest of tcheck.bp's rule table (changes when the logic changes)
  deps ax:sys_write ax:sys_msync sat:3 reduce:41   ; recomputed by the checker; mismatch = reject
  steps
    1  const addov
    2  beta 1 ...
    3  reduce 40 2 ...      ; at most 40 reduction steps, deterministic strategy
    4  sat 3 (formula)      ; oracle: the next block is a hinted resolution refutation
       c 1 -3 5 0 h 2 7 0  ; clause with hints (our own numbering: the bit-blaster's
       d 2 0                ;   variable order is canonical, so no mapping is carried)
       c 0 h 9 11 0        ; the empty clause
    5  qed 4
```

Every step's check is bounded by its own arguments: `beta` one contraction, `delta` one unfolding, `iota` one recursor step, `prim` one machine op, `reduce k` at most k steps, `sat` linear in the clause block (hinted unit propagation, the F6 argument), `ring` one normal-form computation, `induct` the recursor's type instantiated. No step can loop.

### 8.3 Text normative, binary derived (14.3)

The checker checks TEXT. `tcheck cache in.cert out.cbin` writes the same step stream as fixed-width i64 cells with backreferences (MM0's `.mmb` shape) and records `sha256(cbin)` in the cache index; a consumer that reads the cache re-derives it from the text on a digest mismatch and never checks the binary directly. Cost estimate: a 10 MB certificate at `scan` speed is tens of ms; measured only once the checker exists (`tcheck_ms`).

### 8.4 The reader gate (the definitional link, cross-checked)

`tcheck.bp`'s reader (Bebop fn -> definition term) is trusted; `bebop.bp`'s elaboration reader and `bpref.py`'s parser are two independent implementations of the same grammar. Gate `def_parity: n/837` = the three produce the same term digest for every selfhost fn (a script that prints tcheck's digest, bebop.bp's `--dump-terms` digest, and a Python digest from bpref's AST). Disagreement is a bug in one of three, found by the same method bpref finds emitter bugs.

### 8.5 Merkle root (15.3)

`root = sha256(sorted (obl-sha256 || cert-sha256 || deps-digest) over the commit's obligations)`, computed by `tcheck --root` and by a 30-line Python script; `battery.sh` prints ONE number and compares it to the committed one. The Lean cross-check (section 9.3) is keyed to the same root.

---

## 9. The bootstrap: theorem one, what is trusted at that moment, and the ladder

### 9.1 Theorem one needs no elaborator

Two candidates, in order of what they exercise:

1. **`ordfsm.decide` on the exhaustive matrix** (`ordfsm.bp:33-38`; 12 x 12 = 144 ground facts): statement `(eq (app decide adj 3 4) (lit 0))` etc., proof `reduce k` + `refl` -- a ground fact discharged by the kernel's reducer alone, NO producer of any kind. It re-proves the golden's section A as kernel-checked facts; it is a test rephrased, and it is the first `kernel_checked >= 1`.
2. **`addov` (`money.bp:28`) is correct**: for all a, b : I64, `addov(a, b) = 1` iff the 65-bit sum is outside [-2^63, 2^63). A genuine universal claim over 2^128 inputs; QF_BV with two adders and a comparator; the proof term is `(sat 1 phi)` and the certificate block comes from `tools/prove.py` (untrusted). The first REAL theorem, and the first `cert_checked`.

Then `subov`, `op_add`/`op_sub` (straight-line, `st` as `upd` chains), the FSM's `step` invariants, and `store.bp`'s `st_alloc`/`st_len` bounds -- all adders and comparators.

### 9.2 Trusted at that moment, item by item

`seed/build/seed` (488 `.text` bytes, gated to rebuild; TRUST-CHAIN §1); `bebop.bin` (171,320 bytes; Thompson applies; D5b witness at 46/72 non-vacuous today); `tcheck.bp` source (~3,500-5,200 lines) and its binary; `selfhost/prelude/sha256.bp` (133 lines, 5 fns); the hand-written statement (read by a human -- MM0's `.mm0` discipline); the i64 operator table (= the bit-blaster's definitions, cross-checked against `bebop.bin` and bpref through the 89 constructs); the kernel's reader for the fns the statement names (cross-checked by `def_parity`). NOT trusted: `tools/prove.py`, Lean, any solver.

### 9.3 Lean as the cross-check oracle, with the process budget

`tools/to_lean.py` prints each SAT-shaped obligation as `theorem ... : ∀ a b : BitVec 64, ... := by bv_decide` and, for `reduce`-shaped ones, `by decide`; the Lean run (on-box: 4.33.1, cold 20.3 s, warm 0.4 s, ~735 MB -- lane C) emits a results file bound by sha256 to the statement file and the Merkle root; `battery.sh` compares. **Because `bv_decide` costs +7 processes against the 32 cap with three lanes running, the cross-check is a slot-class batch keyed to the root, not a per-obligation gate.** Inductive theorems (store loops, sha256) get a Lean cross-check only if someone writes the Lean proof; the gate `lean_agree: n/m` counts the SAT- and decide-shaped ones honestly. Disagreement = a bug in `to_lean.py`, the bit-blaster, or Lean -- and the first two are ours.

### 9.4 The ladder, and the instrument that makes the kernel refutable while half-built

L0 `tcheck.bp`: primitive steps + oracle. L1 `prove.bp` (+ `tools/prove.py` for SAT at first): emits L0 certificates from scripts. L2, the elaboration phase in `bebop.bp`: emits L0 terms for the program's types and its obligations. L3 (Milawa's actual trick; NOT on the horizon): prove L1 sound in L0 and admit its verdicts without certificates -- never needed while certificates are cheap, and 15.x makes them the audit trail anyway.

The refutation instrument, built BEFORE `infer`: `tools/kcheck.py`, a Python kernel of the same calculus (~1,000 lines, the bpref of the kernel), and a negative corpus of terms that MUST be rejected -- `Type : Type` (Girard's paradox entry), Hurkens' paradox in the impredicative encoding (must fail because there is no `Prop`), a non-strictly-positive inductive, a recursor applied at the wrong motive level, an ill-typed application, `reduce` short of fuel. From the first `infer` commit the numbers `kernel_neg: 0 accepted of N` and `kernel_parity: n/N` (tcheck vs kcheck on positive and negative corpora) exist, and a kernel that accepts one paradox dies that day on a number, like B4 step 2 did.

---

## 10. What the named claims need from this kernel -- and the nonlinear risk

| claim | shape in the term language | discharge | risk |
|---|---|---|---|
| money laws: `op_add`/`op_sub`/`op_neg`/`op_nonneg` vs the Rust oracle's semantics | straight-line, `st` as `upd` chains, adders/comparators, 64-bit | `sat`; certificates small | none expected |
| `op_taxx`/`op_taxi`/`op_eur` | `sdiv` by constants (10^6, 10^9), one `mul` | `sat` with a constant divider (~4k gates each); expected feasible | unmeasured |
| **`mulov` (`money.bp:30-41`)** | `a * b / bs == a` compared to "the 128-bit product fits" | a 64x64 multiplier AND a 64-bit divider against a 128-bit multiplier: the hard miter | **HIGH** for SAT; `ring` + floor lemmas route |
| FSM: `decide` matrix, `step`/fold invariants, acyclicity (`nilpotent`, 24 loops) | ground facts + loop invariants over 12-bit masks | `reduce`; `induct` on the loop counter with `sat` per step | invariants to write: 24 |
| store: `st_alloc` cursor monotone, `st_len` bounds, crc-on-copy in `st_copy_obj`/`st_compact` (20 loops) | `Cells` as `I64 -> I64`, interval-disjointness arithmetic; syscall footprints as axioms | `induct` + `sat` per iteration; `crc32x` is a primitive whose spec is the CRC polynomial | `deps` will list `sys_msync`, `sys_rename`, `sys_ftruncate` -- and no theorem crosses them |
| **`fp_mul` (`fp.bp:6-30`)** = sign * floor(|a||b| / 2^32) | limb identity vs a 128-bit product | ring identity + the nested-floor lemma `floor(floor(x/2^16)/2^16) = floor(x/2^32)` + range facts (`a0, b0 < 2^32`) | **HIGH** for SAT (different multiplier decompositions); the `ring` route is ~10 lemmas |
| **`isqrt` (`fp.bp:35-44`)** bracket for 0 <= s < 2^62 | Newton loop, <= 6 iterations; invariant `x >= floor(sqrt s)`, descent | `induct` on the loop with an AM-GM-style lemma `(x + s/x)/2 >= floor(sqrt s)` -- nonlinear with `sdiv` | **HIGH** for SAT; needs `ring` + two division lemmas |
| the compiler's own correctness | `run_arm(compile P) = run_bp P` over the AST as cells, with fuel | `induct` over the emission structure; the AArch64 subset semantics as definitions | out of horizon (section 13); F5 per build instead |

**The pre-measurement, zero commits, one slot-class batch:** bit-blast the `fp_mul` and `mulov` miters through Lean's `bv_decide` on-box (it is the oracle; it is installed). If it proves them in minutes and the LRAT is under ~100 MB, the SAT route is open and the certificate size is the number to commit; if it times out, the `ring` module (8-12 fns) is on the path to F9 and should be scheduled before the flagship theorems. Either outcome is a number before a line of kernel is written.

---

## 11. The trusted base when this is done, item by item, against Lean's

| # | Bebop item | size | gate | Lean's counterpart |
|---|---|---|---|---|
| 1 | `seed/build/seed` `.text` | 488 bytes (1,480 ELF) | invariants (viii) rebuild byte-exact | C++ runtime + libc + GMP (~10^5-10^6 lines) |
| 2 | `bebop.bin` | 171,320 bytes; compiles `tcheck.bp` (Thompson) | fixpoint gen3 == gen4; D5b witness 46/72 non-vacuous today; bpref as second executor of `tcheck.bp` on every batch | the C++ compiler that built the kernel |
| 3 | `tcheck.bp` source and binary | ~3,500-5,200 lines / 137-206 fns (est.) | `kernel_neg`, `kernel_parity` vs `kcheck.py`, `checker_neg`; readable in a day | kernel ~10^4 lines C++ (from memory) |
| 4 | the rule table (the logic of section 4) and the i64 operator table | ~15 rules + ~20 ops; `rules-sha256` in every certificate | operator table cross-checked through 89 constructs on `bebop.bin` and bpref | CIC + `Prop` + quotients + levels + literal extensions |
| 5 | the reader from `.bp` fn text to definition terms | inside item 3 | `def_parity: n/837` against `bebop.bp`'s and bpref's readers | the elaborator's `#print` of a statement (trusted for the STATEMENT in Lean too) |
| 6 | the fixed axiom table: 26 syscall footprints | ~26 declarations | listed in every `deps`; hashed into the root | `propext`, `Quot.sound`, `Classical.choice`, `ofReduceBool`, any user `axiom` |
| 7 | the lowering phase in `bebop.bp` (mono + defunc) -- UNTIL section 3.5's certificate lands | est. 30-45 fns | `core_reject: 0`; later the logical-relation certificates | the Lean compiler (unverified, ~10^5 lines) |
| 8 | F5's fragment validator rules (the AArch64 subset semantics) | a few dozen forms | `tv_fragments: t/t` | none -- Lean has no per-binary validation |
| 9 | `selfhost/prelude/sha256.bp` for 15.1/15.3 | 133 lines / 5 fns | golden gate exists | -- |
| 10 | hardware, Linux, binutils for the seed | out of scope | -- | same |

NOT trusted: Lean 4 and its toolchain, any SAT/SMT solver, `prove.bp`, `tools/prove.py`, `tools/to_lean.py`, the binary certificate cache, the elaboration phase's PROOF output (re-checked), the unification and level solver (their output is re-checked). Compared with the 09-09 report's list (§7.4), the change is that "the VC generator" has become two named, gated things -- the reader (item 5) and the lowering phase (item 7) -- and the dependent-type checker is inside item 3 instead of being a second trusted program.

---

## 12. Cost in the project's units

Velocity inputs (operator-supplied, not re-measured): ~1.7 commits/h, ~1 roadmap step/h of continuous operation, one `tools/slot.sh` slot (chain ~100 s, battery ~110 s = ~3.5 min per landing).

| lane | file(s) | steps (est.) | serial? | first number |
|---|---|---|---|---|
| K0 twin + negative corpus | `tools/kcheck.py`, `bench/kernel_neg/` | 4 | parallel | `kernel_neg` denominator |
| K1 term store, reader/printer, sha256 binding | `tcheck.bp` | 4 | parallel | `def_parity` |
| K2 lift/subst, reduction with fuel | `tcheck.bp` | 3 | parallel | `kernel_parity` on ground terms |
| K3 infer/check, concrete levels | `tcheck.bp` | 3 | parallel | `kernel_neg: 0/N` |
| K4 inductives + recursors + iota | `tcheck.bp` | 6 | parallel | `kernel_checked >= 1` (theorem one, candidate 1) |
| K5 environment, axiom table, `deps` | `tcheck.bp` | 2 | parallel | -- |
| K6 i64 primitives | `tcheck.bp` | 1 | parallel | -- |
| K7 SAT oracle (bit-blaster 4, Tseitin/clause DB 2, hinted resolution 2, `checker_neg` corpus 2) | `tcheck.bp` | 10 | parallel | `cert_checked`, `checker_neg` (theorem one, candidate 2) |
| K8 `ring`, Merkle root, driver, gates | `tcheck.bp` | 4 | parallel | root as ONE number |
| E0 cap raise to 1024 (append zone) | `bebop.bp` | 1 | **serial** | fixpoint; 600-fn probe |
| E1 term store + grammar reader into terms | `bebop.bp` | 6 | **serial** | `def_parity` |
| E2 infer/unify/implicits/metavars | `bebop.bp` | 8 | **serial** | `elab_neg: 0/N` |
| E3 level inference and instantiation | `bebop.bp` | 3 | **serial** | -- |
| E4 monomorphisation | `bebop.bp` | 4 | **serial** | constructs; `word_budget` lines |
| E5 defunctionalisation, closure conversion, lifted text, escape analysis | `bebop.bp` | 6 | **serial** | constructs; `core_reject: 0` |
| E6 obligation/proof-term writer, position map | `bebop.bp` | 3 | **serial** | `kernel_checked` from elaborated programs |
| E7 constructs (+`neg/`) for closures, generics, dependent types | `bench/parity_constructs/` | 4 | parallel | construct parity |
| P `prove.bp` (CDCL 4, rewriter/induction 5) + `tools/prove.py` (1) | new files | 10 | parallel | certificates for K7 |
| L Lean cross-check (`to_lean.py`, results-file gate, batch rule) | `tools/`, `battery.sh` | 3 | parallel; slot-class runs | `lean_agree` |
| T first theorems: FSM matrix (1), `addov`/`subov`/`op_add` (2), store bounds (2), `fp_mul`/`isqrt`/`mulov` via `ring` or SAT after the section-10 experiment (4-8) | `.obl/.cert` files | 9-13 | after K | `theorems: n` |
| D docs: LANGUAGE.md rewrite (A16), TRUST-CHAIN item list, thesis amendment wording | docs | 2 | parallel | -- |
| **total** | | **~95-100** | E = 31 serial | |

Arithmetic: ~95-100 steps = ~95-100 h of continuous operation at 1 step/h, ~160-170 commits at 1.7/h. Two staffed lanes (K + P + L + T on new files; E on `bebop.bp`) give a wall of ~max(K 37, E 31) + T ≈ **45-50 h**; one lane, ~95 h. Slot time ~6 h total; not binding. **Theorem one arrives after K0-K4 (or K0-K3 + K7), ~20-30 h of the kernel lane, with the elaborator untouched.**

Against the 09-09 report's F6-F9 (30-52 person-weeks): this design removes the Lean-subset kernel conventions and moves automation to an untrusted program, but adds the SERIAL elaborator inside `bebop.bp`, which sits on the same one-writer spine as F2, F3 and F5's compiler slices. In that report's units (calibration: "~1 blueprinted row per 1-2 lane-days, 3-5x faster than human-weeks, and it does NOT transfer to kernels"), the honest range is 1-3 h per K/E step, i.e. **100-300 h**, and the serial spine of Phase F grows by E's 31 steps. The number that decides whether the low end is real is `kernel_parity` after K3: if the twin and the kernel disagree on the first 100 terms, the velocity is the high end.

---

## 13. What is not reachable on the horizon, with the argument

1. **The checker verified in its own logic** (MM0's and Milawa's end state). It needs the interpreter-level semantics of Bebop as definitions in the logic, the AArch64 subset semantics, and an induction over `tcheck.bp`'s own 137-206 fns. Both precedents took years of a specialist (from memory); nothing in the tree's velocity transfers. Self-APPLICATION is free (tcheck checks certificates about tcheck's fns, since they are Bebop); self-VERIFICATION is not.
2. **The compiler's correctness as a theorem.** Reachable in this logic (Piton in Nqthm's weaker one), at multi-year cost; F5's per-build translation validation is what lands, and its validator rules are trusted item 8.
3. **Theorems across `sys_clone`, a crash, or a syscall beyond its footprint.** The 26 syscalls and 7 threading builtins are axioms with declared footprints (unchanged from the 09-09 report §5.2); B1's torn-write harness stays a test.
4. **Function extensionality, `propext`, choice.** Not provided (no `Prop`, no axiom form). A theorem "these two closures are equal as functions" must be stated pointwise. Nothing in sections 9-10 needs them; if a future claim does, the fixed axiom table is the only door and it is a commit that changes `rules-sha256`.
5. **The flagship theorems by SAT alone** (section 10): at risk, not refuted; the experiment decides in one batch.
6. **"If Lean disappeared tomorrow"**: everything above still holds -- Lean's only role is `lean_agree`, and the tooling that produces it is ours. That test is passed by construction; what would be lost is the second opinion on the bit-blaster.

---

## 14. Evidence

Repository (read today; line numbers from `grep -n`/`sed -n`): `docs/LANGUAGE.md` :9, :41-43, :53-55, :64-66, :73, :80-81, :103-117, :119, :127-133; `ROADMAP.md` thesis :8-26, A1b :87, A16 :104, Phase F preamble :181, F0-F9 :184-193, open decisions :198-219, measured :221-292; `docs/RESEARCH-VERIFICATION-2026-09-09.md` §0-§13 (the prior costing and trust list); `docs/RESEARCH-LITERATURE-2026-09-08.md` §6, §14 (house style; LFI; DDC); `docs/TRUST-CHAIN.md` §0-§7 (artifact table; D5b ledger 46/72); `tools/ddc.sh` (measure/full); `bebop.bp` `read_ident` :147, `kernel_marked` :116-121, `vs_cs_take` cap check :2703, fn-cap comments :673, :3575-3579, `emit_array_get` :4001, `vs_array_get_imm/reg` :4018/:4023, `fntab = zeros(8192)` :4237/:5628/:6069/:6232, `compile_program_offs` :6005, `use_*` :6513-6679, `.use` re-slurp :6510, `cli_compile` :6918, the `blr` comment :7378; counts of section 1 (python over `fntab\[(\d+)`, `zeros\((\d+)\)`, `^fn `); `selfhost/prelude/fp.bp` :6-30, :35-44; `selfhost/std/money.bp` :24-41, :43-147; `selfhost/std/ordfsm.bp` :33-38 and fn list; `selfhost/std/dpll.bp` (whole); `selfhost/prelude/store.bp` :160-240, :455-583; `selfhost/prelude/sha256.bp` :1-70; `tools/bpref.py` :1-60 and the def/class list; `tools/typecheck.py` :1-60; `docs/blueprints/A5-arena-relative-addressing.md` :24 (2^29 cells); `docs/PERF.md` :11; `docs/TRAPS.md` :29; `samples/theorem-sample.bp`; `bench/VERIFICATION.md` :1-40; `bench/parity_constructs/` and `neg/` listings. Lane C's Lean measurement (4.33.1; 3.0 GB; 20.3 s / 0.4 s; ~735 MB; +7 processes) is quoted from the coordinator and was not re-run.

Literature (cited = venue named from knowledge; from memory = a figure I do not pin): Reynolds, "Definitional interpreters for higher-order programming languages", ACM 1972; Carneiro, "Metamath Zero", CADE 2019 / arXiv:1910.10703; Davis, "A Self-Verifying Theorem Prover", PhD UT Austin 2009; Myreen & Davis, ITP 2011 (Jitawa); Harrison, HOL Light; Abrahamsson, Myreen, Kumar, Sewell, ITP 2022 (Candle); Kaufmann & Moore, ACL2; Boyer & Moore, Nqthm; Moore, Piton; Kumar, Myreen, Norrish, Owens, POPL 2014; Tan et al., ICFP 2016; Klein et al., SOSP 2009; Sewell, Myreen, Klein, PLDI 2013; Kaufmann, Biere, Kauers, FMCAD 2019 (multiplier verification via SAT + computer algebra); Heule, Hunt, Kaufmann, Wetzler, ITP 2017 (LRAT); Tan, Heule, Myreen, TACAS 2021 (cake_lpr); Thompson, CACM 1984; Wheeler, arXiv:1004.5534; Dybjer, "Representing inductively defined sets by wellorderings in Martin-Löf's type theory", TCS 1997 (why W-types alone do not suffice without funext). From memory, unpinned: every kernel and verifier size in section 6; Lean's `Level`/`inductive.cpp` line counts and the `implemented_by`/`native_decide` `False` demonstrations; MM0's set.mm timing and the status of its verified verifier; Milawa's proof volumes; the SAT bit-width limit for multiplier miters; seL4's and CakeML's effort figures; Piton's dates.

Not determined here: `bin_words` today (three inherited values, section 1); whether fn index 0 is usable under the cap; the certificate sizes of any bit-blasted obligation (section 10's experiment); the cost of `scan`-based certificate parsing; whether a 4 GiB reserve (2^29 cells) or the 1 GiB fallback is what the stub reserves today.

VERDICT: with one language the elaborator lives in `bebop.bp` and the cap must rise to 1024 -- ~30 edit sites by appending a second window zone above `fntab[5000]`, est. ~0 `bin_words`, one A1b-shaped fixpoint commit -- because 291 + a minimal elaborator's 115-170 fns is 406-461 with the compiler still growing; elaboration is a T47-shaped textual phase (zero emitted words, the emitter unchanged) that the thesis amendment must be WIDENED to admit; closures pack into one i64 (9 + 29 bits) with `if`-chain `apply` fns (0 `blr` sites exist) and collide with the arena, not the register model (escaping closures in loops = trap-census row 5); the CIC-class kernel is 137-206 fns / ~3,500-5,200 lines in a standalone `tcheck.bp` needing no cap change, because levels are concrete, there is no `Prop`, no quotients, no nested/mutual, no eta, no K and NO definitional-equality search (every conversion is an explicit step or `reduce k` with the fuel in the certificate), while `I64` is the machine word and the SAT oracle and `ring` sit inside; the logical core alone is HOL-Light-class at 38-55 fns; each of Lean's holes has a construction-level closure (no `sys_run` in the checker, definitions read from bodies, a fixed axiom table with no `axiom` form, `deps` recomputed, fuel per step, levels as integers); theorem one (`decide`'s matrix by `reduce`, then `addov` by the oracle) needs no elaborator and arrives after ~20-30 h of the kernel lane; the flagship `fp_mul`/`isqrt`/`mulov` are multiplier-vs-multiplier miters that SAT may not close and the `ring` route must be scheduled after one `bv_decide` experiment; the whole is ~95-100 steps (31 serial in `bebop.bp`), ~45-50 h wall on two lanes, with the honest 1-3 h/step range for kernel work making it 100-300 h; a half-built kernel gets its refutation number from a Python twin and a paradox corpus built first; and the checker's self-verification and the compiler theorem are beyond the horizon in any logic.
