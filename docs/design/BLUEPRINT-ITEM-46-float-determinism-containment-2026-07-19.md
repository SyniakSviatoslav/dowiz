# BLUEPRINT — Item 46: float-determinism containment, evidence-scoped

- **Date:** 2026-07-19 · **Tier:** 0/1-class (evidence-scoped hardening) · **Status:** BLUEPRINT
  (planning artifact, no code) · **Arc:** §I "Whole-System Determinism & AI-Optional Arc".
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I item 46
  (lines 654–682); `docs/regressions/REGRESSION-LEDGER.md` row 25 ("Layer A — integer-CORDIC
  sin/cos", lines 169–173); `docs/audits/hardening/CHECKLIST.md`;
  `CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md` §2.3 (non-rewrite ruling).
  Code ground truth: this worktree.
- **Dependency status:** READY NOW. Composes with item 14's closed toolchain-bump gate (no hard
  prerequisite). Gates no other item.

---

## 1. Problem + non-goals

### Problem
The directive requires 100%-predictable, mathematically-deterministic behaviour. Money is already
integer-exact (MANIFESTO C2; `kernel/src/money.rs`), but the wider kernel carries **1043 `f64`/`f32`
occurrences across 69 files** (measured this session). The determinism question is narrow and
specific: *which float sites can produce a different bit pattern on a different toolchain or host,
and do any of them feed a comparison surface that would then diverge?*

### Non-goals (explicit — roadmap lines 654–658, synthesis §2.3)
- **NOT a kernel-wide `f64`→fixed-point rewrite.** Rejected as disproportionate. The synthesis
  ruling: the ONE real float-nondeterminism bug ever shipped was libm `sin`/`cos` ULP drift
  (`REGRESSION-LEDGER.md` row 25), and basic IEEE-754 arithmetic (`+ − × ÷`, and `sqrt` —
  correctly-rounded) is **bit-deterministic for a fixed binary on the pinned toolchain**. A blanket
  rewrite optimizes a non-problem.
- **NOT** touching money (already integer; out of the float plane entirely).
- The full fixed-point conversion is **parked** as an explicitly-flagged-LARGE item with named
  reopening triggers (§5), not built now.

## 2. Current-state grounding (verified this session)

### 2.1 The historical bug and its fix — where the CORDIC actually lives (accuracy correction)
`REGRESSION-LEDGER.md` row 25 (Layer A): *"`f64::sin`/`f64::cos` evaluated on two platforms (or
under `-ffast-math` / different libm) produce differing ULP; a digest pinned to that float result
is [non-reproducible]. GREEN — `cordic_sincos` is pure integer fixed-point (CORDIC rotation,
32-bit …)."*

> **ACCURACY CORRECTION (verified this session):** the roadmap calls this the "Q30 CORDIC". The
> integer-CORDIC `cordic_sincos` lives in **`tools/eqc-rs/src/cordic.rs`** — the eqc equation→Rust
> **compile-time codegen tool**, NOT a `kernel/src` runtime module (repo-wide grep: zero `cordic`
> hits under `kernel/src/`). Consequence for item 46: the kernel's **live runtime transcendental
> sites are NOT CORDIC-backed today** — they call `std`/libm directly. So "migrate-to-CORDIC-class"
> for a live site is *real work* (route the site through an integer replacement, possibly
> eqc-generated), not a status-quo. The executor should verify the exact Q-format in
> `tools/eqc-rs/src/cordic.rs` rather than assume "Q30".

### 2.2 The named deterministic-kernel plane (roadmap line 659–661) — per-site inventory
Item 46 scopes the inventory to `spectral.rs`, `markov.rs`, `token_bucket.rs`, `attention.rs`.
Transcendental call sites verified this session:

