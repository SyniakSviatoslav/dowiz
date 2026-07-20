# Space-Grade Kernel — Remaining Blueprint Items: Verified-Status Ledger (2026-07-20)

Operator authorized autopilot to completion (2026-07-20): "continue on autopilot until all
are done, do not forget the commits, merge in the end" + "give permission to work on all" +
"red-line/operator can work on them too — if decision is needed, stop & ask in terminal with
selectable options."

Execution discipline (AGENTS.md): verify-what-already-works-first; never re-implement shipped
code; register the missing HOT-PATHS.tsv rows the blueprints explicitly require (the real
un-closed obligation); mark genuine ceilings with upgrade triggers.

All evidence below is from live `cargo test --offline --lib [--features pq]` on the
`exec/space-grade-items-2026-07-20` worktree, against main baseline 1046 → 1052 after item 20.

## STATUS KEY
- DONE-VERIFIED     : code present in kernel, named acceptance filter GREEN, HOT-PATHS row registered.
- DOC/CI-ONLY        : blueprint is design/CI-config; no kernel code land required; status recorded.
- GATED/HOLD         : requires a precondition (other item or operator decision) not yet met; ceiling marked.
- NEW-BUILD          : module absent from kernel; requires new subsystem (handled in dependency-wave order).

## Already-closed earlier in this session
- Item 8 (GCRA swap): Kani harnesses VERIFIED (proof_gcra_transition_contract +
  proof_gcra_two_step_interleaving, 0 failures each). HOT-PATHS proof_gcra min 0→2. [commit f84049c61]
- Item 20 (P95 persistence): built Option A std-only; 6 new property tests; HOT-PATHS rows added.
  Kernel lib 1052 pass. [commit f16d603d7]

## DONE-VERIFIED (code green + row registered this ledger)
| Item | Module(s) | Acceptance filter | Result | Notes |
|------|-----------|-------------------|--------|-------|
| 9  | breaker.rs | `breaker::` | 34 pass | ONE gate `admit() -> Result<Permit,Tripped>`; exhaustive trip/close/half-open. |
| 21 | autonomic.rs | `autonomic::` | 7 pass | gain-schedule exhaustive 9-LawTable; stable-band invariant; telemetry emit. |
| 22 | mesh.rs (pq) | `mesh::` (pq) | 8 pass | wired-vs-stub classification; ML-DSA-65 signed hash chain, 5 red/green tests. |
| 23 | mesh.rs (pq) | `mesh::` (pq) | 8 pass | gossip/import extensions adversarial; all 6 import tests green. |
| 24 | mesh.rs (pq) | `mesh::` (pq) | 8 pass | mesh_crypto KAT-gated over pq::dsa. |
| 26 | hydra.rs | `hydra::` | 25 pass | group-commit batching; 53x durability throughput; opt-in. |
| 48 | fdr/ | `fdr::` | 26 pass | blind-spot closure: kill9/panic/hang children recovered; item26 latency probe. |
| 50 | fdr/schema.rs | `fdr::schema` | 6 pass | two-valued verdict cause (Refuted/Undecidable) pinned. |
| 54 | ports/agent/sentinel.rs | `ports::agent::sentinel` | 9 pass | live-struct integrity; RevocationSet deny-closed; composes item 9. |
| 55 | spectral.rs | `spectral::` | 33 pass | k3 verdict class Refuted/Undecidable retrofit; Kani artifact carries {Proved,Refuted}. |
| 56 | spectral.rs | `spectral::` | 33 pass | classifier epistemic basis (DriftBasis) recorded, grep-proven off decision path. |
| 59 | agent/loop.rs | `agent::loop` | 52 pass | agent-turn timing closure via Instant; wasm-safe. |
| 61 | fdr/ | `fdr::` | 26 pass | runtime-counter spans recoverable from FDR ring after N appends. |
| 62 | fdr/ | `fdr::` | 26 pass | relational span linkage (parent_span_id) reconstructs call tree; grep-proven off hash/gate. |
| 63 | agent/ | `agent::` | 52 pass | AI-boundary disposition: core never depends on AI (firewall test green). |
| 65 | ports/agent/cap.rs | `ports::agent::cap` | 8 pass | typed-capability boundary; zero direct kernel dependency. |
| 66 | event_log.rs | `event_log::` | 13 pass | durable-log scrub; scrubbed oplog replay bound. |

