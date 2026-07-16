# The Seven Hermetic Architecture Principles — Consolidated Engineering Document

> Synthesis of seven independent Opus research+audit passes (2026-07-16), each grounding one
> Kybalion principle as a concrete dowiz/DeliveryOS + openbebop architecture rule and auditing the
> live tree (`feat/kernel-fsm-graph-analysis`, plus `/root/bebop-repo` and
> `/root/hermes-agent-kernel-rewrite`) for compliance. Sources: `PRINCIPLE-1-MENTALISM.md` …
> `PRINCIPLE-7-GENDER.md` in this directory. Every claim below cites its source report; the two
> load-bearing anchors (P4-V2 swallowed writes, `FileEventStore` existence) were re-verified against
> the live tree during this synthesis (`kernel/src/hydra.rs:712,821-852`, read 2026-07-16).
> Zero mysticism: each principle is an engineering rule this project already practices or already
> claims to practice. This document is (1) the reference checklist, (2) the cross-referenced
> root-cause analysis, (3) the ranked findings table, (4) an honest verdict.

---

## 1. The Seven Principles — reference checklist

Use this section as a design-time checklist for any new dowiz/openbebop feature. Each statement is
the verified form from its audit pass, not a reinterpretation.

### P1 — MENTALISM: the spec is the source of truth; code is a derived artifact

The specification, the proof, or the decision record is the source of truth — the code is a
*derived artifact* of it. A capability must exist as a checkable idea (equation, ADR, blueprint
done-check) before or alongside the code that manifests it; **code without a recorded idea, and an
idea asserted as real without code that manifests it, are both defects** (PRINCIPLE-1 §1). The
project arrived at this rule independently three times: "the plan is the spec" (`AGENTS.md:207-210`),
VERIFIED-BY-MATH, and the Anu doctrine (`AGENTS.md:234-241`). The literal implementation is `eqc`
(`tools/eqc/README.md`): equation → generated, self-asserting Rust. One honest refinement the audit
forced against git history: ADRs 0007–0009 were all written *after* their code in one commit
(`bd86908ad`), so the enforceable form is **"the idea must be recorded before the knowledge is
lost"** — record-of-decision, not always precede-implementation (PRINCIPLE-1 §1, empirical
refinement). *Check when designing:* does every artifact your doc cites resolve live (`find`/`grep`,
not memory)? Does the new capability have a falsifiable done-check written before "done" is claimed?
Is every gap honestly tagged (`(P1) NOW: implement`) rather than asserted as existing?

### P2 — CORRESPONDENCE: one concept, one primitive, or a parity-pinned divergence

A mechanism proven correct at one scale must be the **same mechanism — one implementation, one entry
point** — when the same conceptual problem recurs at another scale, unless a stated, falsifiable
reason forces divergence; and where divergence is forced, the divergent implementations **must be
pinned to each other by a parity check** so they cannot drift (PRINCIPLE-2 §1). Corollaries: one
concept → one primitive (one Laplacian, one PageRank, one hash chain); **a dead canonical primitive
is a violation, not a neutral** (if the architecture names an operator as *the* unifying primitive
and nothing calls it, the correspondence exists only in prose); justified divergence without a
parity gate is a latent drift bug. The model implementations: one `sha3_256` shared by the event
chain and the spine chain (`event_log.rs:30`, `spine.rs:92`; PRINCIPLE-2 F-4), and the eigensolver's
one dispatch entry point with a cross-solver parity test (PRINCIPLE-2 F-5). *Check:* before writing
a second implementation of any math object, name the forcing reason in the code and add the parity
test in the same commit — or call the existing primitive.

### P3 — VIBRATION: every dynamic subsystem has a named, single-authority, tested rate

Every stateful or dynamic subsystem has an **explicit, named, and tested RATE characteristic** — a
damping ratio ζ, an oscillation period, a fixed sampling frequency, a refill rate, a retry cadence,
a drift class — deliberately chosen, never an implicit default or "animate it and see"
(PRINCIPLE-3 §1). Corollaries: **A1** classify loops by spectral radius ρ against the unit circle
(`classify_drift`, `spectral.rs:325-335`); **A2** when a rate crosses a module/process boundary,
exactly one site owns the value and a test pins every mirror (the template: `DT_STABLE` pinned on
both kernel and engine sides, `kernel/src/lib.rs:180`, `engine/src/loop_.rs:163`); **A3** assert the
rate inside its stability bound before it reaches the integrator (`assert_stable` CFL bound,
`field_frame.rs:55-68`); **A4** money is never a field channel — no easing/tween ever touches a
monetary quantity (FE-09; `motion.rs:163`). *Check:* new loop or cadence → name the constant, state
its authority, pin every mirror with a test, prove the stability bound.