| Site (file:line) | Call | Role | Provisional disposition |
|---|---|---|---|
| `attention.rs:33` | `(x - m).exp()` | softmax over affinity row | pin-under-golden (dynamics/affinity, never money — `attention.rs:15`) |
| `markov.rs:73` | `(1.0/tol).ln() / (1.0/slem).ln()` | mixing-time bound | pin-under-golden (advisory metric) |
| `markov.rs:186` | `p * p.log2()` | Shannon entropy of a row | pin-under-golden (advisory metric) |
| `spectral.rs:55` | `self.re.hypot(self.im)` | Complex modulus | classify: if it feeds `classify_drift`/a golden signature → golden; else exempt |
| `spectral.rs:59` | `self.im.atan2(self.re)` | Complex argument | classify: same as `:55` |
| `token_bucket.rs:70–72` | `as_secs_f64()`, `refill_rate * elapsed` | wall-clock refill | **basic-arith-exempt** — no transcendental; wall-clock-driven ⇒ inherently non-deterministic by construction (degrade-closed), never a replay/comparison surface |

`attention.rs:13–15` already *claims* bit-reproducibility ("softmax subtracts the row max … sums in
a fixed order — bit-reproducible across native / wasm32"). That claim rests on libm `exp`
determinism for a fixed binary; item 46 either backs it with a golden test or reclassifies it —
**it is currently an unbacked in-code assertion**, exactly the kind scope (ii) exists to close.

### 2.3 Out-of-plane transcendental sites (inventoried for completeness, disposition = classify)
The inventory (scope i) must *classify* every transcendental site so none is unclassified; these sit
outside the named plane and are provisionally display/edge/analytic (not replay-comparison), but
each needs an explicit line:

- `geo.rs:19–35` — haversine/bearing `sin/cos/atan2` (routing/display, not authority).
- `field_eigenmodes.rs:163–178,395–396,600–601` — analytic `cos` Laplacian eigenmodes (UI render).
- `spectral_laplacian.rs:143` — `cos` analytic path eigenvalues.
- `micrograd.rs:137–160,231–288` — autodiff `sin/cos/exp/ln/powf/tanh` (edge learning; `attention.rs:19`
  says learning lives here, not in the kernel core).
- `online.rs:148–150,234` — sigmoid `exp` / `ln` (edge learning).
- `simd.rs:46,123` — softmax `exp` (mirrors `attention`).
- `retrieval/bm25.rs:207,349` — idf `ln` (retrieval ranking).
- `intake.rs:430` — entropy `ln`.
- `ports/customer.rs:452` — `2f64.powf(...)` brute-force-time security estimate (display).
- `householder.rs:786,841` — `sin` in *test fixtures only*.

### 2.4 The re-execution mechanism that makes this enforceable (verified precise — roadmap 663–669)
- The full-suite `cargo test` job is **always-on** and pinned via `rust-toolchain.toml`, and item
  6's `hardening-gate` re-runs oracles unconditionally. So a golden float value that diverges under
  a new compiler turns the bump PR **RED** — *once this item adds the missing golden coverage*.
- The `toolchain-bump-gate` itself only requires a `spot-check-<new>.md` presence artifact on a
  `channel` bump (CHECKLIST §10 "one sanctioned presence-check exception"); it is the always-on
  `cargo test` under the new pin that actually re-runs the goldens. Item 46's job is to make sure the
  in-plane float surfaces HAVE goldens in that always-on set.

### 2.5 Fleet-heterogeneity framing (roadmap 672–676)
Local-first mesh ⇒ heterogeneous peers replay each other's `DecisionUnit`s via
`decision/import.rs::import_unit` (`decision/import.rs:81`, "independent replay … author's GREEN
never trusted"). So the multi-ISA reopening trigger is evaluated against **fleet heterogeneity
(incl. aarch64 consumer devices)**, not a single-host assumption. Scope (ii)'s cross-host comparison
surfaces are the first line regardless of whether a rewrite is ever triggered — a value that crosses
`import_unit`'s replay boundary must be integer-domain or golden-covered.

## 3. Options considered (≥2)

**Option A — pin-under-golden now, migrate-to-CORDIC only on a triggered divergence (RECOMMENDED).**
Every in-plane transcendental site gets either an integer-domain reclassification or a golden test in
the always-on suite; the fixed-point rewrite is parked behind named triggers.
- Concept: *evidence-scoped containment* — the synthesis §2.3 proportionality ruling.
- Tradeoff: minimal code, immediate RED-provability, honest about the one real historical bug class.
  Does not eliminate libm from the runtime (accepted — basic IEEE + golden pins are sufficient for a
  fixed binary).

**Option B — migrate all in-plane transcendental sites to integer-CORDIC now.**
Route `attention` softmax `exp`, `markov` `ln`/`log2`, `spectral` `hypot`/`atan2` through
eqc-generated integer replacements immediately.
- Concept: *integer-domain everywhere in the plane* (the row-25 fix generalized).
- Tradeoff: eliminates the libm-drift class in the plane outright — but it is disproportionate work
  (`exp`/`log2`/`atan2` CORDIC kernels + parity oracles for advisory metrics), changes numeric
  results, and the synthesis already ruled a blanket rewrite out. Reserve for a *triggered* site.

## 4. Decision + rationale (ADR-format)

**ADR-046: Option A — pin-under-golden, park the rewrite behind named triggers.**

Rationale: the only float-nondeterminism bug that ever shipped was libm `sin`/`cos` ULP drift, and
it is already fixed at the codegen layer (`tools/eqc-rs/src/cordic.rs`). Basic IEEE-754 arithmetic is
bit-deterministic for the pinned binary. The proportionate move is to (i) prove there are no
*unclassified* transcendental sites and (ii) ensure every in-plane float value that could feed a
cross-version/cross-host comparison is either integer-domain or golden-covered — so any real
divergence turns CI RED under the pinned toolchain. Migrating an advisory metric like `markov`
entropy to CORDIC buys nothing (it feeds no replay surface); migrating a site that *does* cross the
`import_unit` replay boundary is the reopening trigger, not a today-task.

## 5. Implementation plan (numbered)

1. **Inventory doc (scope i):** produce `docs/audits/determinism/FLOAT-SITES-2026-07-19.md`
   enumerating every libm-transcendental call site (`sin/cos/exp/ln/powf/log2/atan2/hypot/tanh`;
   `sqrt` EXEMPT — correctly-rounded) in the named plane (`spectral.rs`, `markov.rs`,
   `token_bucket.rs`, `attention.rs`) with a per-site disposition of `migrate-to-CORDIC-class` OR
   `pin-under-golden` OR `basic-arith-exempt`/`comparison-surface-exempt`. Include the §2.3
   out-of-plane sites with a one-line plane classification each. **Acceptance: zero unclassified
   transcendental sites.**
2. **Comparison-surface audit (scope ii):** enumerate every value feeding a cross-version/cross-host
   comparison surface — golden signatures, oracle pins, `wire_code()`s, `DRIFT_BAND`-class constants,
   and (per §2.5) anything crossing `import_unit`'s replay boundary. Each must be **integer-domain OR
   golden-covered**. For `spectral.rs:55/59`: determine whether `Complex::modulus/arg` feed
   `classify_drift`/a golden signature (the item-7 blueprint records `spectral_radius` as the proven
   const `0.0` at `order_machine.rs:383`, so the FSM drift path may already be integer — verify).
3. **Add the missing golden float surfaces** so they sit in the always-on full-suite /
   `hardening-gate` oracle set (§2.4). Each new golden pins the exact bit pattern of the in-plane
   value under the pinned toolchain.
4. **Park the full fixed-point conversion** as an explicitly-flagged-LARGE item with named reopening
   triggers recorded in the inventory doc and the relevant module docs (`attention.rs`, `markov.rs`,
   `spectral.rs`): (a) a *reproduced* cross-version golden divergence in basic float arithmetic, or
   (b) a multi-ISA deployment requirement **evaluated against fleet heterogeneity incl. aarch64
   consumer devices** (§2.5).
5. **Record** the CORDIC-lives-in-eqc accuracy note (§2.1) in the inventory doc so a future reader
   does not assume the runtime is already CORDIC-backed.

## 6. Failure + degradation

- `token_bucket.rs` is a wall-clock-driven rate limiter — non-deterministic **by construction** and
  already degrade-closed (`token_bucket.rs:4` invariant: never over-grant; `saturating_*` clamps a
  backward clock to a zero refill). It is float-arithmetic but never a replay/comparison surface, so
  its non-determinism is correct, not a bug. Recorded as `comparison-surface-exempt`.
- A golden that fails under a toolchain bump is the *designed* RED — it converts a silent
  cross-version divergence into a blocked PR, which is the degradation goal.

## 7. Required tests / proofs (per CHECKLIST.md 5-point standard)

1. **Oracle:** the golden test for each in-plane float surface (scope ii coverage) — the pinned bit
   pattern is the oracle, re-executed by the always-on suite under the pinned toolchain.
2. **dudect gate:** N/A — these are public dynamics/metrics, no secret-dependent timing.
   Record `N/A(no-secret-input)`.
3. **Debug cross-check:** where a value is integer-domain after reclassification, a
   `debug_assert_eq!` against the integer form; for genuinely-float advisory metrics record
   `N/A(golden-oracle)`.
4. **Assembly spot-check on compiler bump:** covered by the existing `toolchain-bump-gate` +
   the `spot-check-<new>.md` `## Full-suite re-run` artifact — item 46 adds no new asm surface; it
   adds the goldens that the bump's full-suite re-run exercises.

**Falsifiable acceptance criteria:**
- The inventory doc exists with a per-site disposition and **zero unclassified transcendental sites**
  in the named plane.
- Every new in-plane golden sits in the always-on full-suite / `hardening-gate` oracle set; a
  deliberately-perturbed golden value turns CI **RED under the pinned toolchain** (red-proven,
  recorded in the PR).
- A `channel` bump is additionally gated on the `spot-check-<new>.md` `## Full-suite re-run`
  artifact.
- The parked rewrite + its two named reopening triggers are recorded in the inventory doc and the
  relevant module docs.

## 8. Security + tenant isolation

No tenant/PII/money surface. The relevant property is *determinism-as-integrity*: a value that
crosses a peer-to-peer replay boundary (`import_unit`) and diverges by a ULP on a different peer
would make a valid `DecisionUnit` replay-reject — a correctness/liveness fault, not a leak. Scope
(ii) closes exactly that: such a value must be integer-domain or golden-covered.

## 9. Operability

- **Observability (<1 min):** a diverged golden names the exact in-plane value + expected vs actual
  bits; the `hardening-gate` re-run surfaces it in the CI log.
- **Rollback:** goldens are test-only; reverting is a test-file revert. The inventory doc is a doc.
- **Scaling/flag gate:** none — no runtime code path added; the parked rewrite is the only future
  flagged work.

## 10. Open / accepted risks + operator-decision points

- **[OPERATOR-DECISION] Per-site disposition where migrate-vs-pin is genuinely a choice.** The
  recommendation is pin-under-golden for all four in-plane sites now, migrate only on a triggered
  divergence (ADR-046). If the operator wants any site *actually migrated* to integer-CORDIC now
  (e.g. `attention` softmax, if it is decided to cross the replay boundary), that is a real cost and
  a real numeric change — flag it, do not assume. *Owner: operator.*
- **[OPERATOR-DECISION / verify] `spectral.rs:55/59` classification.** Whether `Complex::modulus/arg`
  feed a golden signature or replay surface is a verification the inventory must settle; if they feed
  only advisory display, exempt; if they feed `classify_drift`/a signature, golden-cover. The
  item-7 note that `spectral_radius` is a proven const `0.0` (`order_machine.rs:383`) suggests the
  FSM drift path is integer, but the general `Complex` path must be checked. *Owner: item-46
  executor, confirm with operator if a migration falls out.*
- **[ACCEPTED RISK] libm remains in the runtime.** Item 46 does not remove libm from the plane; it
  pins the outputs. Accepted per synthesis §2.3 — basic IEEE + goldens suffice for a fixed binary;
  the multi-ISA case is the parked trigger. *Owner: item-46 executor.*
- **[FLAG] "Q30 CORDIC" nomenclature.** The roadmap's "Q30" is unverified against
  `tools/eqc-rs/src/cordic.rs` (ledger row 25 says "32-bit"); the executor pins the exact Q-format
  in the inventory doc. *Owner: item-46 executor.*
