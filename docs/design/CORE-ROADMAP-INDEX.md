# CORE-ROADMAP INDEX — the one navigation root for all dowiz/bebop2 planning (2026-07-17)

> **This is the WHERE. The WHAT/WHEN is
> [`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md)
> (canonical roadmap, execution phases P01–P30, §9 there points back here).** Every planning
> document in the corpus is reachable from this file in ≤2 hops; a plan not listed here does not
> exist for navigation purposes (add a row when adding a doc). Built by the Layer-I consolidation
> pass ([`CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md`](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md));
> honesty rule inherited from it: rows marked **OPEN** or **MISSING** are never turned into links
> to files that don't exist.
>
> **Numbering, once and canonically:** `P01–P30` is the sole execution numbering (P01–P19 =
> numbered blueprint files in `sovereign-roadmap-2026-07-16/`; **P20–P30 exist only as standalone
> blueprint files indexed from SOVEREIGN §8.1–§8.12** — auditing "the P0x files" alone misses a
> third of the roadmap). `Layer A..I` is an orthogonal **altitude axis** over clusters of those
> phases — formerly spelled "P-A..P-I", renamed to kill the P-D/P04 lexical collision (ruling:
> P-I audit §4). On-disk `BLUEPRINT-P-X-*.md` filenames keep their provenance names.

---

## 1. Crosswalk — Layer A..I ↔ P01–P30 (the anti-double-numbering artifact)

A Layer *rolls up* numeric phases + standalone blueprints + research batches; it never replaces
them. Layer I maps to zero numeric phases; several phases appear under two layers (a lens, not a
partition).

| Layer | Altitude scope | Rolls up (numeric phases · standalone blueprints · batches/arcs) |
|---|---|---|
| **A** | Core kernel primitives — equations-not-primitives, tensor/sparse/branchless memory | P04 · P11 · P28 · eqc-rs wiring (`geo.rs`/`domain.rs`) · [BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md](BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md) · [BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md](BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md) · masterwork Batches 1+8 · P30 lanes W1-L1/L5/L10 |
| **B** | State/consistency + living memory — event log, content-hashing, snapshots/epochs | P07 (exactly-once port — LIVE money-red-line bug) · P12 · P28 snapshot seam · P30 W1-L2/L11 · masterwork Batch 2 + doc 19 |
| **C** | Safety / self-healing / self-terminating — breakers, invariants, authority boundary | P10 (adjacency) · P27 · masterwork Batch 3 + doc 19 Part 2 · hydra hysteresis + restart-intensity (T-6/W3-L4) |
| **D** | Consensus / trust / capability — Sybil-resistance, DecisionUnit gossip, capability issuance | P03 · P06 · P10 · P14 · P29 §2 (Decision Compiler) · masterwork Batches 4/6/7 · R-3 `RootDelegationPolicy` docket |
| **E** | Network / hardware / crypto-in-core | P03 · P09 · masterwork Batch 5 v2 (target-corrected) · W2-L5 AVX2 parallel-independent-verify (batch-accept REJECTED on a real SSR-2020 forgery) |
| **F** | Local AI / MoE mesh | P21 · P15 (E13-cpu) · P29 · masterwork doc 21 (distributed-inference rejection) · `harness-2026-07-16/HARNESS-LLM-BACKEND.md` |
| **G** | Product/UI on kernel — wasm bridge, physics-UI, greenfield `web/` beachhead | P16 · P17 · P20 · [living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md](living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md) · masterwork Batch 9 |
| **H** | Ops / telemetry / benchmarks / regression | P01 (`ci.yml:23` fix) · P08 · P24 · P25 (· P26 soft) · chaos harness · `docs/regressions/REGRESSION-LEDGER.md` |
| **I** | Cross-repo consolidation (meta — this index, banners, fold-ins) | **no numeric phase** — proof the letters are a lens, not a renumbering |

Cross-cutting blocker, unchanged: **P06 `key_V`** gates Layer C's independent-verification leg,
Layer D's capability issuance, Layer G's product-safety story, E3-Phase-B (spectral arc), and P30's
signed DecisionUnit import (MEMORY: `sovereign-architecture-19-phase-roadmap-2026-07-17.md`).

## 2. Layer blueprints (Wave 2 + Wave 3, `CORE-ROADMAP-2026-07-17/`)

| Layer | Blueprint | One line |
|---|---|---|
| A | [BLUEPRINT-P-A-kernel-primitives.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md) | eqc-rs equation wiring + eig2x2 dedup + normalize-before-hash groundwork; IS masterwork Wave 1 formalized to the 20-point contract |
| B | [BLUEPRINT-P-B-state-consistency.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-B-state-consistency.md) | The three correctness closures of tile→normalize→hash→snapshot: exactly-once `commit_after_decide` port, hash-canonicalization as a type invariant, drift-gated snapshot admission |
| C | [BLUEPRINT-P-C-safety-self-healing.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-C-safety-self-healing.md) | `integrity_check` hysteresis band + restart-intensity as a launch-path predicate; finite-anchored-authority doctrine applied |
| D | **OPEN — not written.** Blocked on the R-3 `RootDelegationPolicy` operator ruling; its Wave-1 audit is MISSING (§3) — surviving quotes: `BLUEPRINT-P-E` §1 (IssuanceBudget seam, `P-D-audit:131-135,107,217`) | — |
| E | [BLUEPRINT-P-E-network-crypto-core.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-E-network-crypto-core.md) | AVX2 SIMD crypto-verify lane (parallel-independent-verify, NEVER batch-accept); crypto is 8–20× the whole packet stack |
| F | **OPEN — not written.** Was pending the MoE-specific masterwork redo; interim grounding: doc 21 + P21/P15/P29 rows in §1 | — |
| G | [BLUEPRINT-P-G-product-ui.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-G-product-ui.md) | Greenfield `web/` build-out (NOT a migration): wire `FieldSim`, bind the remaining 21 kernel exports, first real DOM surface; money-flip explicitly gated out |
| H | [BLUEPRINT-P-H-ops-telemetry.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-H-ops-telemetry.md) | Chaos/fault-injection harness + the one real CI bug (`ci.yml:23`) + benchmark CI gate + ledger migration; P24/P25 folded in by reference, not re-derived |
| I | [BLUEPRINT-P-I-consolidation.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md) | This consolidation, formalized + executed: banners, fold-ins L1–L6, this index, the Layer ruling |

## 3. Wave-1 audits (Opus, ground-truth passes)

| Audit | Status |
|---|---|
| [P-I-audit-cross-repo-consolidation.md](CORE-ROADMAP-2026-07-17/P-I-audit-cross-repo-consolidation.md) | ON DISK — the authority behind §§5–8 of this index |
| `P-D-audit-root-delegation-policy.md` | **MISSING ON DISK** (verified 2026-07-17 — quoted with line numbers by `BLUEPRINT-P-E` §1, then lost; dir was untracked, no git recovery). Do NOT re-run blind: harvest the surviving quotes first |
| `P-G-audit-product-ui-post-decommission.md` | **MISSING ON DISK** — its scope statement and G1–G3 gap table survive adopted verbatim in `BLUEPRINT-P-G` §0–§1 |
| `P-H-audit-telemetry-regression-benchmarks.md` | **MISSING ON DISK** — its Areas 3–4 verdicts survive in `BLUEPRINT-P-H` §0–§1 and `BLUEPRINT-P-A` §0 (criterion-harness row) |

## 4. The quality contract

[`CORE-ROADMAP-STANDARD-2026-07-17.md`](CORE-ROADMAP-STANDARD-2026-07-17.md) — the operator's
standing 20-point quality bar for ALL future planning ("not a guidance — a quality constant, zero
divergencies"). Every blueprint in §2 above carries a compliance map against it. Its §0 inventory
and §3 header carry the 2026-07-17 Layer-I corrections (6th master doc; Layer naming; P01–P30).

## 5. The mesh-masterwork corpus

[`bebop2-mesh-tensor-hermetic-2026-07-17/INDEX.md`](bebop2-mesh-tensor-hermetic-2026-07-17/INDEX.md)
— navigates the 9-batch tensor/state/safety/consensus/network/equations/product audit, the V1/V2
syntheses (V2 governs), and the correction docs. Roadmap entry point: Phase 30 (SOVEREIGN §8.12).
Not duplicated here — that INDEX is authoritative for its own directory.

## 6. Fold-in ledger (would-be-lost items, all six dispositioned — P-I audit §3)

| ID | Item | Where it lives now |
|---|---|---|
| L1 | Update-blob code-signing (ML-DSA vs pinned root; `kernel/src/pq/codesign.rs` live) | [BLUEPRINT-P10 § Addendum L1](sovereign-roadmap-2026-07-16/BLUEPRINT-P10-hub-runtime-kill-switch-boot.md) |
| L2 | Transport bake-off rationale (Zenoh/Reticulum/TCPCLv4/BIBE; libp2p rejected) | [`../transport-research-2026-07-12.md`](../transport-research-2026-07-12.md) (restored from blob `94e257fe9`), cross-linked from [BLUEPRINT-P09](sovereign-roadmap-2026-07-16/BLUEPRINT-P09-confidential-self-healing-wire.md) |
| L3 | Courier out-of-app notification/wake path | [BLUEPRINT-P13 § Addendum L3](sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md) — delivery semantics dissolved-by-mesh; device-wake kept as a P13 sub-unit |
| L4 | Anonymous `.onion`/Tor tier | E53-form waiver, SOVEREIGN §9.2 (trigger: vendor-node tier ships AND a venue requires anonymity) |
| L5 | "Lost reports" honesty ledger (13 + ~20 reports) | One line, SOVEREIGN §9.2 — closed-as-lost, decisions survive in `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`; never resurrected |
| L6 | Self-development research queue (causal/do-operator · category-theory functorial map · info-geometry · integer laws) | **Cross-track, NOT in P01–P30:** MEMORY → `physics-math-exploration.md` — the operator's always-running growth axis, parallel to the product roadmap |

## 7. Cross-cutting arcs (own consolidated docs, not numeric phases)

| Arc | Doc | Note |
|---|---|---|
| Agentic mesh protocol (B1–B4) | [agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md](agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md) | Authored in worktree `/root/dowiz-agentic-mesh`, merged in (`cabc01f6a`); Wave 0 landed `f30189262`; canon-diffs CD-1..8 operator-merge pending |
| Spectral energy-flow evolution (E1–E3) | [spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md](spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md) | Authored in worktree `/root/dowiz-spectral-evolution`, merged in (`230cc6998`); E1+E2 landed `6bd181a02`; E3-Phase-B gated on P06 `key_V` |
| Living Interface (feeds Layer G / P16) | [living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md](living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md) | In-repo and easy to overlook — listed here so no reader needs prior knowledge of the directory |

## 8. Superseded master roadmaps (historical / audit-trail ONLY — never plan against these)

Six master docs exist (the standard's original inventory said 5; P-I audit corrected it). One is
canonical; the other **five carry SUPERSEDED banners** as of 2026-07-17:

| Doc | Status |
|---|---|
| [MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md) | **CANONICAL** (P01–P30; §9 = consolidation record) |
| [`../../MASTER-ROADMAP-MVP-2026-07-12.md`](../../MASTER-ROADMAP-MVP-2026-07-12.md) (repo root) | Historical — bannered; sourced L1, L2 |
| [MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md](MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md) | Historical — bannered; targets the deleted Node/TS stack; sourced L3, L4, L5 |
| [MASTER-EXECUTION-PLAN-2026-07-13.md](MASTER-EXECUTION-PLAN-2026-07-13.md) | Historical — bannered; ancestor of the Layer A–I altitude spine; its 9 sub-plan dirs are indexed in MEMORY "Active arcs (earlier)" |
| [MASTER-INTEGRATION-PLAN-2026-07-14.md](MASTER-INTEGRATION-PLAN-2026-07-14.md) | Historical — bannered; best-absorbed of the five (zero would-be-lost items) |
| [MASTER-ROADMAP-10-PHASES-2026-07-14.md](MASTER-ROADMAP-10-PHASES-2026-07-14.md) | Historical — bannered; sourced L6 (the cross-track pointer) |

---

*Maintenance rule (from BLUEPRINT-P-I §8): new planning doc ⇒ one new row here, Layer chosen by
which numeric-phase cluster it serves. DoD checks D1–D5 in BLUEPRINT-P-I §5 are the drift alarms.
Index built 2026-07-17 on `feat/p19-growth-engine` (HEAD `b64a2c1c6`).*