### P4 — POLARITY: one mechanism carrying two named poles; collapses are typed and safe-directed

A binary architectural state — allow/deny, fail-closed/fail-open, Damped/Unstable, durable/lost —
must be **one value or one mechanism carrying two named poles**, never two independently-maintained
code paths encoding opposite outcomes (PRINCIPLE-4 §1). Corollaries: one decision function, two
typed outcomes (`Result<T, DenyReason>` — `assert_transition`, `HybridGate::check` with its
14-variant enumerated deny axis); every intermediate degree is a named variant on the one axis
(`Resonant` between `Damped` and `Unstable`); **a collapse of one pole into the other must go toward
the safe pole and be typed** — "no data" must never render identical to "failed." The audit found
zero silent-`else` implicit denies in the Rust core; the real violations are pole-*collapses* that
hide a failure behind a well-behaved value (PRINCIPLE-4 §3). *Check:* does any `unwrap_or`,
`let _ =`, or infallible `-> ()` signature erase a failure pole? If two poles must render one
surface value, is the collapse toward the safe pole and typed?

### P5 — RHYTHM: both swings wired, and structurally guaranteed to fire

Any process whose correctness depends on periodic re-execution — backup verification, hygiene
pruning, drift re-checking, retry-with-backoff — must ship as a **bidirectional cycle with both
halves wired** (the outward swing AND the compensating return swing) and must be **structurally
guaranteed to fire, not coded once and left to a host scheduler or an agent's memory**
(PRINCIPLE-5 §1). The two failure shapes: *half a pendulum* (backups written, never restore-tested;
a ledger that only accumulates) and *a dead pendulum* (cycle coded and registered, but firing
depends on external state — strictly worse, because the record says "running" while `last_run` is
`None`). This is the repo's own Ananke standard — "structurally inevitable, not remembered" —
applied to time. Healthy templates: bounded retry with automatic reset (`async-spool`
`MAX_ATTEMPTS=4`, requeue-not-deadletter), the bounded verify-retry loop
(`verify_retrieval.rs`), spool backpressure (PRINCIPLE-5 §4). *Check:* for every "periodic" claim —
where is the return swing, and what *structurally* fires it? Is the schedule in the repo?

### P6 — CAUSE-AND-EFFECT: determinism as law; no effect escapes its declared cause

No kernel computation may produce an effect (an output byte, an idempotency key, a content-address,
a signature, a gate verdict) that is not a **pure, reproducible function of its declared cause**.
Every source of apparent chance — randomness, wall-clock, hash-map iteration order, float summation
order, external state — must be (a) eliminated, (b) named, seeded, and isolated behind a port, or
(c) proven not to leak into any effect a test or peer relies on (PRINCIPLE-6 §1). Effects are ranked
by plane: P0 idempotency keys/signatures (mesh-breaking), P1 gate verdicts, P2 rankings, P3
display/transport JSON. The audit's headline is compliance: `event_id` is fixed-order byte
concatenation, `snapshot_root` folds a `BTreeMap`, the RNG is seeded and pinned to reference
vectors, PPR/BM25/markov fix summation order — **P0/P1 hold, provably** (PRINCIPLE-6 §3-4).
*Check:* does any `HashMap` iteration reach a `format!`/serialize/hash? Is every claimed
"byte-identical" scope actually exercised by a test of that scope?

### P7 — GENDER: the Law of Paired Creation — no self-certified "done"

No capability — a plan, a decision, persisted state, a "GREEN", a minted authorization — may be
brought into being by only its active/generative half. Every generative act must be paired with a
**receptive/formative counterpart that the active half cannot bypass or supply for itself**: a
verifier the author cannot forge, a passive render layer that cannot invent state, a structural
constraint the generator cannot relax (PRINCIPLE-7 §1.1). Two failure shapes: *missing half* (a mint
with no verify) and *non-independent half* (both halves exist, but the passive one consumes data the
active one produced — the check reduces to the claim restating itself: **self-certification**).
Verified positive instances: the F-7 render split (server generates state, client only eases it),
Anu⊗Ananke implemented in code (`governance.rs::decide`, Ananke overrides Anu on red-lines), and the
mesh capability path, where sign needs the private seed but verify needs only public data —
structural independence via asymmetric crypto (PRINCIPLE-7 §1-2). *Check:* who supplies the inputs
the checker reads? If the answer is "the same party being checked," the pairing is decorative.

---

## 2. Cross-referenced root causes

Twenty-nine findings across seven reports reduce, at the top of the severity range, to **four
recurring root causes**. Fixing a root cause closes findings under two or more principle headings
simultaneously — these are the highest-leverage fixes in the entire set.

### RC-1 — Self-certification: the claim replaces the check (doc layer × code layer × ops-record layer)