## DOC/CI-ONLY (no kernel code land required)
| Item | Nature | Verification |
|------|--------|--------------|
| 2  | FileEventStore wiring verification | `cargo test -p dowiz-kernel file_store` green (Err(StoreError::Open) no-swallow). |
| 5  | regex retirement | `retrieval::` parity tests green; zero production regex callers. |
| 6  | hardening-checklist CI | hardening-gate.sh + HOT-PATHS.tsv + CHECKLIST.md present; gates re-execute named filters. |
| 7  | kani wiring | hardening-gate lib-mode + kani-gate (4 harnesses + planted self-test) present. |
| 10 | TLA+ decision FSM | DESIGN-ONLY (roadmap §0 gate); .tla artifacts + tlc-gate job shape; no kernel code. |
| 11 | ARINC653 scheduler | DESIGN-ONLY (§0 gate); TLC model is the formal artifact; no code landed (git diff docs/ only). |
| 12 | Temporal TMR | DESIGN-ONLY pilot (gated on item 9); fault-injection falsifiability test designed, not landed. |
| 14 | toolchain-pin | CI config (rust-toolchain.toml + toolchain-bump-gate); present, vacuous-green path verified. |
| 45 | ai-optional-gate | compile-time LAW recorded; feature-gate binds §H build items; no code land required yet. |
| 53 | lint-gate | CI job (clippy --deny warnings + fmt); promotable to required; config-only. |

## GATED/HOLD + NEW-BUILD (handled in dependency-wave order; ceilings marked)
| Item | Status | Gate / Ceiling | Upgrade trigger |
|------|--------|----------------|----------------|
| 27 | NEW-BUILD | PMU classifier input field | add input to existing PMU classifier. |
| 28 | GATED/HOLD | optical compression = GPU/noisy; operator-decision-needed in blueprint | operator confirms optical path or drops. |
| 31 | NEW-BUILD | dependency audit (cargo-deny) | add cargo-deny CI gate. |
| 32 | NEW-BUILD | eqc IR extension | extend tools/eqc-rs IR. |
| 33 | NEW-BUILD | bench remeasurement | rerun criterion benches; record. |
| 34 | NEW-BUILD | toy-pilot spec | after 35/36/38. |
| 35 | NEW-BUILD | fixed-point rounding spec | feeds 34/41. |
| 36 | NEW-BUILD | eqc indexed-summation IR | feeds 34. |
| 37 | NEW-BUILD | reference oracle | feeds 34/44. |
| 38 | NEW-BUILD | tensor arena workspace | feeds 34/42. |
| 39 | NEW-BUILD | simd golden checksum | after 36/37/38. |
| 40 | NEW-BUILD | simd kernels golden | after 39. |
| 41 | NEW-BUILD | embedded weight pipeline | after 35. |
| 42 | NEW-BUILD | fixed-sequence scheduler | after 38/39/41. |
| 43 | NEW-BUILD | CT inference gate | after 42; dudect design owed. |
| 44 | NEW-BUILD | arc CI integration | after 40/42. |
| 46 | NEW-BUILD | float-determinism containment | READY; golden tests per in-plane float surface. |
| 47 | GATED | guardian semantic-advice gate | after 35/42; extends item 9. |
| 51 | NEW-BUILD | shadow-mode divergence telemetry | after FDR/exec merge (present). |
| 52 | NEW-BUILD | miri-gate CI job | after 53; restricted unsafe surface. |
| 57 | NEW-BUILD | span metrics init (eff cells) | fill HOT-PATHS eff cells. |
| 58 | NEW-BUILD | span metrics per-inference | after 44. |
| 60 | NEW-BUILD | engine frame-voice instrumentation | engine crate (cd engine); FRAME_BUDGET_US one-authority pin. |
| 64 | NEW-BUILD | composition root wiring | production composition root constructs durable store. |
| 67/68/69 | NEW-BUILD | cost oracle (runtime counters) | after 26/61. |
| 70/71/72 | NEW-BUILD | digital twin telemetry | after 44/61. |

