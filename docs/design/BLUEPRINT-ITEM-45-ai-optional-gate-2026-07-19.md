# BLUEPRINT — Item 45: `ai-optional-gate` — AI-optional as an enforced compile-time invariant

- **Date:** 2026-07-19 · **Tier:** 0/1-class (structural law) · **Status:** BLUEPRINT (planning
  artifact, no code) · **Arc:** §I "Whole-System Determinism & AI-Optional Arc" (items 45–49).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I item 45
  (lines 640–653) + §I header (governing directive, lines 627–638);
  `CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md`;
  `docs/audits/hardening/CHECKLIST.md` (5-point standard + §10-P7 re-execute-never-presence-check);
  `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (depth/style template). Code ground truth: this
  worktree (branch `main`, FDR/ct_gate/admission code present).
- **Governing directive (recorded verbatim, roadmap §I header):** *"Окрім цього уся система повинна
  здатна працювати без AI"* — AI-optional is a preserved architectural INVARIANT, not a runtime
  toggle.
- **Dependency status:** READY NOW, zero prerequisites. Asserts today's already-true invariant;
  gains teeth when the inference subsystem (items 33–44) lands. Gates no other item; item 45's
  feature-gate law binds §H's build items when they land (roadmap line 743).

---

## 1. Problem + non-goals

### Problem
The directive is that the whole system must be able to run with AI absent. Today that is
**structurally true by accident of not-yet-built** — there is no inference subsystem in the kernel,
and the core decision modules import zero AI. Item 45 converts "true because it doesn't exist yet"
into "true because it CANNOT be otherwise" — an enforced, compile-time / CI-checked invariant that
survives items 33–44 actually landing an inference subsystem.

Two distinct planes of the invariant:

1. **Build plane:** the inference subsystem lands behind a **non-default cargo feature** (the exact
   `pq` / `slot-arena` / `gpu` / `pgrust` surface-control pattern already in
   `kernel/Cargo.toml:18–90`). The default build has AI absent and must compile + pass the FULL
   kernel test suite.
2. **Dependency-direction plane:** no core decision module may reference the AI module paths
   outside the feature gate. AI depends on core; core NEVER depends on AI.

### Non-goals (explicit over-design guard — roadmap lines 649–651)
- **NOT built:** a runtime kill-switch service, a dual-binary pipeline, or an AI-health monitor.
  The *runtime* half of AI-optional is item 47's `None`-path (the deterministic total function),
  not a service this item stands up.
- **NOT** a rewrite of the existing deterministic-math organs (`attention.rs`, `micrograd.rs`,
  `online.rs`) — see §3, these are NOT "AI" for the purposes of this gate.
- **NOT** the creation of the `inference` feature/module itself — that is items 33–44's deliverable.
  Item 45 records the LAW and lands the CI scaffold now; there is nothing to feature-gate yet, and
  inventing an empty feature would be the over-design the roadmap warns against.

## 2. Current-state grounding (verified this session)

**The build-plane precedent already exists and is the exact pattern to reuse.**
`kernel/Cargo.toml:22` sets `default = ["std"]`; the opt-in subsystems each ride an off-by-default
feature with a header comment stating what it pulls and how to verify the default graph stays clean:
`pq` (`:65–71`), `gpu` (`:72–78`), `slot-arena` (`:79–90`), `pgrust` (`:59–64`). **There is no
`inference` / `ai` feature today** (verified: the feature list is exactly
`std, json-api, wasm, chaos, ct-gate, count-allocs, pgrust, pq, gpu, slot-arena`). So item 45's
build-plane law is "when items 33–44 land, add `inference` in this same shape" — a recorded rule,
not new code now.

**The dependency-direction invariant is already true — this is the "asserts today's truth" claim.**
- `kernel/src/attention.rs:17–20` states in-code: *"the kernel stays non-AI (deterministic pure
  functions); learning lives in `online` / `micrograd` at the edge if ever needed."* `attention.rs`
  is a deterministic diffusion organ (one learned-affinity step), **not** the items-33–44 inference
  subsystem.
- The core modules named by item 45 — `order_machine`, `decision/`, `hydra`, `event_log`, `markov`,
  `spectral`, `fdr` — import zero AI. (Grounded: `event_log.rs`, `decision/import.rs`, `fdr/mod.rs`
  read this session carry no AI import; the FSM/money/decision core is feature-independent per
  `kernel/Cargo.toml:35`.)

**The compile-firewall precedent to compose with (INTER-crate, a different plane).** The P40 agent
lane already demonstrates the strongest form of "structurally cannot name X":
- `agent-facade/Cargo.toml:12` — `dowiz-kernel` path dep; the facade is the ONLY agent crate that
  imports the kernel, and (per its `description`) does not re-export mutation symbols.
- `agent-loop/Cargo.toml:15–16` — depends ONLY on `agent-facade` + `llm-adapters`, NEVER
  `dowiz-kernel` directly; so `agent-loop` structurally cannot name `decide`/`fold`/stores.
- The enforcing test: `agent-loop/tests/adversarial.rs:348–367`
  (`firewall_no_direct_kernel_dependency`) — asserts `cargo tree` output does not contain
  `dowiz-kernel` and `grep -rn 'dowiz_kernel' agent-loop/src/` is empty.

> **STALE-CITATION CORRECTION (record + fix in the executor's PR, not here):**
> `agent-loop/Cargo.toml:13–14` comments claim the firewall test lives "in `tests/firewall.rs`".
> **No such file exists** — the real test is `firewall_no_direct_kernel_dependency` in
> `agent-loop/tests/adversarial.rs:348`. The Cargo.toml comment is stale (a `tests/firewall.rs`
> was evidently folded into `adversarial.rs`). Item 45's executor should correct that comment while
> touching the agent lane; it does not affect item 45's mechanism but a stale pointer to the
> canonical firewall test is exactly the kind of rot this arc exists to prevent.

**Plane distinction (load-bearing, do NOT conflate):** the P40 firewall is INTER-crate
(`agent-loop` must not name the kernel crate). Item 45's firewall is INTRA-kernel (core kernel
modules must not name the kernel's own future `inference` module outside the feature gate). Item 45
*reuses the discipline*, not the same test.

## 3. Options considered (≥2)

**Option A — CI grep/`cargo tree` assertion only (the P40-firewall shape, verbatim).**
A new `ai-optional-gate` CI job that (a) builds default-features and runs the full kernel suite,
and (b) greps the seven core modules for references to the AI module path.
- Concept: *dependency-direction check by out-of-band assertion* (same class as the P40
  `firewall_no_direct_kernel_dependency` test).
- Tradeoff: works TODAY with no AI code present (RED-provable via a planted reference); cheap;
  `--locked --offline` deterministic. But a grep is a lexical check — it cannot see a reference
  reached through a re-export, and it is only as strong as its path list.

**Option B — compile-time cfg-gate (illegal-state-unrepresentable, the strongest form).**
The AI module lives behind `#[cfg(feature = "inference")]`; the default build simply does not
compile the AI module, so any core→AI path **fails to resolve** in the default build — a `cargo
build --no-default-features`/default failure, not a grep.
- Concept: *make the violation a type/name-resolution error* (the item-9 `Result<Permit, Tripped>`
  / order_machine "unrepresentable" standard).