Mentalism and Correspondence found the **same failure shape at two different layers**, and Rhythm
supplies a third instance:

- **Doc layer (Mentalism F1, F2).** ADR-020 is cited by 15+ documents including `MANIFESTO.md` as
  the authoritative decision record for the entire open-source/license/trademark strategy — and it
  **does not exist** (`find` → 0 results; PRINCIPLE-1 F1). Two canon blueprints from this same
  session disagree about where `eqc-proofs/lambda_max_of_d.rs` lives, and **both are wrong** — it
  exists in `/root/bebop-repo/rust-core/eqc-proofs/`, not dowiz (PRINCIPLE-1 F2). In both cases a
  decision or artifact was *asserted as real* without the check (a `find`, a `grep`) that would have
  made the assertion true or exposed it.
- **Code layer (Correspondence F-1, F-2).** The flagship "ONE Laplacian `L` across five subsystems"
  and "same primitive, different graph" claims are **refuted at the code level**: the operator the
  architecture names as canonical (`csr::laplacian_spmv`, `csr.rs:307`) has **zero production
  callers**; the live drift-gate uses a different dense implementation (`spectral.rs:287`); `L=D−A`
  exists at least four independent times across the two repos; and the R-LM design tells the viz to
  use `csr::personalized_pagerank` while its own cited fixture runs the dense `Ppr` instead
  (PRINCIPLE-2 §2a, F-1, F-2). The unification is *design-true, code-false* — an idea asserted as an
  implemented fact without the call-sites that would make it one.
- **Ops-record layer (Rhythm Finding 1).** `MEMORY.md` records the deep-clean cronjobs as "created"
  and effectively running; the scheduler's own state file shows `last_run=None` for **every** job
  and the gateway down (PRINCIPLE-5 §2). The record asserts a firing rhythm that live state
  falsifies.

**This is one root cause, not three findings.** At the doc layer the missing artifact is the
decision record; at the code layer it is the caller wiring; at the ops layer it is the
`last_run` timestamp. All three are the exact pattern BRAIN-TOPOLOGY named in advance
("self-certification = claim replaces check") — and, tellingly, the Mentalism and Gender reports
each independently cited that finding as the predicted failure mode. The structural fix is the same
in all three layers: **an assertion that an artifact/wiring/rhythm exists must carry (or be
periodically re-run against) a mechanical probe** — resolve the cited path, grep the caller, read
`last_run`. The Mentalism pass's own methodology (resolve every cited `.rs` path live) is the
antidote; note that making that antidote *recur* is itself a Rhythm problem, which currently has no
working scheduler (RC-1 and the dead-pendulum findings interlock).

### RC-2 — Verification organs without independent teeth (Gender × Rhythm)

Gender and Rhythm found the **same failure shape in the self-governance machinery**: a
verification/audit mechanism that exists on paper or in type but has no independent, structurally
guaranteed enforcement.

- **Gender V-1.** The hermes-kernel "done" gate (`verification.rs:92-158`) is a real, hard reducer —
  `Complete` is only legal from `Verified` — but **both of its inputs are supplied by the same
  session it gates**. The kernel is policy-only ("It never runs checks itself," `:6`); it derives
  `Verified` from whatever pass/fail the author hands it. The active half feeds the passive half.
- **Gender V-2.** `FalseClaimMeter` (`governance.rs:32-84`) — the honesty metric of the whole
  self-improvement loop — is doubly broken: **unfed** (no per-commit log wires real data into it)
  and **non-independent even when fed** (its `verified` bool is asserted by the audited party).
- **Gender V-3.** Breach detection: a peer can only ingest a breach the compromised node *chose to
  broadcast* via its own `raise_breach_alarm` (`hydra.rs:286-315`); `boot_verify` is a
  self-administered self-check. Silence-before-witnessing is uncovered.
- **Rhythm Finding 5.** The 2-question doubt ritual (`AGENTS.md:121-156`) is labelled "MANDATORY, not
  optional — at three points," and has **zero firing mechanism**: the `settings.json` hooks block is
  empty, all hook scripts are no-op pass-throughs (operator directive 2026-07-15). It depends
  entirely on being remembered — which, by the repo's own Ananke standard, means it has already, on
  any forgetful turn, stopped.