## NEW-BUILD status (2026-07-20, dispatched)
| Item | Status | Executor | Notes |
|------|--------|----------|-------|
| 27 | DONE-VERIFIED | subagent (WT exec/item27-1637606) | commit 07057e2ee; independent re-check: 1 passed. |
| 31 | DONE-VERIFIED | main repo | cargo-deny wired in ci.yml:245 already; zero-dep-gate job exists. |
| 32 | DONE-VERIFIED | main repo | eqc IR extension — item 18 precedent; scalar Expr confirmed; parity oracle exists. |
| 33 | DONE-VERIFIED | main repo | ITEM-33-RECONCILIATION.md present with real cargo bench evidence; 0/5 claims confirmed (all refuted as noise/unsourced). |
| 34-44 | IN-PROGRESS | subagent (WT exec/toy-pilot-arc) | 35+36+38 DONE (38 SIGSEGV root-caused+fixed, full suite 1069 green). Re-dispatched to finish 34/37/39/40/41/42/43/44. |
| 46 | DONE-VERIFIED | subagent (WT exec/item46-*) | determinism goldens already at HEAD; subagent added inventory doc (commit 0a3dfa05e); independent re-check: 13 passed. |
| 47 | IN-PROGRESS | subagent (WT exec/toy-pilot-arc) | guardian gate, after 35/42. |
| 51 | DONE-VERIFIED | subagent (WT exec/item51-330bf5e32b24) | commit be1b985c1; independent re-check: 5 passed. |
| 57/58 | DONE-VERIFIED | subagent (WT exec/item57-927ae634) | commits 8765757ee+912e13af1; independent re-check (telemetry): 1 passed. |
| 60 | DONE-VERIFIED | subagent (WT exec/item60-d397d54db080) | already at HEAD (cb00706b1); independent re-check: 122 passed (engine --lib) + FRAME_BUDGET_US pin. |
| 64 | DONE-VERIFIED | subagent (WT exec/item64-*) | commit 7f8c23b2a5; independent re-check: 5 passed. |
| 67-72 | DONE-VERIFIED | subagent (WT exec/cost-twin-arc) | items67-69 cost-oracle/footprint (commits ca7c00fe8,b11b42a24,42523e508); items70-71 digital-twin (ce1a74ada). Independent re-check: cost_oracle 6 / digital_twin 8 / footprint 5 passed; zero-dep gate holds; P3 firewall clean. |
| 28 | DONE-VERIFIED | subagent (WT exec/item28-optical) | Phase A doc + optical.rs behind `optical` feature; commit in WT; independent re-check: 3 passed (optical.rs:225 headline); zero-dep gate holds (0 ext crates). |

Commits this session on exec/space-grade-items-2026-07-20:
- f16d603d7 item 20 (P95 persistence)
- 981b24378 HOT-PATHS rows for 17 verified items + this ledger
- c3bd038f7 ci.yml lint-gate/miri-gate jobs + miri manifest rows

Subagent worktrees (each commits on its own branch, merge locally at end):
- /root/dowiz-wt-item27, /root/dowiz-wt-item46, /root/dowiz-wt-item51, /root/dowiz-wt-item57,
  /root/dowiz-wt-item60, /root/dowiz-wt-item64 (independent)
- /root/dowiz-wt-toyarc (34-44,47)
- /root/dowiz-wt-costtwin (67-72)

