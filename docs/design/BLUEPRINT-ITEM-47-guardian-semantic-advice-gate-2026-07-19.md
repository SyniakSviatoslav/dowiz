# BLUEPRINT — Item 47: Guardian — semantic advice gate + deterministic-primary path

- **Date:** 2026-07-19 · **Tier:** 1-class (safety seam) · **Status:** BLUEPRINT (planning artifact,
  no code) · **Arc:** §I "Whole-System Determinism & AI-Optional Arc".
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I item 47
  (lines 683–704), item 40 (plane distinction), item 9 (breaker precedent), item 42 (bounded-loop
  source-structure assertion); `docs/audits/hardening/CHECKLIST.md`;
  `CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md`. Code ground truth:
  `kernel/src/decision/import.rs` (the named precedent), `kernel/src/ports/agent/admission.rs` (name
  collision), `kernel/src/ports/agent/scope.rs` (RedLine deny-by-default).
- **Dependency status:** **GATED.** Spec after item 35 (the AI proposal-schema seam — NOT yet
  built); full wiring after item 42; EXTENDS item 9 (breaker — composes, does not gate);
  cross-references item 40. This blueprint is the *spec skeleton* that item 35's landing unblocks; it
  invents no invariant content (§10).

---

## 1. Problem + non-goals

### Problem
When the AI inference subsystem (items 33–44) produces advice, that advice must never be able to
drive the system into an unsafe state, and the system must remain fully functional when advice is
absent, crashed, or rejected. Item 47 makes AI-optional a property expressed **in the type system**:
the kernel's decision seam takes `Option<Proposal>`, advice is DATA, `None` is a first-class tested
input, and the deterministic path is the *total function* — "the fallback IS the system".

### Non-goals
- **NOT** a competing breaker or a fold-in of item 40/9 — item 47 rejects well-formed-but-unsafe
  MEANING; item 40 rejects corrupted BITS (hardware-fault evidence); item 9 is the trip/latch. They
  compose; none is forked (roadmap lines 693–697).
- **NOT** WCET tooling — loop bounds are asserted by source structure (item-42 style), not measured.
- **NOT** inventing the invariant set — the actual safety laws depend on item 35's `Proposal` schema
  (§10, operator-decision).
- **NOT** a full formal proof of the invariants — CHECKLIST-style exhaustive/differential + proptest,
  per the synthesis §5 proportionality ruling (BITE/runtime-verification primary; Coq/Lean out of
  scope for the arc).

## 2. Current-state grounding (verified this session)

### 2.1 The named precedent to EXTEND — `import_unit` (roadmap line 698–699)
`kernel/src/decision/import.rs::import_unit` (`decision/import.rs:81`) is the same shape at *import*
granularity: a verify-before-persist gate that admits a foreign artifact into `Live` only after an
**independent replay** — "the author-hub's own GREEN is never the certificate" (`import.rs:12–13`),
"ANY disagreement ⇒ reject" (`import.rs:53–54`), "on any reject, **nothing is persisted**"
(`import.rs:78–80`, degrade-closed). Item 47's `admit` is that shape at *proposal* granularity: check
the advice against invariants; on any violation reject and fall back to the deterministic baseline;
persist/act nothing that was not admitted. **Extend this shape; do not fork a parallel type.**

### 2.2 The illegal-state-unrepresentable precedent (roadmap "item-9 `Result<Permit, Tripped>`")
`ValidatedProposal` must be constructible ONLY through `admit` — the item-9 breaker's
`Result<Permit, Tripped>` standard (a `Permit` cannot be forged; it is minted only by the gate). The
kernel already uses this idiom pervasively (the order FSM's forbidden-transition-as-error,
`RedLinePolicy::DenyByDefault` at `ports/agent/scope.rs`).

### 2.3 NAME-COLLISION FLAG — there is already an `admit` in the kernel
`kernel/src/ports/agent/admission.rs:394` defines `Admitter::admit(...)` — the **B1 mesh-capability**
admission (parse-don't-validate: strict TLV decode, `HybridGate::check`, identity binding, sandbox
tier, budget envelope; typed `AdmissionError`, fail-closed at `admission.rs:74–110`). That is the
*capability* plane. Item 47's Guardian `admit(Proposal, &Invariants)` is the *AI-advice* plane — a
**distinct seam**. They share the parse-don't-validate / typed-rejection / fail-closed STYLE (which
is the precedent to extend), but item 47 must NOT reuse the name/type. Recommend a distinct module,
e.g. `kernel/src/ports/agent/guardian.rs` with `guardian::admit` → `Result<ValidatedProposal,
Rejection>`, so a reader never confuses the advice gate with the capability gate.

### 2.4 The FDR event surface exists (for the "every Rejection emits an FDR event" clause)
`fdr::warn!`/`fdr::event!` macros are live (`fdr/mod.rs:59–62`); an `Alarm`-class record fsyncs
(`fdr/ring.rs:134`). Item 47's rejection-emits-FDR clause has a real sink to write to (when the FDR
branch is merged — see item 48's dependency note).