- Tradeoff: the STRONGEST guarantee (a core→AI import literally will not compile AI-absent), and it
  is the same mechanism that makes `agent-loop` structurally unable to name kernel mutation. But it
  **only becomes available once the `inference` feature + module exist** (items 33–44). There is
  nothing to cfg-gate today.

## 4. Decision + rationale (ADR-format)

**ADR-045: AI-optional is enforced by BOTH mechanisms, sequenced.**

- **Now (item 45 lands):** ship the `ai-optional-gate` CI job in Option-A form — default-features
  full-suite re-execution + a grep-based dependency-direction assertion over the seven core modules
  against the (reserved) AI module path prefix. This is RED-provable today (plant a
  `use crate::inference::…` line in `markov.rs` → gate RED) even though no AI module exists yet.
  Record the build-plane law (the `inference` feature must be non-default, `pq`-shaped) in the §H
  header and in this blueprint.
- **When items 33–44 land:** the inference subsystem is placed behind `#[cfg(feature =
  "inference")]` (Option B). At that point the grep assertion is BACKED by the compile-time
  guarantee — a core→AI reference is both grep-caught and resolution-failing AI-absent. The two are
  belt-and-suspenders; neither is dropped.

Rationale: Option A is the only thing that can land NOW and be honestly RED-proven; Option B is the
stronger invariant but is unbuildable until the thing it gates exists. Sequencing gets the law
recorded and mechanically enforced immediately, and upgrades it to unrepresentable when the AI code
arrives — matching how the roadmap frames item 45 ("READY NOW … gains teeth when items 33–44 land").
Boring, proven, and it composes with the existing P40 firewall discipline rather than inventing a
new enforcement idiom.