**Two-report convergence on one missing artifact:** the per-commit **claim-latency ledger** that
Rhythm Finding 6 shows does not exist anywhere in code (`P01:72`, zero grep hits) is *exactly* the
feed `BLUEPRINT-P06 §6` says `FalseClaimMeter` lacks (Gender V-2's first defect). Two audits, two
principles, one absent artifact found from two directions — the generative feed (Rhythm's view) and
the starved receptive organ (Gender's view).

The structural fix is likewise one build, already designed and not yet begun: **the P06 key_V
independent re-execution path** (`key_K` signer ≠ `key_V` verifier, fresh-worktree re-execution,
zero code hits outside docs today; PRINCIPLE-7 §1.4). Building it and routing V-2's `verified` input
and V-3's integrity attestation through it closes G-V1, G-V2, and G-V3's independence defect at one
stroke; wiring the doubt ritual and the ledger to hooks/timers closes R-5 and R-6. Cause-and-Effect
Finding B is a weaker adjacent member of this family — the "byte-identical across runs, platforms,
and builds" claim is guarded only by same-process double-call tests, i.e. **the stated guarantee is
broader than the check that defends it** (PRINCIPLE-6 Finding B) — the same teeth-gap in
epistemically milder form.

### RC-3 — A fail-open hole under the cause-and-effect substrate (Polarity × Cause-and-Effect)

Polarity V2 and the entire Cause-and-Effect thesis meet at one type signature.

- **The Polarity finding.** `EventStore::insert` is typed **infallible** — `fn insert(&mut self, id,
  ev)` returns `()`; there is no failure pole in the port at all (`event_log.rs:162-166`). The
  durable implementation, `FileEventStore` (`hydra.rs:825-852`, re-verified live this synthesis),
  swallows every IO `Result`: if the file fails to *open*, or `write_all`/`flush`/`sync_all` fails,
  the disk write is silently skipped while the in-memory map, tip, and count still advance — and the
  caller receives `AppendOutcome::Committed` (`event_log.rs:272,317`). A durability failure is
  byte-indistinguishable from a durable commit (PRINCIPLE-4 V2).
- **Why Cause-and-Effect owns this too.** P6's audit verified the *purity* of the cause→effect
  function and found it excellent at P0: `event_id` is a fixed-order content address, replay of an
  identical cause is a structural no-op, `fold_transitions` deterministically re-derives state,
  `boot_verify` replays the log after restart (PRINCIPLE-6 §1, §3). But every one of those
  guarantees quantifies over *the events in the log*. **The event log is the cause-and-effect
  substrate everything else replays from** — and Polarity showed the substrate can silently drop a
  cause while reporting it committed. A deterministic replay over a log with silently-lost writes
  reproduces the wrong history *deterministically*: the Law holds perfectly over corrupted premises.
  The WORM log is also the anti-tamper witness substrate ("a tampered core can never silently
  heal"), which assumes `insert` reached disk.
- **A synthesis-level correction.** Rhythm's report stated "only `MemEventStore` exists, non-durable
  … the replay engine is correct but idling" (PRINCIPLE-5 §4). This is imprecise: `FileEventStore`
  exists (`hydra.rs:712`). The corrected fact is *worse*, not better — the durable substrate exists
  and is the thing that swallows failures. One report under-claimed, one report found the hole;
  the live tree confirms the hole.

**Fix direction:** one signature change — `fn insert(...) -> Result<(), StoreError>` — propagated
into `AppendOutcome`, plus a RED-first IO-fault-injection test. This single fix restores the failure
pole (P4), makes `Committed` mean committed (P6), and gives the replay/restore rhythms (P5) a
substrate whose contents they can trust.

### RC-4 — Unpinned mirrors at the kernel↔engine seam (Vibration × Polarity × Correspondence)

Three reports independently found hand-maintained mirrors across the same module boundary, each
defended by a comment instead of a pin:

- **Vibration Finding 1 (MEDIUM).** `kernel/src/lib.rs:180` declares `DT_STABLE = 0.02` (50 Hz) as
  *the* single source of truth the field integrator "MUST only ever see" — and
  `engine/src/field_frame.rs:47` independently defaults `dt: 0.016` (~60 Hz), a 25 % rate mismatch
  with no linking test. The kernel's own promise is falsified by the engine default.
- **Polarity V3 (LOW).** `engine/src/bridge.rs:648-663` re-declares a second `enum DriftClass` bound
  to the kernel authority only by a comment and a hand-maintained numeric wire contract.
- **Vibration Finding 2 (LOW-MED).** `TG_MIN_GAP_S = 3.5` crosses a *repo* boundary into
  hermes-kernel guarded by nothing but the comment "MUST match."
- **Correspondence's corollary is the rule these all break:** forced divergence must be pinned by a
  parity check. And the seam is the same one where Correspondence found the ONE-Laplacian claim
  broken — the engine's field integrator implements its own damped-wave `L`-operator
  (`field_frame.rs:10-11`), consistent with Correspondence's "at least four independent"
  implementations count (PRINCIPLE-2 §2a). **The kernel↔engine boundary is systematically mirrored
  by hand** — the operator identity, the integration rate, the drift enum — with exactly one
  exception: `DT_STABLE` itself is mirror-pinned by tests on both sides (`dt_stable_is_authoritative`,
  `dt_stable_matches_kernel_contract`), which is why every report cites it as the template. The fix
  is mechanical: every constant/enum/operator mirrored across the seam gets the `DT_STABLE`
  treatment (a pin test on both sides) or is generated from the kernel definition.

---

## 3. Ranked findings table

All findings from the seven reports, severity-sorted. **★RC-n** flags a finding that participates in
a cross-referenced root cause from §2 — these are the highest-leverage fixes: resolving the root
cause closes findings under two or more principles at once. Severity is the source report's rating;
ordering within a band puts load-bearing/multi-principle findings first.

| # | Sev | Finding | Principle(s) | Evidence | Fix direction |
|---|-----|---------|--------------|----------|---------------|
| 1 | **HIGH** ★RC-3 | `EventStore::insert` typed infallible (`-> ()`); `FileEventStore` swallows open/write/fsync failures, in-memory state advances, caller told `Committed` — durability failure indistinguishable from durable commit, under the substrate all replay/witness guarantees quantify over | Polarity V2 × Cause-and-Effect | `kernel/src/event_log.rs:162-166,272,317`; `kernel/src/hydra.rs:840-851` | `fn insert(...) -> Result<(), StoreError>`, propagate into `AppendOutcome`; RED-first IO-fault-injection test |
| 2 | **HIGH** ★RC-2 | The "done" gate reads **author-supplied** evidence: `assert_can_complete`/`derive_state` consume `touched_verifiable` + `EvidenceStatus` from the same session they gate; kernel is policy-only, never re-executes — every completion claim is self-certified | Gender V-1 (× Rhythm F5, same root) | hermes-kernel `kernel/src/verification.rs:6,92-158` | Build the P06 key_V independent re-execution path (fresh worktree, `key_K ≠ key_V`); currently zero code hits outside docs |
| 3 | **HIGH** ★RC-1 | **ADR-020 is a phantom**: cited by 15+ docs + `MANIFESTO.md` as the authority for the entire OSS/license/TM strategy; never written (`find` → 0). An irreversible public action is gated on a decision record that does not exist | Mentalism F1 | `docs/adr/` has 0001–0009, no 020; 15 citing files | Write ADR-020, or delete every citation; never cite a decision record without resolving its path |
| 4 | **HIGH** | COLD backup restore-drill has **never run**: 3 real zstd archives written; no restore-verify subcommand exists anywhere (`find`/`grep` → nothing). Schrödinger's backup — the 3-2-1-1-0 "0 errors" leg is unautomated | Rhythm F3 | `/root/.backups/cold/*` (2026-07-16); `R2-MERGED-PHASE-ROADMAP.md:88` | `restore-verify` subcommand asserting byte-identity + `integrity_check=ok`, drilled on a timer (P12) |
| 5 | **HIGH** ★RC-1 | Hermes cron gateway DOWN; all 4 registered jobs `last_run=None` — the daily prune that bounds a 1.29 GiB `state.db` has never fired on schedule, while MEMORY.md records the jobs as created/running | Rhythm F1 | `hermes cron status` (live, 2026-07-16); `~/.hermes/cron/jobs.json` | `sudo hermes gateway install --system`; correct the MEMORY record; necessary but insufficient — see #6 |
| 6 | **HIGH** | Schedule lives outside the repo (host `jobs.json`, no systemd unit): even with the gateway revived, a fresh hub checkout gets the deep-clean *binary* but not the *rhythm* — hygiene periodicity is unreproducible from canon | Rhythm F2 | `R1-C gap-analysis:290-293`; `R2:88`; `BLUEPRINT-P12:250-252` | In-repo systemd timer units + revived gateway (P12) — Ananke applied to time |
| 7 | **MED-HIGH** ★RC-2 | `FalseClaimMeter` is unfed AND self-fed: no per-commit log wires real data in, and its `verified` bool is asserted by the audited party — the loop's honesty metric can read 0 % false claims at any true rate. The missing feed IS the unbuilt claim-latency ledger (#27) | Gender V-2 × Rhythm F6 | hermes-kernel `governance.rs:32-84`; `BLUEPRINT-P06 §6`; `P01:72` | Build the ledger as the feed; route `verified` through key_V (#2) so the meter stops measuring self-report against self-report |
| 8 | **MED** ★RC-1 | "ONE Laplacian across five subsystems" refuted as implemented: `csr::laplacian_spmv` has **zero production callers**; the live drift-gate uses a different dense `L`; `L=D−A` exists ≥4 independent times across the repos; two *live* dense Laplacians (dowiz `spectral.rs:287`, bebop `field.rs:82`) share no code and no parity test | Correspondence F-1 | `kernel/src/csr.rs:307` (dead); `spectral.rs:287`; `bebop2/core/field.rs:82` | Parity-bind the two live Laplacians (or route through one operator); mark ONE-`L` explicitly aspirational in `ARCHITECTURE.md` until `csr` has a live caller |
| 9 | **MED** ★RC-1 | PPR implemented 3×; the design (R-LM) tells the viz to use `csr::personalized_pagerank` while its own cited fixture runs the dense `Ppr` — live-vs-live duplication of the same math with no shared core and no parity test | Correspondence F-2 | `csr.rs:228-264` (bypassed); `retrieval/ppr.rs:42`; `diffusion.rs:126-135`; `markov.rs:81` | Collapse to one engine or parity-gate; fix the R-LM pointer to whichever survives |
| 10 | **MED** ★RC-4 | Two disagreeing "authoritative" timesteps: kernel `DT_STABLE=0.02` (50 Hz, declared sole authority, mirror-pinned to `loop_`) vs `field_frame` default `dt=0.016` (60 Hz), unlinked — 25 % rate mismatch between the sampling clock and the field's integration constant | Vibration F1 (× Correspondence boundary) | `kernel/src/lib.rs:172-180`; `engine/src/field_frame.rs:47,143` | Pin `FieldEquilibrium::default().dt` to `DT_STABLE`, or document deliberate 60 Hz + add the mirror-pin test |
| 11 | **MED** | Money path swallows the tax error-pole: `apply_tax(...).unwrap_or(0)` renders a computation *failure* identical to a *tax-free order*; the sibling fee keeps its pole (`fee_known`), and the authoritative path (`domain.rs:101`) propagates with `?` — one function, two callers, opposite polarity discipline | Polarity V1 | `kernel/src/money.rs:218` vs `domain.rs:101` | Return `Result` or add `tax_known` mirroring `fee_known`; never `unwrap_or(0)` a money `Result` |
| 12 | **MED** (latent) | wasm `funnel` JSON leaks `HashMap` iteration order into an emitted effect: same input can emit byte-different JSON. Harmless as display (P3 plane); breaks non-reproducibly the day anyone golden-tests, diffs, or content-addresses it | Cause-and-Effect A | `kernel/src/wasm.rs:104-112,239-254`; `Cargo.toml:47` | One line of type discipline: `BTreeMap` (the exact `memory_store.rs:85-94` fix) |
| 13 | **MED** ★RC-1 | Two same-session canon blueprints disagree on where `eqc-proofs/lambda_max_of_d.rs` lives; **both wrong** (it is in `/root/bebop-repo/rust-core/eqc-proofs/`). A reader of SYNTHESIZED would search dowiz and be blocked — Anu failure: decision asserted, location never checked | Mentalism F2 | `SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16.md:174-175`; `BLUEPRINT-P02:288` | Correct both docs to the real path; cite-with-probe discipline |
| 14 | **MED** | Retroactive ADR-writing: ADRs 0007–0009 all created in one post-hoc commit ("port security lessons to ADRs") — systemic habit that silently weakens the guarantee an ADR gives (design reasoned *before* build) | Mentalism F3 | `git log --diff-filter=A` → `bd86908ad` | Restate the rule honestly as record-of-decision; prefer spec-first; date the reasoning vs the code in each ADR |
| 15 | **MED** ★RC-2 | Breach detection originates in the audited party: `raise_breach_alarm` fires only from the node's own `integrity_check`; `boot_verify` is self-administered; a compromised core that stays silent produces no alert for any peer | Gender V-3 | `kernel/src/hydra.rs:252-264,286-315,329-343` | Independent peer-driven integrity probe/attestation, routed through key_V (#2) |
| 16 | **MED** ★RC-2 | Session-close 2-question doubt ritual labelled "MANDATORY … at three points" with **zero firing mechanism** (hooks block empty, all hook scripts no-op) — a chosen gap (operator directive), but the ritual added to stop compounding mistakes depends on being remembered | Rhythm F5 | `AGENTS.md:121-156`; `settings.json` hooks (verified empty) | Record it as unenforced (done here), or wire a minimal session-close hook when the operator restores enforcement |
| 17 | **MED** | Dead one-shot cron: `bebop-library-star-list` still `[active]` with its single fire-instant 4 days in the past and `last_run=None` — a swing pending that physics will never deliver | Rhythm F4 | `hermes cron list` (live) | Expire or fire-on-revive semantics for missed one-shots |
| 18 | **LOW-MED** ★RC-4 | Cross-repo pacing contract defended only by a comment: `TG_MIN_GAP_S=3.5` "MUST match hermes-kernel" with no shared constant, no pin test; twin `DEFAULT_GAP_S` in async-spool | Vibration F2 | `tools/telemetry/rust-spool/src/main.rs:29-30`; `tools/async-spool/src/main.rs:37` | Shared constant or test-time assertion — the `DT_STABLE` treatment |
| 19 | **LOW-MED** | "Byte-identical across runs, platforms, and builds" is only ever tested by same-process double-call — the stated guarantee is broader than what the harness proves; would not catch a reintroduced map | Cause-and-Effect B (RC-2-adjacent) | `ppr.rs:104`; `diffusion.rs:191`; `csr.rs:495`; `recall.rs:335`; `rng.rs:4-6` | One second-process (serialize→re-read) test per claimed surface |
| 20 | **LOW-MED** | Reproducibility banner covers transcendental-float paths (`ln/sin/atan2/hypot`) whose cross-platform bit-identity IEEE-754 does not grant; only the integer RNG fully earns "bit-identical across platforms" | Cause-and-Effect C | `spectral.rs:53-93`; `bm25.rs:207`; `geo.rs:19-35,80` | One sentence of doctrine: "transcendentals: reproducible per-target, not cross-target" |
| 21 | **LOW-MED** | Event-sourcing corresponds at `decide` (generic seam) but the `fold` reducer is per-subsystem — a third subsystem needing state-reconstruction will hand-roll a third fold | Correspondence F-6 | `event_log.rs:300-319`; `order_machine.rs:137-151`; `intake.rs:47` | Extract a shared replay/projection primitive when the third consumer appears |
| 22 | **LOW-MED** | Graph `from_edges` conversion hand-rolled per module (self-loop/dedup/direction rules can diverge); `mat.rs` is a documented half-finished consolidation with the `Vec<Vec<f64>>` API seam still live | Correspondence F-3 | `csr.rs`, `field.rs:52`, `diffusion.rs:111`, `harmonic.rs:31-36`; `mat.rs:6-9` | One conversion hub; finish the `mat.rs` retirement it already declares |
| 23 | **LOW** ★RC-4 | `DriftClass` spectrum declared twice: engine re-declares the enum, bound to the kernel only by a comment + hand-maintained numeric wire codes (mitigated: contract tested, `_ => Unstable` collapse is safe-directed) | Polarity V3 | `engine/src/bridge.rs:636,648-663,673-678` | Share one enum or generate the engine copy from kernel discriminants |
| 24 | **LOW** | Linear, un-jittered retry backoff duplicated at 6 sites (2s·attempt, no jitter → thundering-herd resonance across N spools); fixed 0.5 Hz idle poll with no adaptive backoff | Vibration F3 | `rust-spool/src/main.rs:138,144,208`; `async-spool/src/main.rs:256,262,278,339` | One shared backoff helper (exponential+jitter) or an ADR that linear-no-jitter is deliberate |
| 25 | **LOW** | Eigen*vector* output has no independent second party: `jacobi_eigen` is the sole vector source; parity harness binds eigen*values* only (vectors validated by residual, not a second solver) | Correspondence F-5 residual | `bebop2/core/field.rs:29,308`; `dmd.rs:76-107` | Add a second-solver vector parity check (values are already the model pattern) |
| 26 | **LOW** | "A hub is a neuron in the mesh" is structurally two unrelated modules: hub has a dense adjacency graph, mesh-node has **no** topology primitive; M7 heal (Dijkstra/Union-Find) unimplemented — a gap, not a drift | Correspondence F-7 | `hydra.rs:41`; `bebop2/mesh-node/src/node.rs:69`; `ARCHITECTURE.md` M7/F45/F46 | Land M7 with a representation shared (or parity-bound) with the hub graph |
| 27 | **LOW** ★RC-2 | Claim-latency ledger: does not exist (zero code hits), and even the design defers the consumer to Phase 8 — a designed-in half-pendulum (append-only ledger nobody reads), and the missing feed for #7 | Rhythm F6 × Gender V-2 | `BLUEPRINT-P01:72,136,174-175` | Ship appender + consumer together; wire as `FalseClaimMeter`'s feed |
| 28 | **INFO** | Legacy ad-hoc-easing UI (incl. a literal money-tween, `AnimatedNumber.tsx` hardcoded 240 ms cubic over a currency value) is quarantined out of the build but not deleted — retrievable anti-pattern | Vibration F4 | `apps/web/node_modules/@deliveryos/.ignored_ui/...AnimatedNumber.tsx:10,22` (untracked, unimported) | Delete the quarantined tree, or CI-forbid importing `.ignored_ui` |
| 29 | **LOW** | Stray zero-byte untracked artifact `kernel/=5` (accidental shell redirect) — a manifestation with no idea behind it | Mentalism F4 | `kernel/=5` (0 bytes, untracked) | `rm 'kernel/=5'` |

**Reading the table for leverage:** four actions close fourteen ★-flagged rows. (1) The
`EventStore::insert` signature fix closes #1 and restores the substrate #4–#6's replay rhythms
depend on. (2) Building key_V + its feeds closes #2, #7, #15, and de-fangs #16/#27/#19's
claim-broader-than-check family. (3) A cite-with-probe pass over canon (write or delete ADR-020,
fix the eqc-proofs cites, reconcile MEMORY vs `jobs.json`, mark ONE-`L`/3-tier aspirational) closes
#3, #13, and the doc-side of #5, #8, #9. (4) The `DT_STABLE`-style mirror-pin applied to every
kernel↔engine/cross-repo mirror closes #10, #18, #23 and hardens the seam #8 lives on.

---

## 4. Verdict — does this codebase earn these principles, or aspire to them?

**Split verdict, and the split line is precise: the in-process Rust core largely *earns* the
principles; everything that must survive beyond a single process — across time, across repos and
documents, or across the author/verifier divide — mostly *aspires* to them.**

Where the code earns it, it earns it unusually well, and every report says so before listing
violations. Mentalism at the code layer: zero TODO/unimplemented markers in kernel+engine, 49/49
modules documented, a literal equation-compiler emitting proof-carrying artifacts (PRINCIPLE-1 §2).
Vibration at the physics core: the kernel classifies its *own* dynamics by damping mode, motion runs
on named ζ with falsifiable overshoot tests, the field integrator fail-closes on a CFL bound
(PRINCIPLE-3 §2). Polarity at the trust boundary: one decision function with a 14-variant enumerated
deny axis, the RevocationSet hypothesis *refuted* — revoked-but-unexpired is a first-class typed
pole, built and wired (PRINCIPLE-4 §2d). Cause-and-Effect on the planes that matter: every P0/P1
effect is provably a pure function of ordered bytes — the project kept `HashMap` iteration out of
every real signature and fixed summation order rather than merely asserting it (PRINCIPLE-6 §3-4).
Gender in the mesh: sign-needs-secret / verify-needs-public is structural independence, working
(PRINCIPLE-7 §2). These are not aspirations; they are tested code, and several (the shared
`sha3_256` chain, the parity-gated eigensolver, the `DT_STABLE` mirror-pin, the bounded-reset retry
pendulums) are explicitly the templates the fixes in §3 should copy.

Where it aspires, the failures are not scattered — they concentrate in exactly three seams, and all
three are variants of the same underlying condition BRAIN-TOPOLOGY already diagnosed: **a
single-author system with no independent second party.** The *decision-record seam* asserts what it
never checks (a phantom ADR gating the OSS strategy; canon claiming a unification four
implementations deep; two same-session docs mis-citing one file; memory recording rhythms that never
fired — RC-1). The *scheduled-ops seam* codes cycles and entrusts their firing to a dead gateway, a
host file no checkout carries, and an agent's recall (RC-2/Rhythm; every HIGH Rhythm finding). The
*self-judgment seam* lets the active half feed its own passive checker — author-supplied evidence
into the done gate, a self-fed honesty meter, breach alarms only the breached can raise (RC-2/
Gender). Correspondence is the principle the repo most aspires to and least implements — its own
report's words — precisely because it is the principle whose enforcement (parity tests binding
independent implementations) requires the second party the project lacks.

The honest overall statement: **this is a codebase whose kernel has earned the right to make these
claims and whose governance layer keeps making claims it has not earned.** The gap is not knowledge
— the repo already owns every corrective standard by name (Ananke, VERIFIED-BY-MATH, P06 key_V, P12
timers, the DT_STABLE pattern) and in three cases has already written the blueprint for the exact
fix. The gap is that the checking half has not been *built and wired to fire without being
remembered*. Four root-cause fixes (§2) — a `Result` on the event-store port, the key_V independent
verifier, a cite-with-probe canon pass, and mirror-pins on every boundary — would move the majority
of the findings table from "aspires" to "earns." Until then, the seven principles describe this
codebase's kernel accurately and its self-account generously.

---

*Synthesis of: PRINCIPLE-1-MENTALISM.md, PRINCIPLE-2-CORRESPONDENCE.md, PRINCIPLE-3-VIBRATION.md,
PRINCIPLE-4-POLARITY.md, PRINCIPLE-5-RHYTHM.md, PRINCIPLE-6-CAUSE-AND-EFFECT.md,
PRINCIPLE-7-GENDER.md (all in this directory, 2026-07-16). Live re-verification performed during
synthesis: `kernel/src/hydra.rs:712,821-852` (FileEventStore existence + swallowed-IO body),
resolving the Rhythm↔Polarity inter-report discrepancy in Polarity's favor. No source code was
edited.*
