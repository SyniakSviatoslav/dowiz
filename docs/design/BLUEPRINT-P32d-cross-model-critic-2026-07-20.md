# BLUEPRINT — P32d: Cross-model critic — a decorrelated, advisory-only check on control-loop decisions

- **Date:** 2026-07-20 · **Component:** CORE (P32d, absorbs the hydraulic-loop-v2 cross-model critic —
  the one item of the arc's 7 math corrections with no code at all) · **Status:** BLUEPRINT v1
  (planning artifact, no code changed by this pass). Converts P32d's standing flag ("**Blueprint:**
  none — needs a short design note first") into the short design note it called for: what gets
  critiqued (a concrete loop-output type), the decorrelation requirement, and the advisory-only
  integration point.
- **Sources read this session (verified against the live worktree, not memory):**
  `docs/design/ROADMAP.md` §10.5.1 P32d (the three DoD items this blueprint discharges/specifies);
  `kernel/src/markov.rs:42-50` (`pub enum Verdict { Healthy, LimitCycle, StrangeAttractor }`, the
  attractor detector's advisory loop-health decision, produced by `analyze_detailed:110`);
  `kernel/src/online.rs:28,82` (`LinearSGD`/`ScalarAdam` — the online control-loop parameter
  updaters, a second candidate loop-output); `kernel/src/ports/llm.rs:368-382` (`pub trait
  LlmBackend { fn chat … -> Result<_, LlmError> }`, fail-closed via `LlmError::Unavailable:170`);
  the **research-verifier** decorrelation precedent (`ROADMAP.md:768`, and the agent charter — a
  verifier run on a *different model/provider* than the thing it checks); the standing
  **GROUND-TRUTH-over-PROXY** rule (advisory signals never gate a deterministic decision) and item
  27's already-established "P6 preserved — `analyze_detailed`/`classify_drift` stay pure, telemetry
  is recorded output not classifier input" boundary (`BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md`).

---

## 1. Scope / goal

P32d is a **decorrelated multi-model check on control-loop decisions** — the mechanism behind the
hydraulic arc's own math-correction discipline, applied at runtime to a live loop output. It is
explicitly **distinct from the harness-level review agents** (those review code diffs; this
critiques *loop outputs*), and it is **advisory-only**: it surfaces disagreement as a signal, it
never gates a deterministic decision (GROUND-TRUTH-over-PROXY). This blueprint delivers P32d DoD
item 1 (the design note: ≥1 concrete loop-output type + decorrelation constraint + advisory-only
integration point) and fully specifies DoD items 2–3 (minimal implementation + RED-provability).

**Anti-scope (from the roadmap, restated so it binds):** do NOT build a general "AI council"
framework — one loop output, minimal voting, advisory only. Critic output must NEVER gate anything
deterministic (that would violate GROUND-TRUTH-over-PROXY and re-introduce the proxy-council this
repo already removed). Do not couple this to the AGENT phase's `LlmBackend`-wiring timeline: the
design note (this doc) proceeds now; the implementation may reuse AGENT's `LlmBackend` once it is
wired to consumers (soft dependency, §5).

## 2. What gets critiqued — the concrete loop-output type (DoD item 1a)

The critic needs a *typed, live, advisory* loop decision to check. The kernel already produces one:

- **Primary target — `markov::Verdict`** (`markov.rs:42`). `analyze_detailed(states)` classifies a
  tool-outcome / control-loop sequence as `Healthy` / `LimitCycle` / `StrangeAttractor` — an
  advisory loop-health decision that already fails open and gates nothing (the Markov attractor
  detector is advisory by construction). This is the ideal first critic input: it is already
  advisory (so an advisory second opinion on it changes no contract), it is a small closed enum (so
  "the two judges disagree" is unambiguous), and item 27 already brackets `analyze_detailed` with a
  telemetry station (`PmuStation::bracket`) — the exact integration seam a critic record rides on.
- **Second target (named, not built here) — a control-loop parameter update** from
  `online.rs::LinearSGD::step` / `ScalarAdam` (the learnable substrate P31e will feed) or the
  bebop-repo hydraulic governor's PID gain adjustment. Same critic shape; deferred so rung 1 stays
  one output.

The critic asks the decorrelated judges a **bounded, checkable** question about the primary output —
not "is the loop good" but: *"Given this exact state window and the detector's own definitions
(trapped / low-entropy / spectral-oscillation), is `StrangeAttractor` the defensible label, or does
the evidence better fit `Healthy`/`LimitCycle`?"* — a question with a small answer set that maps
1:1 onto the enum, so agreement/disagreement is a typed comparison, not prose.

## 3. Decorrelation requirement (DoD item 1b)

Per the research-verifier precedent: the critic's judges must be **decorrelated from the producer and
from each other** — different model AND/OR different provider than whatever produced/tuned the loop,
so a shared blind spot cannot pass unchallenged. Concretely, two `LlmBackend` handles selected to
differ on `(provider, model)` (e.g. one local Ollama model + one managed/remote model via the
existing `ManagedApiAdapter`/`dispatch.rs` path — the same swappable-backend machinery P41 already
specifies). Decorrelation is a **hard construction rule**, checked at wiring time: if only one
backend is available, the critic emits `Reading::Unavailable("no decorrelated second judge")` and
records that named absence — it never silently runs two copies of the same model and calls the
agreement "confirmation" (that would be correlated noise, the exact anti-pattern the research-verifier
charter forbids).

## 4. Advisory-only integration point (DoD item 1c) — design

- The critic runs **beside** `analyze_detailed`, never inside it. `analyze_detailed`/`classify_drift`
  stay **pure and float-deterministic** — item 27's P6 boundary holds verbatim: the critic is
  *recorded output*, never a *classifier input*. The deterministic `Verdict` is authoritative; the
  critic's opinion is a companion annotation on the same record.
- **Output = one structured signal, never a gate.** On each critiqued verdict the critic emits a
  single `FdrEvent`-style record (reusing the Tier-1 `fdr` machinery items 4/29/27 already built) —
  a `critic_disagreement` event carrying: the deterministic `Verdict`, each judge's mapped label,
  an `agreement: bool`, and the two judges' `(provider, model)` fingerprints. It is `rclone move`-able
  to the same telemetry sink as item 27's `markov_verdict` records. It is consumed by the existing
  advisory surfaces (the self-improvement ledger, a human reviewing loop health) — **nothing reads it
  to make a deterministic decision**, and there is no code path where a disagreement changes what the
  loop does. Disagreement is a *flag to look*, exactly like the Markov detector itself.
- **Fail-open, degrade-closed on the AI side.** `LlmError::Unavailable` (a judge is down, offline
  mode, or no network) → the critic records `Reading::Unavailable` and the loop proceeds untouched.
  The order/courier/money flow is provably unaffected in every mode (mode 1 no-AI: the critic simply
  never runs; the deterministic `Verdict` is unchanged). This inherits P41's three-mode contract.

## 5. Fits the existing architecture

- **Zero new deterministic primitives, zero new kernel deps.** Reuses `markov::Verdict`, the
  `LlmBackend` port, the `fdr` event machinery, and item 27's `PmuStation::bracket` seam. The critic
  lives in the AGENT/`llm-adapters` layer (it needs an LLM), behind the same compilation firewall —
  it consumes the kernel's `Verdict` type but the kernel never depends on it, preserving the
  AI-depends-on-core-never-the-reverse invariant.
- **Consistent with GROUND-TRUTH-over-PROXY.** This repo already *removed* the advisory-council-as-gate
  (memory: council/proxy/advisory-hooks removed, deterministic gates kept). P32d is deliberately the
  allowed shape: an advisory signal that informs a human/ledger, structurally unable to gate — the
  same posture as the Markov attractor detector it critiques.
- **Reuses the research-verifier decorrelation doctrine** rather than inventing a new voting scheme;
  two decorrelated judges, disagreement surfaced, no majority-rules gating.

## 6. Acceptance criteria (RED → GREEN, per this repo's standing culture)

Discharges P32d DoD items 2 and 3.

1. **DoD item 1 is discharged by this document** — a concrete loop-output type (`markov::Verdict`,
   §2), the decorrelation constraint (§3), and the advisory-only integration point (§4) are specified.
2. **GREEN (DoD item 2), minimal implementation:** one `markov::Verdict` is critiqued by ≥2
   decorrelated judges (distinct `(provider, model)`), and their disagreement is surfaced as a logged
   `critic_disagreement` `FdrEvent` — **not** a gate. A test asserts: (a) agreement path emits
   `agreement:true` and changes no loop behavior; (b) the deterministic `Verdict` returned to callers
   is byte-identical whether the critic ran or not (the advisory-only invariant, mechanically pinned).
3. **RED-provable (DoD item 3), honestly obtained:** feed a deliberately corrupted loop output — a
   state window that the deterministic detector labels `Healthy` but the harness hands the critic as
   `StrangeAttractor` (a planted mismatch) — and assert the critic emits `agreement:false`. A critic
   that can never disagree proves nothing; this is its planted-fault self-test, mirroring the
   item-7/item-6 planted-fault discipline.
4. **Decorrelation is enforced, not assumed:** a test with only one backend available asserts the
   critic records `Reading::Unavailable("no decorrelated second judge")` and runs no correlated
   double-check.
5. **No deterministic-path regression:** `analyze_detailed`/`classify_drift` remain pure and
   unchanged (item 27's P6 boundary re-asserted by an existing purity test); the default no-AI build
   (`cargo tree -e no-dev` AI-free per P41 mode 1) is untouched — the critic lives in the AI layer.
6. **Roadmap update:** replace P32d's "Blueprint: none — needs a short design note first" flag with a
   link to this file; flip its §10.5.1 status from PLANNED to "design note landed, implementation
   soft-gated on AGENT `LlmBackend` wiring."