### 2.5 The bounded-loop assertion precedent (item 42)
Every loop in the Guardian's static procedures is `0..MAX_N` with a source-structure assertion (a
test that greps/parses for an unbounded loop in the module), the same discipline item 42 defines for
inference kernels. No `while let` on an unbounded stream in the advice path.

## 3. Options considered (≥2)

**Option A — `Option<Proposal>` seam with a total deterministic function + admitted-nudge
(RECOMMENDED, this is the roadmap's design).**
The decision seam is `decide(inputs, Option<Proposal>) -> Decision`. `None` → the deterministic
baseline (total function). `Some(p)` → `guardian::admit(p, &invariants)`; `Ok(ValidatedProposal)` is
applied as a bounded nudge on top of the baseline, `Err(Rejection)` → the baseline is used AND an FDR
event is emitted.
- Concept: *parse-don't-validate + AI-as-optional-data* — advice can only ever act through the
  checked gate; the deterministic path is authoritative.
- Tradeoff: requires the baseline to be genuinely total (defined for every input without advice) —
  which is the point. The `None`-path must be bit-identical to a no-AI build (testable, §7).

**Option B — advice-required with a fail-open default.**
Treat advice as an input the decision needs, with a permissive default when absent.
- Concept: *AI-in-the-loop*.
- Tradeoff: **rejected** — it inverts the invariant (system depends on AI), violates the governing
  directive ("система повинна здатна працювати без AI"), and makes `None` a degraded rather than
  first-class path. Recorded only to show it was considered and why it loses.

## 4. Decision + rationale (ADR-format)

**ADR-047: Option A — `Option<Proposal>` seam; deterministic path is the total function; advice
admitted only through `guardian::admit`.**

Rationale: this is the only design where AI-optional is a *type-system* property rather than a
runtime promise. `None` being first-class and the baseline being total means "AI absent/crashed/
rejected" all collapse to the same tested, authoritative path. `ValidatedProposal` constructible only
via `admit` makes "acted on un-admitted advice" unrepresentable (item-9 standard). It extends the
proven `import_unit` verify-before-trust shape rather than inventing a parallel gate, and it composes
cleanly with item 40 (bits) and item 9 (breaker) on separate planes. Distinct from item 40 by plane;
distinct from `ports/agent/admission.rs::admit` by name and domain.

## 5. Implementation plan (numbered — spec-level, unblocked by item 35)

1. **Seam types** (in `kernel/src/ports/agent/guardian.rs`, distinct from `admission.rs`):
   - `Proposal` — opaque advice DATA (concrete fields defined WITH item 35's schema; §10).
   - `Invariants` — a table of checkable equations, each a NAMED pure `fn(&Proposal) -> Result<(),
     Rejection>`; the "`result.velocity < MAX_SAFE_SPEED`" class is the *example* shape, not the real
     set.
   - `ValidatedProposal(pub(self) …)` — private constructor; the ONLY mint is `admit` (item-9
     `Permit` standard).
   - `Rejection` — closed typed enum (one variant per invariant + a decode/shape variant),
     `Debug + Clone + PartialEq + Eq` (the `AdmissionError` shape at `admission.rs:74`).
2. **The gate:** `pub fn admit(p: Proposal, inv: &Invariants) -> Result<ValidatedProposal,
   Rejection>` — runs each invariant in a fixed `match`-based order (order_machine style), static
   dispatch, every loop `0..MAX_N`. First failing invariant short-circuits to its `Rejection`.
   Parse-don't-validate: a `ValidatedProposal` is proof every invariant held.
3. **The seam:** the decision entry takes `Option<Proposal>`. `None` → deterministic baseline
   (total). `Some(p)` → `admit`; `Ok` applies the bounded nudge, `Err` → baseline + one FDR event
   (`fdr::warn!` carrying the `Rejection` variant + a proposal fingerprint, NEVER PII/menu-only per
   the AI red-line).
4. **FDR-on-reject + breaker composition:** every `Rejection` emits one FDR event. When item 9's
   breaker exists, repeated rejections route through it (the SAME composition clause as item 40 —
   design does NOT gate on item 9; the emit is unconditional, the routing is additive).
5. **Bounded-loop source-structure assertion** (item-42 style): a test asserting no unbounded loop
   exists in the guardian module's static procedures.
6. **Feature placement (see §10):** the `Proposal`/`Invariants`/`ValidatedProposal`/deterministic-
   baseline types compile in the **default (AI-absent) build** so the `None`-path is always the
   system (item 45); only the *production of a `Some(Proposal)`* lives behind the `inference` feature.

## 6. Failure + degradation (failure-first)

- **Advice absent (`None`):** deterministic baseline, bit-identical to a no-AI build. This is the
  designed primary path, not a fallback.
- **Advice crashed / malformed:** surfaces as `None` (the inference subsystem's failure is caught at
  its boundary; the seam never sees a partial `Proposal`) → baseline.
- **Advice well-formed but unsafe:** `admit` returns `Err(Rejection)` → baseline + FDR event. No
  cascade: a rejected proposal changes nothing about the decision, only emits a record.
- **Every external call** (the inference produce-advice call) has: a timeout at the inference
  boundary (item 42/40's concern), and a `None` fallback here. Zero cascade into the decision.

## 7. Required tests / proofs (per CHECKLIST.md 5-point standard + roadmap 700–704)

1. **Oracle:** exhaustive enumeration where the advice domain is enumerable; otherwise an
   oracle/differential corpus + a **proptest sweep** — the item-5 regex-parity testing stack, reused
   not reinvented. The invariant spec doc writes every law as a checkable equation (the oracle's
   reference form).
2. **dudect gate:** N/A — advice is public data (menu-only per the AI red-line), no secret-dependent
   timing. Record `N/A(public-advice-input)`.
3. **Debug cross-check:** the `None`-path bit-identity test IS the differential cross-check — assert
   `decide(inputs, None)` is byte-identical to the no-AI deterministic baseline for a corpus of
   inputs.
4. **Assembly spot-check:** N/A — not a branch-free crypto path. Record `N/A(not-CT)`.
5. **Structural:** the bounded-loop source-structure assertion (item-42) green; the
   `ValidatedProposal`-only-via-`admit` property proven by the private constructor (a test that
   cannot construct one out-of-band = compile-time).

**Falsifiable acceptance criteria (roadmap 700–704):**
- The invariant spec doc with every law as a checkable equation.
- **Planted-invalid-advice red→green:** a proposal violating an invariant is demonstrably REJECTED
  (RED absent the gate — P7).
- **`None`-path bit-identity:** output is bit-identical vs the deterministic baseline over the corpus.
- Exhaustive-where-enumerable + differential corpus + proptest sweep all green.
- Every `Rejection` emits an FDR event (assert the record appears in the ring).
- The bounded-loop source-structure assertion green.

## 8. Security + tenant isolation

- **AI red-line:** the `Proposal` and every FDR reject record are **menu-only / zero-PII** (the
  standing AI-input rule). The proposal fingerprint in the FDR event is a hash, never raw content.
- **No privilege via advice:** a `ValidatedProposal` grants no capability — it is a bounded nudge on
  a decision the deterministic baseline already authorized. Money/auth/migration red-lines remain
  denied-by-default (`ports/agent/scope.rs`); the Guardian cannot admit a proposal that touches them
  (an invariant enforces it).
- **Tenant isolation:** unchanged — the Guardian sits on the decision seam, downstream of tenant
  scoping.

## 9. Operability

- **Health (degraded-vs-down):** advice-rejected/absent is *degraded-but-nominal* (baseline runs);
  the system is never "down" for lack of AI. FDR reject-rate is the observability signal.
- **Observability (<1 min):** each rejection is one FDR event with the `Rejection` variant; a
  rejection spike is visible in the ring/`alert.jsonl`.
- **Rollback:** the `inference` feature is off-by-default (item 45); disabling it reverts to the pure
  `None`-path with zero code change.
- **Flag/scaling gate:** the produce-advice path is behind `inference`; the gate itself is always
  compiled (so `None`/reject paths are always tested).

## 10. Open / accepted risks + operator-decision points

- **[HARD GATE] Spec depends on item 35.** The `Proposal` schema (and therefore the real
  `Invariants` set) is item 35's deliverable. This blueprint is the *skeleton*; the concrete invariant
  equations CANNOT be written until item 35 lands. **Item 47 does not start its spec before item 35.**
  *Owner: item-35 lead → item-47 executor.*
- **[OPERATOR-DECISION] The invariant set.** "`result.velocity < MAX_SAFE_SPEED`" is an example
  class. The actual safety laws (what advice may/may not nudge, the hard bounds) are a product +
  safety decision, defined with item 35 — NOT invented here. *Owner: operator + item-35 lead.*
- **[OPERATOR-DECISION] Feature placement of the seam.** Recommendation (§5.6): the
  `ValidatedProposal`/`Invariants`/deterministic-baseline types live in the DEFAULT (AI-absent)
  build so the `None`-path is always the system; only advice *production* rides `inference`. Confirm
  this split. *Owner: operator + item-45/47 leads.*
- **[FLAG] Name collision.** Do NOT name the seam `admit`/`Admitter` at module top level — that
  collides with `ports/agent/admission.rs:394`. Use `guardian::admit` in a distinct module. *Owner:
  item-47 executor.*
- **[COMPOSES, does not gate] Item 9 breaker + item 40.** The FDR-on-reject emit is unconditional;
  the breaker routing is additive when item 9 exists; item 40 handles the bits plane. Recorded as
  design that does not block on either. *Owner: item-47 executor.*
