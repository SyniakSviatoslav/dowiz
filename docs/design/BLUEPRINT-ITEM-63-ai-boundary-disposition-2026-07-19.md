# BLUEPRINT — Item 63: Item-45 Spec Extension — AI-Boundary Disposition Table + Build-Provenance Record + Feature-Matrix Legs

- **Date:** 2026-07-19 · **Tier:** spec-level now (roadmap §K, item 63) · **Status:** BLUEPRINT
  (planning artifact, no code) — the table + provenance record are **spec-level now; teeth when item
  45 lands** (the `inference` feature gate). Audit-3 P3's reject-list is endorsed as correct, not a
  deferral.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 63
  (lines 1081–1100) + item 45 (lines 640–653, the host — **no separate item-45 blueprint file
  exists**, confirmed by glob; item 63 amends item 45's spec in the roadmap-arc tracking, not the
  roadmap file itself);
  `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` §2.3 (P2/P4/P5 adopted; P1 recommendation;
  P3 reject-list); ground-truth code headers: `kernel/src/{micrograd,online,attention,evals}.rs`,
  `kernel/src/ports/llm.rs`, `kernel/src/agent/loop.rs`, `engine/src/voice.rs`.
- **Prerequisites:** the disposition table + provenance record are **READY NOW** (spec-level); the
  *gate teeth* wait for **item 45** (the `inference` feature). The feature-matrix CI legs (c) activate
  "once the flag exists" (item 45).

---

## 1. Scope & goal

**Goal.** Item 45 establishes the AI-optional compile-time invariant (inference behind a non-default
`inference` feature; no core→AI dependency). But item 45's spec is **silent** on a set of already-
existing modules that straddle the AI boundary. Item 63 (a) writes the **disposition table** that
classifies each, (b) adds a **build-provenance FDR record** naming the compiled feature set, and (c)
adds **feature-matrix CI legs** so the AI-absent build stays green forever, not only at gate-landing.

**Non-goals.**
- NOT building item 45's gate (that is item 45); item 63 *extends its scope clause* and pre-loads the
  table it will enforce.
