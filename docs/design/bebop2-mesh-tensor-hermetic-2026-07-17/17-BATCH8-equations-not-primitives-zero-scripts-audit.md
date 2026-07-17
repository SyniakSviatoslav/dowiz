# BATCH-8 — "Equations, not primitives" + "scripts → zero": a live-source audit

> **Scope:** research + audit only (no code written). Answers the operator's standing
> execution-model rule (memory `bebop2-mesh-masterwork-2026-07-17.md`): *core logic
> as EQUATIONS compiled via `tools/eqc`; Rust/other langs are adapters/bridges;
> scripts (bash/python) minimized toward zero.* Read live source across `dowiz/kernel`,
> `dowiz/engine`, `dowiz/tools`, `dowiz-spectral-evolution/kernel`,
> `bebop-repo/bebop2` (proto-wire / proto-cap), `dowiz-agentic-mesh`.
>
> **Verdict labels:** `PROVEN` = a value/artifact was read off disk this pass ·
> `MEASURED` = a count/grep result · `JUDGED` = an engineering assessment.
> Complexity/rewrite-cost is NOT used as an objection (per the arc's rule); a concept
> is rejected only on hard correctness grounds (a real bit-permutation eqc cannot
> express, a genuinely iterative solver with no closed form).

---

## (A) eqc.py bootstrap / port verdict — **ALREADY DONE; the flagged contradiction is resolved, the dead files just weren't deleted**

The memory flagged a "live contradiction on its own home ground": `tools/eqc/eqc.py`
(the equation compiler) is written in Python, violating "no python." **That is already
fixed.** A from-scratch, zero-dependency Rust port `tools/eqc-rs/` exists (`src/lib.rs`,
`tests/proof.rs`, `README.md` — all dated **2026-07-17**, i.e. created *after* the
memory note), and it is the one CI actually runs:

- `dowiz/.github/workflows/ci.yml:30-42` — job `eqc-proofs` runs `cd tools/eqc-rs &&
  cargo test --release`. The Python `tools/eqc/` is **not referenced by any workflow**
  (MEASURED — grep of both repos' `.github/`).
- `tools/eqc-rs/README.md:11` — *"Supersedes `tools/eqc/` (Python + SymPy), retired
  2026-07-17."* `src/lib.rs:38-43` repeats it in the module doc.

**Feature parity (PROVEN by reading both):** eqc-rs reimplements exactly the subset
eqc.py exercised — `Expr` = `{Sym, Num, Sum, Prod, Pow(i32), Sqrt, Sin, Cos, Exp}`
(`eqc-rs/src/lib.rs:52-62`), the three emitters (`emit_f64_rust`, `emit_fixed_rust`
returning `Result<_, FixedPointUnsupported>`, `emit_proof_program`), and the same
Q-format fixed-point model (`Add→I₁+I₂`, `Mul→(I₁·I₂)/2^SHIFT` in i128, `Pow(n≥0)→
repeated mul`). It is arguably **more correct** than eqc.py: eqc.py leaned on SymPy's
`rust_code` printer, which `rust-core/tests/eqc_proofs_test.rs:8` documents as buggy
("sympy's `rust_code` mis-distributes (Add)/Mul"); eqc-rs hand-rolls a fully-
parenthesized `to_rust_f64` (`lib.rs:140-158`) that avoids that class entirely, and its
proof reference is an independent tree-walking `Expr::eval` (`lib.rs:124-138`) rather
than SymPy's `evalf`.

**Is there a bootstrapping / chicken-egg problem? — NO (PROVEN + JUDGED).** Two
independent reasons:
1. eqc is a *codegen* tool, not a self-hosting compiler. It emits `.rs` text at
   authoring time; it never has to compile itself with itself. The output is ordinary
   committed Rust built by normal `cargo` (`lib.rs:12-16`). Nothing about porting it to
   Rust requires eqc to already exist in Rust.
2. eqc-rs deliberately has **no text parser**: equations are authored as `Expr` trees
   via Rust operator overloading (`a * x.clone().pow(2) + b * x + c`, `lib.rs:161-215`).
   `README.md:18` states it outright — *"there is no text parser, so there is nothing
   to bootstrap."* My own reading agrees: the only thing SymPy provided that eqc-rs does
   not is algebraic *simplification* (simplify/diff/integrate), and eqc.py **never used
   those** (`README.md:13-17`), so there is **zero capability regression**.

**What the port cost / traded (JUDGED):** the sole real difference is authoring
ergonomics — SymPy let you write `sp.sqrt(a**2+b**2)`; eqc-rs wants
`(a.pow(2)+b.pow(2)).sqrt()`. That is a syntax tax, not a capability loss, and it buys
zero-dependency + Rust-native + the removal of a buggy third-party printer.

**The remaining, honest gap (JUDGED — this is the real finding, not the language of the
compiler):** *neither* eqc.py nor eqc-rs is wired into a single **dowiz kernel organ.**
The only live consumers are four **toy** equations in `bebop-repo/rust-core/eqc-proofs/`
— `lambda_max_of_d = 2*d`, `cheby_coeff_inner`, `cheby_recur_weight`, `cosine_div`
(PROVEN: read `lambda_max_of_d.fn.rs` = `2.0 * d`; consumed by
`rust-core/tests/eqc_proofs_test.rs` via `#[path]` includes). Every real numeric organ
in `dowiz/kernel` (`money`, `domain`, `geo`, `spectral`, `order_machine`) is **hand-
written Rust**. So "equations not primitives" is **built-and-proven-capable but unused
in the product kernel** — and that is true regardless of whether the compiler is Python
or Rust. The high-leverage work is not "port the compiler" (done) but "actually route
kernel organs through it," which §(B) inventories.

**Action items falling out of (A):**
- **Delete the retired Python trio** `tools/eqc/{eqc.py, demo.py, test_eqc.py}` (+
  `requirements.txt`, `__pycache__`). Confirmed safe: **not CI-wired** (CI uses eqc-rs)
  and **no longer a codegen input** (eqc-rs supersedes it). This is exactly the memory's
  "actually delete confirmed-dead legacy code — verify not CI-wired/codegen-input
  first" rule, and both preconditions are met. (MEASURED / PROVEN.)
- The stale worktree `dowiz-agentic-mesh/tools/eqc/*.py` is the same dead trio; delete
  with its branch or on merge.
- The bebop `rust-core/eqc-proofs/*.fn.rs` were generated by the *old* eqc.py but are
  committed hand-inspectable Rust — they are fine to keep; if regenerated, regenerate
  from eqc-rs.

---

## (B) Equation-extraction candidates — kernel numeric organs

**Framing (critical, honest):** eqc's reach is **scalar `args → scalar`, over `± · `,
non-negative integer powers, constants** (+ `sqrt/sin/cos/exp` in the f64-only path).
It **cannot** express: variable-length reductions (Σ over n neighbours/edges/logits),
matrix/elementwise operators of runtime size, complex arithmetic, iterative solvers
(power iteration, Faddeev-LeVerrier, Durand-Kerner, QR), or GF(2) bit-permutations
(SHA3, FNV). Those are not "unconverted primitives" — they are genuinely outside a
scalar-arithmetic codegen and are correctly hand-written Rust. The table separates the
two honestly.

| # | file:line | current code (what the loop/expr does) | proposed equation | eqc-**today** capable? |
|---|-----------|----------------------------------------|-------------------|------------------------|
| 1 | `kernel/src/geo.rs:39-41` `ema_next` | `prev + alpha*(sample-prev)` — the 1-D Kalman / EMA affine update, hand-written | `ema = (1−α)·prev + α·sample` (affine map) | **YES, cleanly.** Pure `±·`; f64 today, fixed-point representable. Zero blockers — the ideal *template* organ. |
| 2 | `kernel/src/domain.rs:95-111` `compute_order_total` + `money::apply_tax` | `total = subtotal + tax(subtotal,rate) + fee`, `checked_add` guards; **this is literally eqc-rs README's flagship** `tax = sub*rate` | `total = subtotal·(1+rate) + fee` in Q-format i64 | **PARTIAL.** Arithmetic yes (README worked example). Blocked on: (a) S2 removal of `tax_rate: f64` → integer basis-points; (b) eqc-rs emitting `checked_mul`/overflow-guard (its own roadmap TODO, `eqc-rs/README.md:96`). Red-line / operator-gated. |
| 3 | `kernel/src/householder.rs:224-229` **and** `246-250` | 2×2 eigenvalue block: `tr=a+e; det=ae−bd; disc=tr²−4det; r=(tr±√disc)/2` — **duplicated verbatim, two copies** | `eig2x2(a,b,d,e) = (tr ± √(tr²−4det))/2` (closed-form quadratic) | **NO as-is** (complex arithmetic; disc can be <0, uses `Complex`). But the *duplication* is a real drift hazard — consolidate to ONE shared closed-form helper regardless of eqc. |
| 4 | `kernel/src/geo.rs:15-28` `haversine_meters` | `2R·asin(√(sin²(Δφ/2)+cosφ₁cosφ₂sin²(Δλ/2)))` hand-written trig | same, as an `Expr` | **PARTIAL.** eqc-rs has `sqrt/sin/cos/exp` but **no `asin`/`atan2`** node yet. Add those two nodes → haversine + `bearing_deg` become f64-emittable (fixed correctly refused). |
| 5 | `kernel/src/spectral.rs:284-297` `laplacian` / `incidence.rs:107-117` | `L=D−A`; per-edge `flow=w·(x_head−x_tail)` scattered ±1 | per-entry `L_ij = δ_ij·deg_i − A_ij`; per-edge kernel is affine | **NO** (whole operator): `deg_i=Σ_j A_ij` is a variable-length reduction. The scalar *leaf* (`w·(x_h−x_t)`) is eqc-able; the graph scatter is a DOD/matvec concern, not eqc's. |
| 6 | `kernel/src/simd.rs:29-42` `softmax_scalar` | `exp(xᵢ−max)/Σⱼexp(xⱼ−max)` over variable-length row | softmax (canonical) | **NO** — variable-length `max`+`Σ` reduction. The per-element leaf `exp(x−m)` is eqc-able; the reduction is not. |
| 7 | `kernel/src/spectral.rs:64-94` `Complex::{mul,div,sqrt}` | `(ac−bd, ad+bc)`, complex div, principal `sqrt` — hand-written | canonical complex-op closed forms | **NO** — single-real-output codegen; no complex type. Would need an eqc complex extension. |
| 8 | `kernel/src/order_machine.rs:311-361` `spectral_radius()` | **1000-iteration power iteration + Rayleigh quotient** over the FSM's directed adjacency, to compute ρ | **ρ = 0** (compile-time constant: the lifecycle is a fixed DAG ⇒ nilpotent adjacency ⇒ ρ=0, Perron–Frobenius) | **NOT eqc (iterative).** But the deeper finding: the whole graph is a **compile-time constant**, so this 1000-iter loop rediscovers a *theorem* at runtime every call. Replace with a proven `const`/const-eval (or gate the golden-signature on the const). Higher leverage than eqc-ifying. |
| 9 | `kernel/src/order_machine.rs:64-78` `allowed_next` + `claim_machine.rs:72-81` | `match from { Pending => &[Confirmed,Rejected,Cancelled], … }` — the FSM adjacency encoded as a `match` | adjacency **relation** `A[from][to]∈{0,1}`; `legal = A[from][to] ∧ from≠to ∧ ¬scaffold` | **NO** (boolean predicate over a table, not scalar arithmetic). This is procedural→**declarative-data** (a pinned adjacency table), a DOD win, not an eqc-equation win. |
| 10 | `kernel/src/event_log.rs:30-125` `sha3_256` keccak-f | θ/ρ/π/χ/ι rounds — XOR, rotate, AND over GF(2) | Keccak-f[1600] permutation | **NO, correctly.** Bit-permutation, outside `±·`. Crypto boundary — stays hand-written / crypto-core; the blueprint already excludes crypto from the fixed-point subset. |

**Reduction of the table:** the genuinely eqc-today (or one-node-away) organs are the
**scalar closed forms** — `ema_next` (clean), the **money law** (flagship, blocked on
integer-rate + overflow-guard), and `haversine`/`bearing` (blocked only on adding
`asin`/`atan2` nodes). Everything else the task's phrasing gestured at — power
iteration, Faddeev/Durand-Kerner, QR, SHA3, Merkle, FNV, softmax/Laplacian reductions —
is either a genuinely iterative solver or a reduction/permutation that eqc **cannot and
should not** try to express. That boundary is the honest result, not a dodge.

---

## (C) Full script inventory — 3-way classification

**Headline (MEASURED, `find -maxdepth 5` sh+py, excl `node_modules`/`target`/`.git`):**
`dowiz` **153** · `bebop-repo` **41** · `dowiz-agentic-mesh` **121** = **315 files**.
That raw number is misleading; the breakdown matters more than the total:

| Bucket | Approx count | Classification | Notes |
|---|---|---|---|
| **Vendored agent/skill bundles** (`.agents/skills/**`, `.claude/skills/**`: pdf, skill-creator, webapp-testing, last30days, skillspector) | ~116 in dowiz, mirrored in agentic-mesh; `tools/skillspector` alone is a whole Python package w/ its own `.venv` | **JUSTIFIED-TO-KEEP (out of core scope)** | Third-party Claude-Code skill packages, **not dowiz product logic**. Not what "core logic is Rust, no scripts" targets. `dowiz-agentic-mesh` re-vendors the same set (worktree duplication). |
| **Harness / git-hook glue** (`.claude/hooks/*.sh` ×12, `.husky/`) | ~14 | **JUSTIFIED-TO-KEEP** | Per `CLAUDE.md`, these are **no-op pass-throughs** already (Mandatory-Proof Rule suspended). Zero computational logic; effectively dead-but-harmless. |
| **CI red-line guards** (`bebop-repo/scripts/ci-*.sh`, `test-*.sh`: no-courier-scoring, crdt-fence, kernel-fence, empty-imports, ungated-keygen, …) | ~24 | **JUSTIFIED-TO-KEEP** | `grep`-based invariant fences — pure CI glue, zero math. These *enforce* the red-lines; removing them weakens governance. Meets the task's own "keep only" bar. |
| **Build / deploy glue** (`scripts/build-kernel-wasm.sh`, `check-zero-oci.sh`, `verify-kernel-engine.sh`, `docker/mkosi-rootfs.sh`, `deploy/check-no-docker.sh`, `bebop2/tooling/*.sh`, `bebop2/*/scripts/check-wasm32.sh`) | ~15 | **JUSTIFIED-TO-KEEP** (build orchestration) | Toolchain/orchestration, no core logic. The operator's "scripts→0" ideal touches these last; they wrap `cargo`/`wasm`/`mkosi` which have no Rust-native substitute. |
| **Telemetry / self-improvement tooling** (`tools/telemetry/*.{sh,py}`, `tools/loop-signals/{check.sh,transcript_events.py,test_transcript_e2e.py}`, `scripts/analyze-bottlenecks.py`, `kernel/benches/bench_track.py`) | ~15 (dowiz) | **GENUINE-GAP (low priority)** | Computational-ish Python in the self-improvement/telemetry loop. Dev-tooling, not product kernel. `analyze-bottlenecks.py:head` even documents *why Python not bash* (stateful pause-accounting) — a real reason, but still a port candidate under the directive. |
| **LLM eval harness** (`eval-layer/{eval_runs,metrics,openrouter_judge}.py`) | 3 (×2 worktrees) | **JUSTIFIED-TO-KEEP** (edge "sense") | The LLM judge is an *advisory edge sense*, explicitly **outside the deterministic core loop** (math-first §1.5.2). Correctly not-kernel. |
| **`dowiz-agentic-mesh/audit/*.py`** (test-login, verify-api, verify-ssr, health-check, …) | 14 | **GENUINE-GAP (throwaway)** | One-off deployment-audit scripts in a worktree. Disposable; delete-on-merge, not port. |
| **bebop recording tooling** (`record-*.py`, `render-cast.py`, `scripts/record-*.sh`, `three-model-review*.sh`) | ~8 | **JUSTIFIED-TO-KEEP** (dev demo/tooling) | asciinema/review harness; no product logic. |
| **`tools/eqc/{eqc,demo,test_eqc}.py`** (dowiz + agentic-mesh) | 3 (×2) | **ALREADY-PLANNED-FOR-PORT → now DELETE** | **Superseded by `tools/eqc-rs`** (§A). Retired, not CI-wired, not a codegen input. Safe to delete. This is the marquee item of the whole "scripts→0" directive and it is already resolved-pending-deletion. |
| **`dowiz-agentic-mesh/tools/loop-signals/markov_attractor.py`** (+ its test) | 2 | **ALREADY-PLANNED-FOR-PORT (done in dowiz)** | The spectral Faddeev-LeVerrier + Durand-Kerner core; `kernel/src/spectral.rs:1-27` states it **ports this proven Python core to zero-dep Rust**. The dowiz copy is **already gone** (MEASURED: `dowiz/tools/loop-signals/` has no `markov_attractor.py`); only the stale agentic-mesh worktree retains it. |

**Honest reading of the headline:** of ~315 sh+py files, **~230+ are vendored agent-
skill bundles + harness/CI/build glue** that are correctly `JUSTIFIED-TO-KEEP` or simply
out of scope for "core logic is Rust." The set the equations-not-primitives directive
*actually* targets — computational core logic living in a script — is a **small tail
(~15-20)**, and its single highest-value member (`eqc.py`) is **already superseded and
merely awaiting deletion.** The directive is far closer to met than the raw 315 implies;
the remaining genuine gaps are telemetry/self-improvement Python, not product-kernel
logic.

---

## (D) bebop2 procedural → equation candidates (proto-cap / proto-wire)

The task hypothesized "state-transition validity checks, quorum threshold math,
signature-batch accounting." **Reading the four files, the quorum-threshold hypothesis
does not hold — and that is itself a finding:** bebop2's mesh consensus is a **CvRDT
lattice-join (monotone set-union) + content-addressed idempotence + Merkle-root
convergence**, *not* quorum voting. There is **no threshold equation to extract** because
there is no threshold. (This matches the memory's Batch-2 finding: CvRDT set-union, HLC
rejected.) Concretely:

| file:line | what it is | closed-form / equation candidate? |
|---|---|---|
| `proto-cap/claim_machine.rs:72-94` `allowed_next` + `assert_transition` | Courier-claim FSM as a `match`-encoded adjacency — **identical pattern to `order_machine`** (row #9 above). `legal = A[from][to] ∧ from≠to` | **Procedural→declarative-DATA**, same as #9. A pinned adjacency table (DOD), *not* an eqc scalar equation. The recurring win is "one adjacency-table primitive shared by order_machine + claim_machine," not eqc-ification. |
| `proto-cap/revocation.rs:94-98` `merge` | `revoked_keys ∪ other; revoked_cap_hash ∪ other` — monotone set-union | **Join-semilattice (CRDT)**, correct by construction. Not numeric; no closed form to extract. Keep imperative. `revocation_hash` (line 129) = SHA3 over TLV — crypto, not eqc. |
| `proto-wire/discovery.rs:33-39` `fnv1a` + `70-104` `merge`/`snapshot_root` | FNV-1a hash fold (XOR·mul), directory set-union, hash fingerprint | **NO.** FNV is a GF(2)/mul bit-fold (like SHA3, row #10) — outside `±·`. `merge` is again a lattice-join. No quorum math present (full-roster anti-entropy, not voting). |
| `proto-wire/sync_pull.rs:457-481` `MerkleLog::root` | recursive pair-hash `sha3_256(L‖R)` tree | **NO** — hash-tree reduction; the "math" is the tree + SHA3. Correctly imperative. |
| `proto-wire/sync_pull.rs:575-621` `pull` / `ingest` | watermark filter `f.seq > last`; `IngestResult{added,dup,rejected}` counters | The **"signature-batch accounting"** the task named — but it is just loop-incremented counters satisfying the **conservation invariant** `added+dup+rejected == frames.len()`. That invariant is worth asserting as a test, but it is not an eqc scalar function. Ed25519 verify (`verify`, line 338) is crypto, not eqc. |

**bebop2 verdict (JUDGED):** the mesh/consensus/capability layer is fundamentally
**combinatorial-algebraic** (FSMs, join-semilattices, hash trees, signatures) — the
domain of *type-level correctness-by-construction*, which the code already achieves
(monotone unions, content-addressed idempotence, pinned discriminants, strict canonical
TLV codecs). It is **not** the domain of eqc (scalar arithmetic). The only cross-cutting
"procedural→declarative" opportunity here is the **shared FSM-adjacency-table primitive**
(order_machine + claim_machine both hand-roll the same `match`+`assert_transition`
shape) — a DOD consolidation, not an equation. Forcing eqc onto this layer would be a
category error, and (per the arc's rules) there is no hard-correctness reason to, so the
honest label is **ADOPT the shared-table primitive; the rest stays as-is (type-enforced,
not equation-enforced).**

---

## Prioritized build-order (smallest / highest-leverage first)

0. **Delete the retired Python eqc trio** `tools/eqc/{eqc,demo,test_eqc}.py`
   (+ `requirements.txt`, `__pycache__`) in dowiz, and the stale copy in
   `dowiz-agentic-mesh/tools/eqc/`. *Smallest possible change; closes the operator's
   explicitly-flagged contradiction; both delete-preconditions (not CI-wired, not a
   codegen input) are PROVEN met.* (~5 files.)

1. **Extend eqc-rs with `asin`/`atan2` nodes + a `checked_mul`/overflow-guard emission
   mode.** Small, additive; unblocks the two highest-value organ conversions (geo trig +
   money). Both are already on `eqc-rs/README.md:95-99`'s own roadmap.

2. **Generate `geo::ema_next` via eqc-rs + emit the parity `#[test]` beside it**
   (row #1). The *template* organ: exact affine, zero blockers, proves the
   author-equation → generate → commit → parity-gate loop end-to-end on a real kernel
   fn. This is the concrete S0.5/S1 "garage proof-of-concept" the math-first blueprint
   asks for, done on the cleanest possible target.

3. **Replace `order_machine::spectral_radius()`'s 1000-iter power iteration with a
   proven `const` (ρ=0)** (row #8). Kills a runtime loop that rediscovers a theorem
   every call; the golden-signature gate keeps it honest. Not eqc, but the single
   clearest "primitive computation that should not run at all."

4. **Consolidate the duplicated 2×2 eigenvalue closed form** (`householder.rs:224-229`
   ≡ `246-250`, row #3) into one shared `eig2x2` helper. Removes a verbatim-duplication
   drift hazard (the exact class math-first exists to kill), independent of eqc.

5. **Generate the money law** (`domain::compute_order_total` / `money::apply_tax`,
   row #2) fixed-point via eqc-rs, *after* the S2 integer-basis-points change removes the
   last `tax_rate: f64`. Red-line — operator-gated. Highest product value; correctly last
   because it touches money.

6. **Shared FSM-adjacency-table primitive** (rows #9 + claim_machine) — one pinned
   adjacency table + `assert_transition` reused by `order_machine` and
   `proto-cap::claim_machine`. DOD consolidation, not eqc.

7. **(S6, longer horizon) Equation-IR + regenerate-and-diff CI gate** so a committed
   organ that diverges from re-generation fails CI — "drift becomes impossible, not
   merely detected." Only worthwhile once ≥3 real organs (steps 2, 5) flow through
   eqc-rs.

**One-line closing (JUDGED):** the arc's "equations not primitives / no python" rule is
*much further along than the memory implies* — the compiler is already Rust, the Python
is dead-pending-deletion, and the real work is the unglamorous middle: routing 2-3
genuinely-scalar kernel organs (ema, money, haversine) through eqc-rs with parity gates,
while honestly declining to force eqc onto the iterative solvers, GF(2) crypto, and CRDT
mesh layer where it is a category error.
