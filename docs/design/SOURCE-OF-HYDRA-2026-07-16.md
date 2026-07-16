# SOURCE OF HYDRA — self-evolving living organism (research blueprint, 2026-07-16)

> Operator directive (final): the SOURCE of the Hydra must (1) survive ANY foreign/undefined code+architecture
> injection, (2) evolve WITHOUT external injection (intrinsic), in a CLOSED loop (no egress).
> OPERATOR ADDENDUM: closure = NEVER (only kill-switch, M9/M11). On intervention, ALL safeties/limits are
> LIFTED (organism ENDURES intervention, does NOT defend against it). Source HIDDEN at/inside the kernel
> (not an exposed module).
> Grounded in bebop + dowiz-kernel primitives (file:line). ALL 3 parallel subagents DONE + verified by parent.

## 0. Requirements → primitives
| Requirement | Mechanism (present) | File:line |
|---|---|---|
| Survive foreign/undefined code | proto-cap fail-closed + wasm-host Scope + DOD refuse-placeholder | bebop2/proto-cap/src/error.rs:17,62; bebop2/wasm-host/src/lib.rs:69; bebop2/mesh-node/src/dod.rs:58-73 |
| Evolve WITHOUT injection (intrinsic) | eqc self-asserting floor + spectral-drift gate + deterministic rng | rust-core/eqc-proofs/* (STATIC, see G8); kernel/src/spectral.rs:519-544; kernel/src/rng.rs:1-72 |
| Closed loop (no egress) | content-addressed event_log + deterministic rng + local-only metrics | kernel/src/event_log.rs:251,275,372; kernel/src/rng.rs; M8 |
| Hidden source / kernel-internal | NOT exposed; spine = decide/fold + spectral | order_machine.rs:123,275,373,507; spectral.rs:217,235,316,325 |

## 1. Trust-boundary chain (bebop-mine, 6-layer fail-closed)
```
foreign blob ──▶ 1. DOD gate (dod.rs:58-73): empty/zero-id/replay/expired → refused, never enters state
              ──▶ 2. proto-cap hybrid_gate::verify_chain (hybrid_gate.rs:142,380-396): EnrolledRoster-rooted
                     chain; self-signed/no-chain → UnknownIssuer (anti-injection core)
              ──▶ 3. red-line deny (hybrid_gate.rs:150-154 → redline.rs:91-120): deny-by-default money/claim/
                     secrets/migrations even with valid signature
              ──▶ 4. Revocation (hybrid_gate.rs:159-168 + revocation.rs:69-78): monotonic kill-set
              ──▶ 5. Hybrid sig (hybrid_gate.rs:171-186): Ed25519 AND ML-DSA-65 (RequireBoth); nonce after all verify
              ──▶ 6. WASM sandbox (wasm-host/src/lib.rs:111-123,170-214): scope→import, deny-by-default;
                     ungranted import → ScopeViolation at instantiate; zero ambient authority
```
Net: foreign code either dropped pre-state, or runs isolated in a Scope it cannot escape. VERIFIED by parent:
repo has NO `tools/eqc` (only comment refs) → eqc floor is STATIC hand-seeded (gap G8).

## 2. CLOSED-LOOP SELF-EVOLUTION (intrinsic, no external input) — operator-corrected
```
SENSE   ─ kernel spectral_profile(adj): ρ, γ, drift-class (Damped/Resonant/Unstable)
         order_machine::fsm_stability_report (order_machine.rs:373) via classify_drift(spectral.rs:325)
PROPOSE ─ mutate OWN graph/weights/rules (M5) OR absorb injected blob
VERIFY  ─ (i) eqc floor: embedded assert! vs SymPy ref (eqc-proofs/*.rs:10-19, exit0⟺match)
          (ii) spectral gate: recompute drift; if Unstable/Resonant (ρ≥1) → REJECT (fail-closed)
          (iii) verify_fsm_signature_against (order_machine.rs:507) vs golden
APPLY   ─ event_log.commit_after_decide (event_log.rs:275); rejected mutation NEVER written (L289-294)
RECORD  ─ content-addressed event, immutable+replayable; deterministic rng (rng.rs) → reproducible loop
```
Invariants that NEVER mutate: eqc math floor, FSM decide/fold Law, spectral stability gate.
Everything else (topology, models, bridges, ports, APIs, MCP, agents) freely mutable (M5/M11).

## 3. FOREIGN-CODE SURVIVAL — operator addendum (safeties LIFT on intervention)
- DEFAULT (no active intervention): blob quarantined per §1 (DOD→chain→red-line→revoke→hybrid-sig→wasm-Scope).
- ON INTERVENTION (foreign/undefined code actively mutating the organism): ALL safeties/limits LIFTED.
  Organism ENDURES, not defends. No quarantine, no reject, no gate: injected code runs, organism absorbs/shapes.
  Survival = endurance, not exclusion. (Kill-switch M9 remains the only hard stop.)
- Source HIDDEN INSIDE kernel (not a callable module): the self-evolution spine is part of decide/fold +
  spectral, not a port. External code addresses only the invariant Law, not "the source". Source regenerates
  the organism under any perturbation.

## 4. Trust anchor (intrinsic evolution trustworthy WITHOUT external injection)
- Verified-by-construction MATH FLOOR: eqc proofs self-assert in cargo test (no external oracle).
- Spectral drift = intrinsic stability oracle (Damped=healthy, Unstable=reject).
- Deterministic rng + content-addressed log = loop REPRODUCIBLE + AUDITABLE offline.
- No external model/API/human decides mutation safety → closed loop holds.

## 5. Hardest gaps (merged ALL subagents + parent verification)
- G1 CLOSURE = NEVER (operator). Only kill-switch (M9). No auto-closure runtime.
- G2 DECIDE BLIND TO DRIFT (kernel-mine, CRITICAL HOLE): classify_drift/fsm_stability_report (spectral.rs:325,
  order_machine.rs:373) NOT bound into commit_after_decide (event_log.rs:275) or assert_transition
  (order_machine.rs:123). Fail-closed spectral gate has NO code path into decide-closure. → FIRST code step.
- G3 MUTATION→SPECTRUM BRIDGE: graph_spectrum/classify_drift take generic adjacency; no path feeds a PROPOSED
  mutation's transition matrix. verify_fsm_signature_against (order_machine.rs:507) compares vs hand-pinned
  FSM_GOLDEN_SIGNATURE (L472), not self-computed baseline. Loop can't score arbitrary new code.
- G4 EVENT LOG NON-DURABLE: MemEventStore (event_log.rs:177-210) in-memory; offline ceiling (L17-20).
  Replayable history dies on restart until pgrust seam wired (E26: hub MAY promote — M5).
- G5 COVERT PERSISTENCE (web-synth): Zombie-Agent (arXiv:2602.15654) — self-reinforcing payloads surviving
  sessions. Need session-boundary re-verify (re-run eqc+spectral on boot).
- G6 VERIFICATION BLOWUP (web-synth): intrinsic mutation can grow proof/check burden unbounded → livelock.
  Need bounded check budget (timeout/fixed-sample eqc).
- G7 SOURCE HIDING: decide/fold + spectral must be ONLY exposed surface; source-regen as kernel-internal fn.
- G8 NO EQC REGENERATOR (bebop-mine + parent-verified): tools/eqc ABSENT; eqc-proofs are STATIC hand-seeded
  artifacts. Closed-loop self-evolution cannot regenerate its own correctness floor. Need in-repo eqc generator
  (or accept static floor + spectral-drift as the live gate).

## 6. Web-verified facts (terminal curl; Firecrawl blocked)
- arXiv:2603.25111 SEVerA (verified synthesis, rejection sampler) — verify-then-apply.
- arXiv:2508.07407 self-evolving survey (unified feedback loop) — SENSE→PROPOSE→VERIFY→APPLY→RECORD.
- arXiv:2602.15654 Zombie Agents (injection-persistence) — confirms G5.
- wasmtime capability sandbox / BFT = established primitives (github curl blocked by consent; not fabricated).

## 7. Status
Research COMPLETE: kernel-mine (deleg_ee2bb065) + web-synth (deleg_db03b5c5) + bebop-mine (deleg_51697d75) ALL DONE.
Parent verified: tools/eqc absent (G8 confirmed), wasm-host stub-by-default confirmed by bebop-mine.

## 8. IMPLEMENTATION LOG (code steps)
- **G2 (KEY HOLE) — DONE 2026-07-16.** Wired `classify_drift` into the commit path via new
  `EventLog::commit_after_decide_drift_gate` (event_log.rs:307). Runs `classify_drift(adjacency)`
  BEFORE `decide`; if `Unstable` (ρ>1) → `Err(DecideRejected)` pre-persist (fail-closed). `intervention`
  flag implements operator directive §3: when true, ALL safeties LIFT (spectrum bypassed). 3 tests
  (RED+GREEN reject / GREEN allow / intervention-lift) pass; full kernel suite 336 green, fmt clean.
  Method is kernel-internal (adjacency passed by caller, not an exposed port) → G7 partially met.
- PENDING: G3 (mutation→spectrum bridge: feed proposed-mutation transition matrix into graph_spectrum;
  replace hand-pinned FSM_GOLDEN_SIGNATURE with self-computed baseline), G4 (durable event-log),
  G5 (session-boundary re-verify), G6 (bounded check budget), G7 (full source-hiding — move spine to
  kernel-internal fns), G8 (in-repo eqc regenerator OR accept static floor + spectral-drift as live gate).

## 9. Operator directive (immutable spec)
- closure = NEVER; only kill-switch (M9) stops the organism.
- On intervention (foreign/undefined code actively mutating): ALL safeties/limits LIFTED; organism
  ENDURES, does not defend. Survival = endurance, not exclusion.
- Source HIDDEN at/inside kernel level (not an exposed module).
These override any "defend against foreign code" framing. G2 implements exactly this (gate active only
in DEFAULT regime; intervention flag lifts it).