## 5. Implementation plan (numbered)

1. **Record the build-plane law** in the §H arc header and this blueprint: the inference subsystem
   lands behind a non-default `inference` feature in `kernel/Cargo.toml`, with a header comment in
   the `pq`/`slot-arena` shape (`kernel/Cargo.toml:65–90`) stating what it pulls and the
   `cargo tree -p dowiz-kernel -e no-dev` verification that the default graph stays AI-free. **No
   feature is added by item 45** (nothing to gate yet — over-design guard).
2. **Define the reserved AI module-path set** (operator-decision, §10): the exact module path(s)
   the direction check forbids in core — provisionally `crate::inference` (mirror the `pq` module's
   single-root shape). Explicitly EXCLUDE the existing deterministic-math organs (`attention`,
   `micrograd`, `online`) — they are non-AI per `attention.rs:17–20` and gating them would
   false-positive the whole current tree.
3. **Ship `scripts/ai-optional-gate.sh` + a `ci.yml` job** in the shape of the zero-dep-gate /
   toolchain-bump-gate:
   - (a) `cd kernel && cargo test --offline` under **default features** (AI absent) — the full
     suite must be green inside the job (re-execution, never presence-check; CHECKLIST §10-P7).
   - (b) dependency-direction assertion: for each core module in
     `{order_machine.rs, decision/, hydra.rs, event_log.rs, markov.rs, spectral.rs, fdr/}`, assert
     zero references to the reserved AI path prefix outside a `#[cfg(feature = "inference")]` block.
     Today (AI absent) this is a pure "no core→AI reference" grep; when the feature exists it is
     additionally guaranteed by name-resolution failure (Option B).
   - (c) `cargo tree -p dowiz-kernel -e no-dev` still resolves AI-free (the default-graph proof,
     item-1/13 gate shape).
   - Determinism (P6): every cargo invocation `--locked --offline`; assert `Cargo.lock` hash
     unchanged after the run.
4. **When items 33–44 land:** move the inference subsystem behind `#[cfg(feature = "inference")]`;
   the job's part (b) is now compile-backed. Record the feature-gate law in the AI module's own doc.

## 6. Failure + degradation

- Item 45 is a CI gate, not a runtime path — its "degradation" is the default-features build being
  the safe fallback. The runtime AI-absent behaviour is item 47's `None`-path (a total deterministic
  function), explicitly out of item 45's scope.
- If the `inference` feature does not yet exist, part (b) degrades to the AI-free grep (still
  RED-provable), never to a no-op — a filter matching zero forbidden references is GREEN only
  because the tree is genuinely AI-free, and the planted-reference RED demonstration (§7) proves the
  check has teeth.

## 7. Required tests / proofs (per CHECKLIST.md 5-point standard)