- NOT moving any module behind `inference` yet (that is items 44/45's build); item 63 *classifies*.
- NOT a runtime kill-switch / dual-binary / AI-health monitor (item 45's explicit over-design guard;
  item 63 inherits it).

**Why "undefined = grandfathered leak."** An unclassified straddling module is the dangerous state:
when item 45's gate lands, an un-named module is neither gated nor asserted-core, so a core→AI
reference through it is invisible. Item 63 forces every straddling module into exactly one of three
classes so the gate's grep can tell a legal seam from a violation.

## 2. Current-state grounding (the eight straddling surfaces)

Confirmed from the module headers this session:

| Module | Grounded description | Seed disposition (executor confirms against source) |
|---|---|---|
| `kernel/src/attention.rs` | "scaled dot-product attention as ONE learned-affinity diffusion step"; `softmax(QKᵀ/√d)·V`, deterministic, **no persisted learned weights in the module** (`:1–14`) | **CORE-DETERMINISTIC** — it is math (the roadmap's own ruling). Confirm: no embedded learned weights; Q/K/V are inputs. |
| `kernel/src/micrograd.rs` | reverse-mode autodiff (`Value = Rc<RefCell<ValueData>>`), pure/deterministic, no vendor runtime; substrate for the B3 online learner **and** the eqc equation-IR autodiff / capture-field fits (`:1–14`) | **AI-EDGE candidate** (autodiff = training-shaped) **with a dual-use flag** — it also serves non-AI eqc/capture math. Executor must decide explicitly (see §7); *undefined is the forbidden state*. |
| `kernel/src/online.rs` | deterministic online learner (`LinearSGD`, `ScalarAdam`) built on the micrograd tape; local-first by construction (`:1–14`) | **AI-EDGE candidate** — this is on-node *learning*; the clearest "moves behind `inference`" case. |
| `kernel/src/evals.rs` | metamorphic benchmark generation + scoring; kernel-primitive oracles; a `mint_semantic` seam takes an **injected `&dyn LlmBackend`** (`:1–14`) | **CORE-DETERMINISTIC + SANCTIONED-SEAM** — the generators/oracles are deterministic kernel math; the `&dyn LlmBackend` injection is a legal always-compiled trait seam. |
| `kernel/src/ports/llm.rs` | trait-only port (`ChatResponse`, `chat`, `EmbedRequest`, …); always compiled; concrete impl downstream (`:110–374`) | **SANCTIONED-SEAM** — the syscall-interface shape; named legal so the gate distinguishes a seam from a violation. |
| `kernel/src/ports/agent/` | capability/scope traits (`RedLinePolicy::DenyByDefault`, `SignatureVerifier`, …) | **SANCTIONED-SEAM** — trait-only authority contracts, always compiled. |
| `kernel/src/agent/` | the bounded plan→act→observe executor (`agent/loop.rs`); defines only the abstract `AgentReasoner` contract; concrete LLM impl downstream (compile firewall per CLAUDE.md) | **SANCTIONED-SEAM** — abstract contract + bounded executor; names no kernel mutation, no concrete model. |
| `engine/src/voice.rs` | `WakeWordSpotter` + `AsrModel::feed` (ASR **inference**); `InferError`; the "battery lever" (`:5,111,121–132`) | **AI-EDGE** — voice inference; the gate's scope clause **must extend to the engine's voice/inference firewall (currently outside it entirely)**. |

Item 45's grounding (the host): roadmap lines 640–653 — inference behind a non-default `inference`
feature; CI job (a) default-features build (AI absent) compiles + passes the FULL suite; (b)
dependency-direction check — no core decision module (`order_machine`, `decision/`, `hydra`,
`event_log`, `markov`, `spectral`, `fdr`) references AI module paths outside the gate. Item 63's table
tells that check which of the straddling modules are *core* (must not be referenced by AI outside the
gate — wait: the reverse), *AI-edge* (move behind the flag), or *seam* (legal always-compiled).

## 3. Implementation plan (numbered)

1. **(a) Write the disposition table** (§2) into item 45's spec (the roadmap-arc tracking / item 45's
   forthcoming blueprint) **and into each named module's own doc header** — each module states its
   class in-source so the classification travels with the code. Every one of the eight is exactly one
   of `CORE-DETERMINISTIC | AI-EDGE | SANCTIONED-SEAM`; *unclassified is forbidden* (the grandfathered
   leak). Resolve the `micrograd` dual-use flag explicitly (§7).
2. **(a) Extend the gate's scope clause** to the engine's `voice`/`inference` firewall — item 45's
   dependency-direction check currently covers the kernel only; item 63 adds the engine surface
   (`engine/src/voice.rs` and any engine `inference` module) so an engine core→voice-inference leak is
   also caught. The grep must recognize `SANCTIONED-SEAM` modules (trait-only ports) as legal so a
   seam is not flagged as a violation.
3. **(b) Build-provenance FDR record.** Emit ONE startup `Kind::Event` FDR record naming the compiled
   feature set (`inference` on/off, `pq`, `telemetry`, `slot-arena`, …) — forensics can tell an
   AI-absent binary from an AI-present one **from the flight recorder alone**. This reuses the
   existing `Kind::Event` (`fdr/schema.rs:186–208`) + `fdr::event!`; the feature flags are compile-time
   `cfg!(feature = …)` booleans recorded as `fields`. **Pairs with item 48's heartbeat** (the startup
   record is the first heartbeat's provenance context).
4. **(c) Feature-matrix CI legs.** Once item 45's flag exists, add CI legs: `default` AND
   `default+inference` each compile + run the FULL suite on every PR — so the **absent leg stays green
   forever**, not only at gate-landing (the zero-dep-gate/toolchain-bump-gate precedent shape). Per
   the repo build model (no workspace), each leg is `cd kernel && cargo test` (and the engine leg
   `cd engine && cargo test`) with the feature set toggled.
5. **Record the P1 dispatch recommendation.** Audit-3 P1 — "dispatch item 45 now, it is READY-NOW and
   converts safe-by-convention into safe-by-gate **before** items 33–44 create real risk" — is
   recorded here as an **operator-dispatch recommendation** (not an item-63 action). Item 63's own
   teeth still wait for item 45; the recommendation is to sequence item 45 early.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 63 |
|---|---|
| 1. **Oracle** | **The table recorded** in item 45's spec + the named modules' docs (documentary, spec-level). **A planted `core→AI-EDGE` reference is RED under the extended gate** (P7 — the gate's own re-executed proof, lands with item 45). **The provenance record is recovered from a real ring in a test** (the `fdr/ring.rs` readback oracle — a test asserts the startup record names the compiled features). **Both matrix legs green in CI when the flag exists.** |
| 2. **Dudect** | **N/A** — a feature-classification + provenance record; no secret-timed code. |
| 3. **Debug cross-check** | **N/A** — classification is a documentary/gate property, not per-call arithmetic. |
| 4. **ASM spot-check** | **N/A** — no branch-free hot path. |
| 5. **Kani/formal** | **N/A** — the property is "every module classified, the gate catches a planted leak, the provenance record recovers," oracle-class. |

**Anti-forgery / anti-grandfather clause (the load-bearing proof):** the extended gate must go RED on
**a straddling module with no class** (the exact "undefined = grandfathered leak" the item names), not
only on a core→AI reference. Demonstrate: add an unclassified AI-shaped module reference → gate RED;
classify it → GREEN. Until item 45's gate exists, this is the recorded obligation on item 45.

## 5. Falsifiable acceptance criteria

1. Every one of the eight straddling modules carries exactly one of `CORE-DETERMINISTIC | AI-EDGE |
   SANCTIONED-SEAM` in the disposition table **and** its own module doc; no module is unclassified.
2. `micrograd`'s dual-use is resolved by an explicit ruling (not left grandfathered).
3. The gate's scope clause names the engine `voice`/`inference` firewall (extending item 45's
   kernel-only scope).
4. A startup `Kind::Event` FDR record names the compiled feature set; a test **recovers it from a
   real ring** and asserts the feature names.
5. When item 45's flag exists: a planted `core→AI-EDGE` reference (or an unclassified AI-shaped
   module) turns the extended gate RED (P7); both `default` and `default+inference` legs are green.
