# CORE-ROADMAP INDEX — the one navigation root for all dowiz/bebop2 planning (2026-07-17)

> **This is the WHERE. The WHAT/WHEN is
> [`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md)
> (canonical roadmap, execution phases P01–P56, §9/§10 there point back here — **2026-07-18
> update: extended P31–P46 → P31–P56 across §11–§15**, see §10 there for the phase index).**
> Every planning
> document in the corpus is reachable from this file in ≤2 hops; a plan not listed here does not
> exist for navigation purposes (add a row when adding a doc). Built by the Layer-I consolidation
> pass ([`CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md`](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md));
> honesty rule inherited from it: rows marked **OPEN** or **MISSING** are never turned into links
> to files that don't exist.
>
> **Numbering, once and canonically:** `P01–P30` is the sole execution numbering (P01–P19 =
> numbered blueprint files in `sovereign-roadmap-2026-07-16/`; **P20–P30 exist only as standalone
> blueprint files indexed from SOVEREIGN §8.1–§8.12** — auditing "the P0x files" alone misses a
> third of the roadmap). **2026-07-18 addendum:** SOVEREIGN §10 extends the execution numbering
> with `P31–P46` (ecosystem-component consolidation phases; §10.2 there is the phase index) —
> `P01–P30` semantics unchanged. `Layer A..I` is an orthogonal **altitude axis** over clusters of those
> phases — formerly spelled "P-A..P-I", renamed to kill the P-D/P04 lexical collision (ruling:
> P-I audit §4). On-disk `BLUEPRINT-P-X-*.md` filenames keep their provenance names.
>
> **2026-07-18 addendum (later same day, consistency pass):** SOVEREIGN §11–§14 extend the
> numbering further to **P47–P53** (payment rails, owner hub, customer identity, compliance
> gate, open map/routing, courier surface, Tor/onion) — every "P01–P46"/"P31–P46" range in this
> file predates that extension; §10.2 there is now the full P31–P53 index. Same day, the wave
> swarm landed first real code for P40/P41/P42/P47/P49 (+G3 render shell), so §0's AGENT
> "executor loop … is 0%" line is historical — see SOVEREIGN §10.2's corrected status cells.

---

## 0. The Ecosystem-Component axis (2026-07-18)

Co-equal in importance to the Layer A–I altitude axis (§1) and the P01–P46 numeric axis — a
THIRD lens for navigation, not a replacement for either. Ask "which ecosystem part" first (this
table), then "which phase" (§2 below or MASTER-ROADMAP §10.2), then "which file" (per-phase
Blueprint links) — three lenses, same underlying phases. Full component sections with per-phase
DoD/anti-scope: [SOVEREIGN §10.5](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md). Row
order IS the critical path.

| Component | Mission | Phases | One-line status (live-verified 2026-07-18) |
|---|---|---|---|
| **CORE** | decide/fold Law, event-log, money, capability primitives, spectral math, self-tuning control loops | P31–P33 | ~90% done, not the bottleneck; dominant failure mode is built-but-unwired code |
| **PROTOCOL** | mesh, capability issuance, crypto, transport, delivery-domain (bebop2) | P34–P36 | ~70% built and PROVEN but 100% stranded from dowiz's own kernel — the single biggest lever in the whole roadmap (P34 = #1 next action) |
| **DELIVERY** | the dowiz product surface: UI, order/courier/payment flow, auth, demo/marketing, app-shell | P37–P39 | ~0% deployable (no HTTP server, no rendered UI, no live deployment) but math/domain logic mostly done in CORE+PROTOCOL — wiring-heavy, not from-scratch |
| **AGENT** | local/network AI, tool-use loop, MCP; three operating modes (no-AI / local-offline / connected) | P40–P42 | substrate (LlmBackend/Ollama) shipped; executor loop connecting it to anything is 0% — a chat backend today, not an agent |
| **ECOSYSTEM/OPS** | external integrations, deployment, monitoring, multi-product platform | P43–P46 | explicitly and deliberately LAST — near-zero built, correctly so, since nothing exists yet to integrate/deploy/monitor |

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

Cross-cutting blocker: **P06 `key_V`** gates Layer C's independent-verification leg, Layer G's
product-safety story, E3-Phase-B (spectral arc), and P30's signed DecisionUnit import (MEMORY:
`sovereign-architecture-19-phase-roadmap-2026-07-17.md`). **Correction (P-D audit §3, 2026-07-17):**
the earlier "P06 gates Layer D's capability issuance" edge is **withdrawn** — P06 is a dev-time merge
fence over code diffs; `RootDelegationPolicy` is runtime courier onboarding. They share substrate
(`load_genesis`/`verify_chain`) and the open **C4b** signing-path hardening, but neither functionally
blocks the other; Layer D ships P06-independent.

## 2. Layer blueprints (Wave 2 + Wave 3, `CORE-ROADMAP-2026-07-17/`)

| Layer | Blueprint | One line |
|---|---|---|
| A | [BLUEPRINT-P-A-kernel-primitives.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md) | eqc-rs equation wiring + eig2x2 dedup + normalize-before-hash groundwork; IS masterwork Wave 1 formalized to the 20-point contract |
| B | [BLUEPRINT-P-B-state-consistency.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-B-state-consistency.md) | The three correctness closures of tile→normalize→hash→snapshot: exactly-once `commit_after_decide` port, hash-canonicalization as a type invariant, drift-gated snapshot admission |
| C | [BLUEPRINT-P-C-safety-self-healing.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-C-safety-self-healing.md) | `integrity_check` hysteresis band (LANDED `a50d44ab0`) + restart-intensity as a launch-path predicate; finite-anchored-authority doctrine. **+2026-07-18 fold-in (§0 context, §13):** verification CRITICALs (budget-NaN degrade-open, `noether` fail-open, drift-gate unauthenticated-`intervention`) + round-2 CSC-LAW containment/eviction-breaker — all instances of one law: zero-authority arithmetic is conditional on input totality |
| D | [BLUEPRINT-P-D-consensus-capability.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-D-consensus-capability.md) | **WRITTEN (reconstructed 2026-07-17; index row corrected 2026-07-18 — was "OPEN").** Budgeted anchor-rooted issuance: `OperatorSigned` + monotonic per-anchor `IssuanceBudget` (Option A, buildable now, **P06-independent**), `FirstContactQr`+hw-attestation as an operator-gated overlay (B), `WebOfTrust` deferred on Cheng–Friedman (C). **+2026-07-18 fold-in (§12; was "§11" on its source branch):** the closed gap is now a LIVE red-team HIGH (agentic-mesh A5 unbounded Sybil / B-3 secret-leaking `RefSigner`); A7/B-6 admission red-line-scope-check gap named as a sibling Layer-D item. **R-3 status (merge reconciliation):** blueprint §11 records the R-3 ruling — Option A adopted 2026-07-18 (operator-overridable), mechanism landed `e08eb07` in bebop-repo |
| E | [BLUEPRINT-P-E-network-crypto-core.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-E-network-crypto-core.md) | AVX2 SIMD crypto-verify lane (parallel-independent-verify, NEVER batch-accept); crypto is 8–20× the whole packet stack. **+2026-07-18 fold-in (§0 context, §14; was "§13" on its source branch):** round-2 FEC (`reed-solomon-simd`, L1 QUIC-datagram/L2 BPv7-shard, below-crypto-verify) + the 32-byte `LaneFrameHeader` wire format (no Confidence/CRC, recompute-as-authority) land in Layer E |
| F | [BLUEPRINT-P-F-local-ai-mesh.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-F-local-ai-mesh.md) | Domain experts as DecisionUnit FAMILIES (DomainTag × compiled units × hub-only build-time oracle — operator-confirmed), NOT per-node models; three gossip extensions to P29 §2 (epoch max-merge, key_V-shaped import replay, lineage-in-one-log); checkpoint-STARK deferred with triggers. **+2026-07-18 status note (§13):** the Mistral/local-LLM audit re-confirms every verdict against the live daemon (Mixtral rejected, dense+Ollama stack green, 12+3 tests pass) — zero design change |
| G | [BLUEPRINT-P-G-product-ui.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-G-product-ui.md) | Greenfield `web/` build-out (NOT a migration): wire `FieldSim`, bind the remaining 21 kernel exports, first real DOM surface; money-flip explicitly gated out. **⚠ SUPERSEDED as product-UI track (MASTER-ROADMAP §16.30/§18.4, 2026-07-18): wgpu-canvas-only, zero DOM — the DOM `web/` path is legacy/reference; its money-recompute findings below remain valid.** **+2026-07-18 fold-in (§0 context, §13):** the E1 forged-order-total CRITICAL (STILL LIVE) makes §8's server-recompute an owned Layer-G money-recompute DoD; V3 1.3–1.6 money-arith cluster surfaced as Layer-A fixes with §3 validators as boundary containment |
| H | [BLUEPRINT-P-H-ops-telemetry.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-H-ops-telemetry.md) | Chaos/fault-injection harness + the one real CI bug (`ci.yml:23`) + benchmark CI gate + ledger migration (all LANDED `f4802927e`/`a952af354`); P24/P25 folded in by reference. **+2026-07-18 fold-in (§9):** GitHub-hygiene/versioning docket (CalVer `2026.07.0` + independent in-code `KERNEL_PROTO`/`MESH_WIRE` version, branch cleanup, PAT scope — operator-gated) + GROUND-TRUTH is 41 commits / test-count stale |
| I | [BLUEPRINT-P-I-consolidation.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md) | This consolidation, formalized + executed: banners, fold-ins L1–L6, this index, the Layer ruling. **+2026-07-18 correction (§1):** its §1 G5 "Layer D/F not written" is stale — both blueprints now exist on disk |

## 3. Wave-1 audits (Opus, ground-truth passes)

| Audit | Status |
|---|---|
| [P-I-audit-cross-repo-consolidation.md](CORE-ROADMAP-2026-07-17/P-I-audit-cross-repo-consolidation.md) | ON DISK — the authority behind §§5–8 of this index |
| [P-H-audit-telemetry-regression-benchmarks.md](CORE-ROADMAP-2026-07-17/P-H-audit-telemetry-regression-benchmarks.md) | **RECONSTRUCTED ON DISK (2026-07-17)** — full re-verification fresh on `main @ caba2203c`; supersedes the stale line numbers embedded in `BLUEPRINT-P-H`/`BLUEPRINT-P-A` (delta table in its §6). Confirms the `ci.yml:23` bug and the `FaultyStore` precedent both still hold |
| [P-D-audit-root-delegation-policy.md](CORE-ROADMAP-2026-07-17/P-D-audit-root-delegation-policy.md) | **ON DISK — reconstructed 2026-07-17** (was lost pre-commit; rebuilt by harvesting `BLUEPRINT-P-E` §1's surviving quotes + re-verifying every cite fresh — all still match). Corrects the "P06 gates Layer D" edge (§3): the capability-issuance leg is **P06-independent** |
| [P-G-audit-product-ui-post-decommission.md](CORE-ROADMAP-2026-07-17/P-G-audit-product-ui-post-decommission.md) | **RECONSTRUCTED ON DISK (2026-07-18 correction — was "MISSING")** — the full audit is restored, re-verified fresh on `main @ caba2203c`; its scope statement and G1–G3 gap table also survive adopted verbatim in `BLUEPRINT-P-G` §0–§1 |

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
| L4 | Anonymous `.onion`/Tor tier | **ACTIVATED 2026-07-18** — direct operator request supersedes the old trigger ("vendor-node tier ships AND a venue requires anonymity"; the demand leg is the request itself, the vendor-tier leg is honored by the phase's wave split). Now phase **P53**, SOVEREIGN §14; blueprint [BLUEPRINT-P53-tor-onion-integration.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P53-tor-onion-integration.md) — C-tor sidecar hosting (arti-hosting deferred on the Tor Project's own maturity warnings), Onion-Location + QR convenience layer, mesh-transport-over-Tor deferred with trigger |
| L5 | "Lost reports" honesty ledger (13 + ~20 reports) | One line, SOVEREIGN §9.2 — closed-as-lost, decisions survive in `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`; never resurrected |
| L6 | Self-development research queue (causal/do-operator · category-theory functorial map · info-geometry · integer laws) | **Cross-track, NOT in P01–P30:** MEMORY → `physics-math-exploration.md` — the operator's always-running growth axis, parallel to the product roadmap |

## 7. Cross-cutting arcs (own consolidated docs, not numeric phases)

| Arc | Doc | Note |
|---|---|---|
| Agentic mesh protocol (B1–B4) | [agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md](agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md) | Authored in worktree `/root/dowiz-agentic-mesh`, merged in (`cabc01f6a`); Wave 0 landed `f30189262`; canon-diffs CD-1..8 operator-merge pending |
| Spectral energy-flow evolution (E1–E3) | [spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md](spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md) | Authored in worktree `/root/dowiz-spectral-evolution`, merged in (`230cc6998`); E1+E2 landed `6bd181a02`; E3-Phase-B gated on P06 `key_V` |
| Living Interface (feeds Layer G / P16) | [living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md](living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md) | In-repo and easy to overlook — listed here so no reader needs prior knowledge of the directory |
| Native pgrust tenant-schema rebuild (red-line, RLS) | [BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md](BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md) | Added 2026-07-18 (`f9b2eb9bb`), not yet /council-reviewed. **Deliberately NOT a Layer A-I item** — RLS/migrations/auth are explicitly out of kernel-autopilot scope (per P30's own note, MASTER-ROADMAP §8.12). Proposal-only, 0 code; the reactivation gate for `attic/` tenant-table revival once a server tier exists. Own DoD/anti-scope already in the doc's §0/§4 — no separate blueprint needed |
| Fail-operational / layout-versioning (R1 + round-2: FEC/CSC-LAW/CWR/LaneFrameHeader/DeltaPatch) | [fail-operational-layout-versioning-2026-07-17/round-2/BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md](fail-operational-layout-versioning-2026-07-17/round-2/BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md) | Self-mapped to Layers B/C/D/E (its own §6); folded into those blueprints' 2026-07-18 sections. **RC-2-broad residual pinned open** (B-T4/E-T4); NaN fix owed to Layer B (URGENT — defeats the shipped drift-gate) |
| Session verification / red-team + roadmap refresh (2026-07-18) | [ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md](ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md) | The patch doc consolidating this session's per-repo verification CRITICALs, GitHub/local-LLM audits, and fail-operational R1+R2 into the Layer A–I structure; source of every "+2026-07-18 fold-in" in §2 above. GROUND-TRUTH is 41 commits stale per its §0 |
| hermes-agent-kernel-rewrite (dev-tooling, separate repo) | MEMORY `hk05-hk09-routing-status-2026-07-16.md` | Tracked **out-of-band by design** — Layer A–I is the dowiz/bebop2 kernel-product axis; hermes is in scope only "where cited" (STANDARD §1). No Layer home is warranted (synthesis §1.6). Open: H-0 GitHub-home decision (16 unpushed commits disk-only); T1 group-chatter prompt-injection MED-HIGH |

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

## 9. Previously-orphaned arcs — now absorbed into SOVEREIGN §10 (2026-07-18)

The 2026-07-18 audit's biggest finding: the ten arcs below had **ZERO reference in this index**
before today. Each is now owned by a P31–P46 phase (SOVEREIGN §10.2 = fast phase index;
§10.5.1–5 = full per-phase DoD/anti-scope). MEMORY-only arcs are cited by filename, not linked
(they live outside the repo, in the agent memory corpus).

| Arc dir/doc | Absorbed into | One-line note |
|---|---|---|
| [field-ui-engine/](field-ui-engine/) + [field-ui-engine.md](field-ui-engine.md) | P38a | FE-01..17; math substrate DONE (compose/zerocopy/widget_store/loop_/motion/money_guard), GPU path 0% — gated on the O18a `graphics-unlock` (`cargo add wgpu`) |
| [dowiz-interfaces/](dowiz-interfaces/) | P38b | DZ-01..12 Sea & Sheet surfaces; DZ-10 voice deliberately Phase-9b-deferred, not lost |
| [rust-engine-rewrite/](rust-engine-rewrite/) | P37 + P38a | RW-01..12; RW-02/03/06/07/08 DONE substrate; RW-09 thin-shell rule → P37; RW-01/04/05/10/11 → P38a |
| integration-ports arc (MEMORY `integration-ports-reactive-arc-2026-07-13.md`) | P42 + P43 (+ existing P22) | IP-08→P42, IP-11/12/13/14/19/20→P43, IP-10/15/16→P22 (not renumbered); IP-01/02/03/04/05/07→already covered by P40/P42 (cross-ref fix, resolved 2026-07-19); IP-06→future blueprint candidate (deferred, no GPU pipeline yet); IP-09→P95 (living-memory); IP-17/18→operator-gated crypto red-line; IP-21→verification scaffolding, downstream — see `GAP-A1-DISPOSITION-AUDIT-2026-07-19.md` |
| ecosystem-strategy arc (MEMORY `ecosystem-strategy-arc-2026-07-13.md`) | P44 + P46 | EC-05 + scale-out units → P44 (LOW PRIORITY); EC-17 + remainder → P46 (FURTHEST FUTURE); shared-KernelFacade concept already built in PROTOCOL |
| ops-reliability arc (MEMORY `ops-reliability-arc-2026-07-13.md`) | P45 | OPS-01..22; attic-revival path dead twice over — formally superseded by the native pgrust rebuild (§7 row above) |
| [mesh-real/](mesh-real/) | P34 + P34B | MESH-01..13 (~70% built + PROVEN, 100% stranded pre-P34 — the roadmap's #1 lever); MESH-14 needs one small docs+CI-lint blueprint (its live-test-citation rule folds into Q1) — see `GAP-A1-DISPOSITION-AUDIT-2026-07-19.md` |
| [docker-swap/](docker-swap/) | P35 | DK-01..10; DK-01/02/03/07 DONE with real `wasip2` component + deny-by-default WASI host — previously zero index reference despite working code |
| [math-first-architecture-blueprint.md](math-first-architecture-blueprint.md) | P31 | S0/S1/S2/S4 DONE; S3→P31c, S5→P31d, S6(=B3, counted once)→P31e, S7 parked→PROTOCOL |
| [hydraulic-loop-v2/](hydraulic-loop-v2/) | P32 + P33 | BP-01..23; resonator/DMD wiring gaps → P32b/P32c; BP-03/04/11/12–21/23 unconfirmed-status audit → P33b |

## 10. New phases P47–P56 + operational runbooks (2026-07-18, same-day follow-ups)

Genuinely new work, not absorbed-orphan-arc registration — each links to its blueprint file
directly. Full DoD/anti-scope lives in the blueprint; SOVEREIGN §11–§15 carry the phase-entry
summaries (Absorbs/Status/Role/DoD/Anti-scope/Depends-blocks) in the same template as §10.5.

| Phase / doc | Component | Blueprint | One-line note |
|---|---|---|---|
| P47–P50 | DELIVERY / ECOSYSTEM-OPS | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P47-P50-gap-closing-phases.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P47-P50-gap-closing-phases.md) + [BLUEPRINT-P48-owner-hub-surface.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P48-owner-hub-surface.md) (P48 promoted to its own file) | Payment rails, owner hub, customer identity, legal/first-order gate — all 3 operator rulings RESOLVED (SOVEREIGN §11) |
| P51 | DELIVERY | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P51-open-map-routing.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P51-open-map-routing.md) | OSM vector + field-rendered routes; satellite imagery rejected on cited resolution physics |
| P52 | DELIVERY | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P52-courier-working-surface.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P52-courier-working-surface.md) | Closes the MVP audit's #1 finding — courier had zero owned UI despite the most-built protocol side |
| P53 | DELIVERY | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P53-tor-onion-integration.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P53-tor-onion-integration.md) | Activates fold-in ledger L4; system-`tor` sidecar not arti (arti hosting still experimental) |
| P54 | AGENT | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P54-llm-agent-verification-harness.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P54-llm-agent-verification-harness.md) | LLM/agent adversarial probes; fine-tuning DEFERRED against the operator's own glossary criteria |
| P55 | PROTOCOL/CORE | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P55-protocol-ecosystem-testing.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P55-protocol-ecosystem-testing.md) | Regression taxonomy from 4 real same-session bugs; proptest confirmed already-live (400 cases) |
| P56 | ECOSYSTEM-OPS | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P56-verification-harness-infrastructure.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P56-verification-harness-infrastructure.md) | Shared storage/scheduling/meta-verification substrate for P54+P55; `hetzner:dowiz/test-results/` sync |
| P21 Part 2 | AGENT | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P21-local-llm-hermes-native.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P21-local-llm-hermes-native.md) §11 | Tiered-Intelligence architecture evaluated against operator proposal — Tier-0-model rejected, Ollama stays, model verdict grounded in real workload |
| Disk-ops runbook | ECOSYSTEM-OPS | [CORE-ROADMAP-2026-07-17/BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md) | Operational runbook, not a numbered phase — 90%→65% disk fix executed same pass; `hetzner:dowiz` remote confirmed live |
| §16 Deployment topology | ALL | [MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md §16](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md#16-deployment-topology--operating-model-decisions-2026-07-18-dialogue-pass) | Post-audit dialogue pass: 3-mode hosting (CF Pages/Hetzner/self-host), CF Tunnel remote access, venue-owned couriers confirmed, single assistant-role in-hub agent, unified multi-channel order-flow, isolated-hub MVP topology (dowiz.org=directory), hybrid auto-posting, Fly.io fully retired (teardown blocked on operator `flyctl auth login`) |
| §17 Long-term ecosystem | ALL | [MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md §17](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md#17-long-term-ecosystem-decisions-2026-07-18-dialogue-pass-continued) | Governance (BDFL now, revisable), crypto-agility from day one, 4 dowiz/vendor-forever dependencies found+closed as swappable ports (CF-tunnel account, cert-root, Cloudflare-as-company, Hetzner-as-company) |
| §18 Launch-blocker program | ALL | [MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md §18](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md#18-launch-blocker-research--synthesis--blueprint-program-2026-07-18) | 5 Opus research passes → Fable synthesis → 2 canon-diffs (P38-rev, P39-rev) + 18 Opus blueprints (P57-P74) across 4 build waves to milestones M1-M4; index below |
| §19 Perf/physics/mesh research wave | ALL | [MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md §19](MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md#19-perf-physics-and-mesh-research-wave--status-ledger-registration-2026-07-19) | Roadmap registration of the 2026-07-18/19 wave — points into the three file rows below (ledger/meta-gap/Q-series): four clusters (mesh M1/P92/P93/P94, perf P75–P91, product P95/P96, Q-series governance), the two-lane dowiz ∥ bebop sequence (bebop gate-0 = P85+C3), all LOCAL/UNPUSHED, zero product code, 3 registered code artifacts (I1 NTT process-red, I2 slot_arena, I3 contention-bench) |
| P57-P64 | CORE/PROTOCOL/DELIVERY | [CORE-ROADMAP-2026-07-17/BLUEPRINT-P57-canvas-text-input.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P57-canvas-text-input.md) · [P58](CORE-ROADMAP-2026-07-17/BLUEPRINT-P58-a11y-mirror-everywhere.md) · [P59](CORE-ROADMAP-2026-07-17/BLUEPRINT-P59-capability-cert-chain.md) · [P60](CORE-ROADMAP-2026-07-17/BLUEPRINT-P60-payment-adapter-core.md) · [P61](CORE-ROADMAP-2026-07-17/BLUEPRINT-P61-notification-fabric.md) · [P62](CORE-ROADMAP-2026-07-17/BLUEPRINT-P62-catalog-multivendor-data-model.md) · [P63](CORE-ROADMAP-2026-07-17/BLUEPRINT-P63-shell-platform-spike.md) · [P64](CORE-ROADMAP-2026-07-17/BLUEPRINT-P64-intent-engine-friction-voice.md) | Wave W1 (foundations, build-parallel): text input, a11y-mirror, capability-certs, payment core, notifications, catalog/multi-vendor, shell spike, intent/friction/voice |
| P65-P66, P69-P70 | DELIVERY | [P65](CORE-ROADMAP-2026-07-17/BLUEPRINT-P65-dispatch-orchestrator.md) · [P66](CORE-ROADMAP-2026-07-17/BLUEPRINT-P66-data-wallet-offline-drafts.md) · [P69](CORE-ROADMAP-2026-07-17/BLUEPRINT-P69-customer-storefront-checkout.md) · [P70](CORE-ROADMAP-2026-07-17/BLUEPRINT-P70-owner-surface.md) | Wave W2 (assembly to M1): dispatch orchestrator, data wallet/offline drafts, customer storefront/checkout (M1 critical path), owner surface |
| P67-P68, P71, P73 | DELIVERY/ECOSYSTEM-OPS | [P67](CORE-ROADMAP-2026-07-17/BLUEPRINT-P67-hub-provisioning-claim.md) · [P68](CORE-ROADMAP-2026-07-17/BLUEPRINT-P68-hub-supervisor-update-backup.md) · [P71](CORE-ROADMAP-2026-07-17/BLUEPRINT-P71-courier-surface.md) · [P73](CORE-ROADMAP-2026-07-17/BLUEPRINT-P73-dowiz-org-landing.md) | Wave W3 (delivery+automation to M2/M3): hub provisioning/claim, hub supervisor (update+backup), courier surface, dowiz.org landing |
| P72, P74 | DELIVERY | [P72](CORE-ROADMAP-2026-07-17/BLUEPRINT-P72-foodcourt-checkout-nleg.md) · [P74](CORE-ROADMAP-2026-07-17/BLUEPRINT-P74-moderation-reports-blocklist.md) | Wave W4 (multi-vendor to M4): food-court N-leg checkout, moderation reports+blocklist |
| P38-rev, P39-rev | CORE/DELIVERY | [BLUEPRINT-P38-webgpu-render-engine.md §12](CORE-ROADMAP-2026-07-17/BLUEPRINT-P38-webgpu-render-engine.md) · [BLUEPRINT-P39-app-shell-installability.md §1.2](CORE-ROADMAP-2026-07-17/BLUEPRINT-P39-app-shell-installability.md) | Canon-diffs (write first, unblock the rest): strikes the IME DOM-overlay + adds AR/VR hard requirements; records the winit+wgpu no-webview desktop shell decision |
| Launch-blocker synthesis | ALL | [CORE-ROADMAP-2026-07-17/SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md](CORE-ROADMAP-2026-07-17/SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md) | Fable's reconciled build plan: 12 cross-cutting dependency resolutions (X1-X12), 4 milestones, 4 operator decisions closed, the full blueprint breakdown |
| Launch-blocker research (R1-R5) | ALL | [docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md](../research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md) · [R2](../research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md) · [R3](../research/OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md) · [R4](../research/OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md) · [R5](../research/OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md) | 5 parallel Opus research passes grounding the whole §18 program in real 2026 prior art |
| Master status ledger — 2026-07-18/19 research wave (M1, P75–P94 + all closed scans) | ALL | [CORE-ROADMAP-2026-07-17/MASTER-STATUS-LEDGER-2026-07-19.md](CORE-ROADMAP-2026-07-17/MASTER-STATUS-LEDGER-2026-07-19.md) | The one-glance status of the entire perf/physics/mesh research day: 18 blueprints to write (P75–P83, P85–P91, P93–P94), 2 fully-blueprinted (M1 inside P92, P92 itself), the validated-design vs no-target closed split, the merged dependency sequence (dowiz ∥ bebop lanes; bebop gate-0 = P85+C3), 15 outstanding operator decisions (§5 there), and the §0 recovered-docs incident note |
| Meta-gap audit — 2026-07-18/19 wave (coverage against 8 dimensions) | ALL | [CORE-ROADMAP-2026-07-17/META-GAP-AUDIT-2026-07-19.md](CORE-ROADMAP-2026-07-17/META-GAP-AUDIT-2026-07-19.md) | Second adversarial read of all 21 P75–P96 blueprints: 4 HIGH findings (G1 P95/P96 orphaned from ledger, G2 GPU WebGL2-floor DoD, G3 P81/P82 gate DoDs unmeetable, G4 P91 conformance vectors absent) + G5–G17 hygiene; confirms agent/model verification is a non-gap owned by P54/P55/P56 |
| **Q-SERIES — Roadmap Verification & Observability (governance layer over P75–P96)** | ALL (governance — no numeric phase) | [CORE-ROADMAP-2026-07-17/BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md) | **A cross-cutting Q-namespace (distinct from P-series to signal governance-not-feature)** that ensures each P-item's OWN stated DoD was actually met as it gets built: **Q1** claim-verification checkpoint (new `DONE-VERIFIED` status + `verified-by` evidence field on the ledger; automated backstop = P56 §4f.3 `StaleGround`; would have caught G1/G3/G4 as a standing gate); **Q2** feature telemetry by extending the real closed `LogEvent` enum (`kernel/src/metrics.rs`) + P83 spans (closes G14/G15 — no new logging system); **Q3** code-review gate — honestly mostly EXISTS (ci.yml gates + `v5c-reexec` + per-blueprint `D-REVIEW` + safety-floor); only add = `reviewed-by` pointer, no CODEOWNERS ceremony; **Q4** interface verification — almost the whole wave has no UI, only P86/P87/P88 (cite G2 WebGL2-floor DoD) + P96 (cite G15) touch a surface; reuse P38 render-floor DoD + Playwright, invent nothing. Proportionate: Q1 deep (real gap), Q2–Q4 cite-and-wire |

---

*Maintenance rule (from BLUEPRINT-P-I §8): new planning doc ⇒ one new row here, Layer chosen by
which numeric-phase cluster it serves. DoD checks D1–D5 in BLUEPRINT-P-I §5 are the drift alarms.
Index built 2026-07-17 on `feat/p19-growth-engine` (HEAD `b64a2c1c6`). **Refreshed 2026-07-18** in
worktree `dowiz-verify-redteam` (branch `research/dowiz-verify-redteam-2026-07-17`): Layer-D row
corrected from "OPEN" to the now-on-disk blueprint; P-G-audit row corrected from "MISSING" to
"RECONSTRUCTED"; §2 descriptions updated with each blueprint's 2026-07-18 session fold-in; two
cross-cutting arcs added to §7 (fail-operational round-2, session-verification synthesis) plus the
hermes out-of-band pointer. No P01–P30 renumbering; no Layer added.*