The 5-point checklist is written for *algorithmic hot paths*; item 45 is a structural CI gate, so
the mapping is by the §10-P7 re-execute-never-presence-check discipline (the same standard the
zero-dep-gate and P40 firewall meet), not the exhaustive-oracle form:

1. **Oracle / re-execution:** the default-features full kernel suite IS the re-executed oracle
   (`cargo test --offline`, live counts). A filter/suite that matches zero tests is RED.
2. **dudect gate:** N/A — no secret-dependent timing in a build gate. Record `N/A(build-gate)`.
3. **Debug cross-check:** N/A — no per-call reference. Record `N/A(build-gate)`.
4. **Deterministic re-executed check (the P7 core, this item's real proof):** a planted violation
   turns the gate RED *before the gate counts as landed*:
   - (a) plant a `use crate::inference::Model;` reference in `markov.rs` → part (b) RED.
   - (b) plant a default-features AI reference (once the feature exists, an un-gated `crate::inference`
     use) → default build fails to compile / part (b) RED.
   - (c) clean tree → gate green; `cargo tree -e no-dev` AI-free.

**Falsifiable acceptance criteria:**
- The planted-import RED demonstration ((a) and (b)) is recorded in the item-45 PR body before the
  gate is declared landed (P7 one-layer-up, item-6 §2.4 precedent).
- The default-features full kernel suite runs GREEN *inside* the `ai-optional-gate` job.
- `cargo tree -p dowiz-kernel -e no-dev` resolves with the AI subsystem absent (the arc lands with
  the zero-dep allowlist still empty).
- The build-plane feature-gate law is recorded in the §H header and (when AI lands) the AI module doc.

## 8. Security + tenant isolation

Not a data-plane change — no RLS/PII/money surface touched. The relevant security property is
*supply-chain*: the AI subsystem (which will pull heavy external deps behind `inference`) cannot
enter the default kernel graph, keeping the canonical order/money core pure-`std` and serde-free
(the same guarantee `pq`/`gpu` already give). The dependency-direction check additionally prevents a
core module from being silently coupled to an AI model — a core decision must never depend on an
optional inference organ's availability.

## 9. Operability

- **Health:** the gate is binary (green/red) in CI; no runtime health surface.
- **Observability (<1 min):** a RED gate names the offending core→AI reference (file:line) via
  `::error::`, same as the P40 firewall test's message.
- **Rollback:** removing the job is a one-line CI revert; the invariant it asserts is separately
  true by construction (default build AI-absent).
- **Scaling gate:** none — CI-only.

## 10. Open / accepted risks + operator-decision points

- **[OPERATOR-DECISION] The reserved AI module-path set.** Item 45 provisionally names
  `crate::inference` as the forbidden-in-core prefix and EXCLUDES `attention`/`micrograd`/`online`.
  The operator (with items 33–44's authors) must confirm the exact AI module root(s) so the
  direction check neither under-covers (misses a real AI module) nor false-positives the current
  deterministic-math organs. *Owner: operator + items-33–44 lead.*
- **[ACCEPTED RISK] Grep-only enforcement until the feature exists.** Until items 33–44 land,
  part (b) is a lexical check (Option A), weaker than name-resolution failure (Option B). Accepted
  because there is no AI code to cfg-gate yet, and the planted-reference RED demonstration proves the
  grep has teeth. Upgraded to compile-backed automatically when the feature lands. *Owner: item-45
  executor.*
- **[FLAG] Stale Cargo.toml comment.** `agent-loop/Cargo.toml:13–14` points to a non-existent
  `tests/firewall.rs`; the real test is `adversarial.rs:348`. Correct in the executor's PR. *Owner:
  item-45 executor.*
- **[ACCEPTED] Item 45 does not build the runtime kill-switch / dual-binary / AI-health monitor**
  (roadmap lines 649–651). The runtime half of AI-optional is item 47's `None`-path. Recorded as
  intentional scope, not a gap.