6. `cargo tree -e no-dev` byte-unchanged (the provenance record uses existing FDR machinery; no dep).

**Falsifier:** any straddling module left unclassified; the engine voice firewall still outside the
gate's scope; a provenance record that cannot be recovered from the ring; a matrix leg that only runs
at gate-landing (not every PR); a `SANCTIONED-SEAM` port flagged as a violation.

## 6. Dependency gates

- **Table + provenance record (a)(b):** **READY NOW** (spec-level) — documentary classification +
  an FDR record via existing `Kind::Event` machinery. No prerequisite.
- **Gate teeth + matrix legs (c):** gated on **item 45** (the `inference` feature must exist for the
  dependency-direction gate and the `default+inference` CI leg). Item 63 pre-loads item 45's scope;
  the enforcement is item 45's job.
- **Pairs-with:** **item 48** (heartbeat) — the provenance record is the first heartbeat's context.
- **Coordination:** items 44/45 (the inference subsystem + gate). Item 63's classification of
  `micrograd`/`online` as AI-EDGE candidates is the input to item 44/45's decision on what moves
  behind `inference`.
- **Recommendation (not a gate):** dispatch item 45 early (audit-3 P1), recorded for the operator.

## 7. Operator-decision points & accepted risks

- **[OPERATOR] `micrograd` dual-use ruling.** `micrograd.rs` is autodiff (training-shaped ⇒ AI-EDGE)
  but ALSO serves non-AI eqc equation-IR autodiff and capture-field fits (`:8–9`). Classifying it
  `AI-EDGE` (behind `inference`) would gate the eqc/capture math too; classifying it
  `CORE-DETERMINISTIC` leaves a training-capable engine in the always-compiled core. The honest
  options: (i) split the AI-training uses behind `inference` while keeping the pure autodiff primitive
  core; (ii) classify the whole module `CORE-DETERMINISTIC` (it is dependency-free deterministic math)
  and rely on item 45's *dependency-direction* check to stop core→AI wiring. Flagged for operator
  ruling — it decides how much of the growth-substrate math is AI-gated. **Owner:** operator.
- **[OPERATOR] Dispatch item 45 early (audit-3 P1).** Recommended so safe-by-convention becomes
  safe-by-gate before items 33–44 create real AI-inference risk. This is a sequencing decision.
  **Owner:** operator.
- **[ACCEPTED] Spec-level teeth deferred.** Item 63's gate enforcement genuinely waits for item 45;
  the table + provenance record land now (they are useful independently — the provenance record works
  the moment it ships, feature flags or not). This is a real, honest split, not a hidden dependency.
  **Owner:** arc lead.
- **[ACCEPTED] `evals` mixed class.** `evals.rs` is CORE-DETERMINISTIC with a SANCTIONED-SEAM
  injection (`&dyn LlmBackend`); a module carrying two classes is recorded as such (the deterministic
  core + the named legal seam), not forced into one. **Owner:** arc lead.
