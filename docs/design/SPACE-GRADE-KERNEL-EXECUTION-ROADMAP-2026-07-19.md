# Execution Roadmap — Space-Grade Kernel Synthesis, Items 1–32, Dependency-Ordered

**Source:** `docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` (commit `10164bd74`).
**Sorting rule:** actual technical dependency, lowest first — not the document's topic order. The
two dependencies the source states explicitly — **item 21 strictly after item 9** (§16(c)) and
**item 23 after item 22** (§17(b) addendum) — are preserved verbatim and never resequenced. Every
other ordering choice below that is not explicit in the source is flagged **[new ordering choice]**
with its reason.

---

## 0. Operator rulings — recorded 2026-07-19, same day as the synthesis

All five open decision gates were presented and ruled on the same day:

| Gate | Source | Ruling |
|---|---|---|
| GCRA lock-free TokenBucket swap | §1.3 / item 8 | **ADOPT** — gated behind the differential oracle + Kani interleaving proof already scoped in item 8; built and tested before it ships. |
| Mesh integration approach | §17(d) / item 22 | **REIMPLEMENT IN DOWIZ, ZERO-DEP** — bebop's proven mesh-node/proto-wire/proto-cap serves as design reference/parity oracle only, not a linked dependency. The same ruling covers `agent-governance-wasm`'s `bebop2-core` path-dep and `mesh-adapter`'s sibling paths per §25's table. |
| Optical/pixel context compression | §20(c) / item 28 | **PURSUE** — model-weight dependencies are ruled outside §0's compiled-Rust-crate scope; archival/display-plane content only, never the P0/P1 determinism planes (§10/P6). |
| ARINC-653-style scheduler | item 11 | **PURSUE, design-only** — Phase 0 (design doc + TLC model), no code until the breaker (item 9) exists, per the source's own restriction. |
| SIHFT triple-vote pilot | item 12 | **PURSUE, design-only for now** — needs the breaker + FDR to exist first regardless of the ruling; design/scoping work can start. |
| eqc indexed-summation IR extension | item 32 | **PURSUE** — extend eqc's `Expr` language to support the Laplacian's neighbor-sum operator, not just scalar control laws. |

The Laplacian reimplement-vs-vendor fork (§14(d)) is **not a live gate** — §26(d)'s correction found
`laplacian_spmv` already exists in-kernel at `csr.rs:552`; the surviving work is the parity pin
(item 18, Tier 0 below) and reconciling bebop's `step_wave` as a third representation, not a
build-vs-vendor choice. The §27 frequency/wave-domain communication idea remains parked — no
research has been done, and the source document itself requires a research pass before its own
gate applies.

---

## A. Tier 0 — zero prerequisites, read-only or self-contained on already-tested surfaces. READY NOW.

Nothing here depends on any other item; each is pure investigation or a small change under existing
test coverage.

- **Item 2** — `FileEventStore` wiring verification. Dual check: (a) is the durable store constructed
  anywhere, (b) has the `Result`-typed `insert` fix landed since 07-16 (§10/P4 — "a wired store that
  swallows IO is arguably worse than an unwired one"). Highest consequence-per-cost in the roadmap.
  **✅ RESOLVED-AS-DEFECT-FILED 2026-07-19** (re-verified adversarially against live `HEAD`): **(b) PASSES**
  (typed `StoreError` propagation confirmed, fix `4dec04218`, regression test `hydra.rs:1188-1218`);
  **(a) FAILS** — no production composition root constructs the durable store (all 6 `FileEventStore::open`
  sites are test-only; no binary builds a durable `Hydra`/`EventLog`). Defect filed:
  [`BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md`](BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md)
  (verification: [`BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md`](BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md)).
  Fix scoped there as a follow-up Tier-1 build item (§B territory) — NOT built (Tier-0 read-only audit).
- **Item 3** — `order_machine` const-adjacency + `idx_of` dedup. Golden signature and 1e-12 oracle
  already cover it.
- **Item 30** — state-machine proliferation audit (`capability_cert.rs`, `hub_provisioning.rs`,
  `hub_supervisor.rs`, `hydra.rs`). Read-only table. **✅ CLOSED 2026-07-19** —
  `AUDIT-ITEM-30-state-machine-final-2026-07-19.md`: all 4 modules INDEPENDENT (0 shared with the
  FSM proof kit), 4 PARITY-PIN tickets (I30-T1..T4, 0 collapses forced). **1 confirmed silent
  defect** (I30-D1, `resume()` owner-zeroing) fixed with a red→green guard on
  `exec/space-grade-tier0-2026-07-19` (`707848dfd`); the in-session "2 confirmed silent defects"
  phrase confirmed UNSOURCED.
- **Item 15** — eigen-surface entry-point + parity-scope verification. Read-only; defect filed only
  if found. **✅ AUDITED 2026-07-19** — single eigen-surface HOLDS (`spectral.rs:225 eigenvalues` →
  `householder::eigenvalues_contig`, no `lowrank.rs`); gap = R3 parity is values + dominant-residual
  only (`spectral.rs:1254 let _ = dvecs;`). Ticket **I15-T1** (vector-scope cross-solver pin) filed
  in [`AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md`](AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md);
  not built (new scope).
- **Item 16** — `GraphSpectrum` single-spectrum audit. Read-only unless a P2 defect forces collapse.
  **✅ RESOLVED-BY-REFACTOR 2026-07-19** — P2 CONFIRMED (`graph_spectrum` computed the adjacency
  spectrum 3×, `graph_energy_report` 4×, both claiming "single pass"). Collapse LANDED, option (b)
  (internal, zero public-signature change): `classify_drift_with_rho` + shared `drift_guards_ok`/
  `drift_band`; `graph_spectrum` now = exactly 2 passes (adj + Laplacian), `graph_energy_report`
  4→2. Proof = thread-local `EIGEN_CALLS` exactly-2 counter + field-consistency test. Kernel suite
  **902 / 0 / 3** (was 899). Committed `e125f0c97`, pushed to `exec/space-grade-tier0-2026-07-19`.
  Resolution note: `AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md` §0.
- **Item 17** — `engine` thick/thin classification table (RC-4's three mirrored items as first
  entries). **✅ AUDITED 2026-07-19** — RC-4 triple: `DriftClass` + `dt` CLOSED (pinned post-H2);
  **L-operator OPEN** (`engine/src/field_frame.rs:10-40` engine-side 5-point Neumann Laplacian
  unpinned to kernel `csr.rs:552 laplacian_spmv`). Ticket **I17-T1** (engine-boundary Laplacian pin;
  cross-references item 18's intra-kernel pin, does NOT duplicate it) filed in the tickets doc; not
  built (new scope).
- **Item 19** — retrieval spectral-routing audit (`diffusion.rs`/`ppr.rs`). Read-only.
  **✅ AUDITED 2026-07-19** — independent-by-design (zero `spectral`/`GraphSpectrum` refs;
  `ppr.rs:6-7` "No eigendecomposition"), correctly so — NOT the second GraphSpectrum consumer. New
  smell: `ppr.rs:3-5` is a comment-bound unpinned mirror of `markov.rs:162-170`'s inner loop, no
  test pin. Ticket **I19-T1** = parity-pin (NOT collapse — `retrieval/mod.rs:14` red-lines touching
  `markov.rs`) filed in the tickets doc; not built (new scope).
- **Item 22 (verification half only)** — read `mesh.rs`, classify real-port vs stub. The ruling is
  now recorded (§0 above: reimplement), so this verification informs HOW MUCH of `mesh.rs` is
  reusable scaffolding versus needs building from scratch, not whether to proceed.
  **✅ VERIFICATION COMPLETE 2026-07-19** — proof filed
  [`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`](AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md)
  (classification table, one row per public symbol, file:line + caller-or-NONE + verdict; blueprint
  independently re-verified and CONFIRMED). Finding: `mesh.rs` (387 lines, `#[cfg(feature="pq")]`,
  `pq` NOT default) is a real, tested ML-DSA-65 signed-log primitive with **ZERO production kernel
  callers** — `MeshLog`/`MlDsaSigner`/`Signer` bench-only, `SignedEntry`/`MeshError`/`HubTransport`
  uncalled, protocol layer absent; even `mesh-adapter` bypasses it (bebop path-deps, no `pq`).
  **Scoping handoff to item 23 (gated strictly after — NOT started here):** it is **"mostly stub
  above the log layer"** — reuse `SignedEntry`/`MeshLog`/`MlDsaSigner` + the `HubTransport` seam
  as-is (keep, don't rewrite), but sync/consensus/capability/gossip start near-scratch; gossip
  admission must extend `decision/import_unit()`, never fork a parallel importer (synthesis §17(b)).
- **Item 18 (narrowed)** — the Laplacian parity pin: dense `laplacian()` ↔ `csr.rs:552
  laplacian_spmv`, plus a `step_wave` reconciliation note. **[new ordering choice]** — promoted from
  mid-roadmap to Tier 0 because §26(d) shrank it to one parity test against an oracle already in-tree.
- **Item 31 (investigative half)** — `rusqlite` usage read + reclassification; pin the
  `cosmic-text = "*"` wildcard; verify `sha2`-vs-kernel-keccak on the body digest.
  **✅ INVESTIGATIVE HALF COMPLETE 2026-07-19** — findings filed
  [`AUDIT-ITEM-31-dependency-findings-2026-07-19.md`](AUDIT-ITEM-31-dependency-findings-2026-07-19.md)
  (blueprint independently re-verified). **(a) rusqlite** → KEEP-and-contain (cat-2 foreign format,
  Hermes `state.db` only, no default build path) — docs-only ruling. **(b) cosmic-text `*`** →
  DEFECT CONFIRMED + FIXED: pinned to already-resolved `0.19.0` (`engine/Cargo.toml:30`), lockfile
  unchanged, `cargo check --features text` green, committed `c2d0f306a` on
  `exec/space-grade-tier0-2026-07-19`. **(c) sha2 vs keccak** → NOT a defect, KEEP `sha2`; blueprint
  CORRECTED (`pub mod pq` is `#[cfg(feature="pq")]`-gated, not "already linked" — the swap would
  pull `aes-gcm`+`curve25519-dalek`, a net dep increase). Bonus flag confirmed: **dual in-kernel
  Keccak-f[1600]** (`event_log.rs:67` vs `pq/keccak.rs:156`) — dedup ticket owed to item 25, filed
  not fixed. Enactment half (Tier 2) allowlists rusqlite+sha2.

## B. Tier 1 — foundational builds. READY, in this internal order.

- **Items 1 + 13 combined** — the CI zero-dep gate, born deterministic:
  `cargo tree -e no-dev --locked --offline` + lockfile-hash assertion, 3-crate allowlist shrinking
  monotonically. **[new ordering choice — bundling]**: item 13 hardens item 1's own mechanism;
  building it nondeterministic first is two passes over one CI job.
- **Item 14** — `rust-toolchain.toml` pin + structural compiler-bump trigger. Independent, parallel.
- **Item 25 (procedure doc first) — ✅ DONE (2026-07-19).** The slot-arena/qrng standing procedure
  is codified and independently re-verified in
  [`PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md`](PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md)
  — all citations checked against live HEAD, both precedents (slot-arena override, QRNG
  never-replace) confirmed. **This doc is now BINDING**: Items 4+29 (`tracing`/`tracing-subscriber`
  logger/FDR rewrite, incl. the `telemetry` `SpanMetricsLayer` consumer) and Item 5 (`regex`
  retirement) MUST run its numbered 10-step ruling per crate before cutover (§18(a)'s "under this
  exact procedure, not a bespoke one"). **Re-verification finding (new scope, owed ticket, not an
  item-25 fix):** the `qrng` feature is undeclared in `kernel/Cargo.toml`, so the QRNG provider
  (incl. its sanctioned `master_seed()`) is dead code that never compiles — a
  standing-rule-vs-reality inconsistency filed in the procedure doc §3.
- **Items 4 + 29 combined, with the §1.2 `JsonWriter` absorbed in the same change** — the hand-rolled
  logger/FDR tier-(b) buffer with the energy/hardware field set first-class in the schema from day
  one. Both bundlings are the source document's own explicit mandate (§21, §10/P2). The largest
  Tier-1 item; the keystone of the tier.
- **Item 5** — retire `regex`, after the logger exists. Ruling recorded per item 25's procedure.

## C. Tier 2 — process/verification layer. Parallelizable.

- **Item 6** — §4 hardening checklist codified + CI enforcement, with §10/P7's correction built in:
  CI must re-execute oracles and dudect self-tests, never presence-check artifacts.
- **Item 7** — Kani wiring (Keccak, FSM graph algorithms, NTT arithmetic, GCRA transition — now
  applies to the adopted GCRA, §0 above).
- **Item 8** — GCRA decision package. **Ruling: ADOPT (§0 above).** Differential oracle + Kani
  interleaving check now execute toward a real swap, not just an evidence package.
- **Item 31 (enactment half)** — per-crate allowlist CI gate + shared kernel-side JSON-parse
  primitive for the seven serde carriers + manifest-recorded rulings. Depends on items 1 and 25.
- **Item 26** — batching research pass. Zero prerequisites; scheduled low-priority, measurement-only.
- **Item 27 (classifier-input half)** — PMU counters feeding `Verdict`/`DriftClass`. The
  autonomic-response half moves to Tier 4 per the source's own routing requirement.

## D. Tier 3 — THE PIVOT.

- **Item 9** — build `kernel/src/breaker/` from Blueprint A under the §1.5/§10-P4 standard (typed
  `Result<Permit, Tripped>`, unconstructible tripped-but-permitting state, `CommitError` alarms
  routed in). **The pivot point of the entire roadmap** — items 11, 12, 21, 27(response), and 32
  (control-law half) all sit behind it. Best entered after item 2's finding and Tier 1's FDR.
- **Item 10** — TLA+ spec of decision-import + order FSM. No structural dependency on the breaker;
  same-tier verification of the same state-machine family, runs in parallel with item 9.

## E. Tier 4 — gated on the breaker.

- **Item 21** — autonomic gain-scheduling module. Explicit stated dependency: strictly after item 9.
- **Item 11** — ARINC-653 scheduler Phase 0 (design doc + TLC model only). **Ruling: PURSUE,
  design-only (§0 above)** — can start now as a design artifact; the model itself doesn't need the
  breaker to exist, only the eventual code does ("code comes only after the breaker exists").
- **Item 12** — SIHFT triple-vote pilot. **Ruling: PURSUE, design-only for now (§0 above)** — the
  pilot itself needs breaker + FDR; scoping/design work can start immediately.
- **Item 27 (response half)** — after item 21.
- **Item 32 (split)** — Laplacian half already lands with item 18 (Tier 0). **Ruling: PURSUE the IR
  extension (§0 above)** — this can start as its own eqc-rs capability work, independent of the
  breaker; only the §16 pilot-control-law half needs items 9 + 21.

## F. Parallel lanes

- **Spectral/physics lane:** item 18 (Tier 0, narrowed) → item 32's Laplacian half (also Tier 0/now).
  eqc IR extension (item 32, ruled PURSUE) runs alongside, independent.
- **Living-memory lane:** item 19 (audit) → **item 20** (P95 persistence — genuinely open,
  externally ungated, READY now) → **item 28** (optical compression — **ruled PURSUE**, pilot scoped
  to the archival plane only, sequenced after item 20 since it consumes the same durability
  machinery).
- **Mesh/gossip lane:** **item 22** (verification, READY) → reimplementation work (per the §0 ruling,
  not a vendor integration) → **item 23** (explicit stated dependency: after item 22 — preserved
  exactly; extends `import_unit()`, no parallel importer) → **item 24** (crypto surfaces under §4 —
  depends on item 6's re-executing CI machinery and item 14's trigger).

---

## G. Garden of Eden — Recommended First Execution Batch, hand to Opus now, in this order

1. **Item 2** — Proof: a cited line constructing the durable store in production, or a filed defect;
   plus the §10/P4 check on `Result`-typed `insert`. **✅ DONE 2026-07-19 — defect filed** (no production
   construction site exists; (b) `Result`-typed insert confirmed): `BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md`.
2. **Item 30** — Proof: a table, one row per module, citing file:line for shared-vs-independent
   state-machine logic; every independent one gets a collapse-or-parity-pin ticket.
   **✅ DONE** — proof table + 4 parity-pin tickets in `AUDIT-ITEM-30-state-machine-final-2026-07-19.md`;
   1 confirmed defect (`resume()` owner-zeroing) fixed (`707848dfd`).
3. **Items 15, 16, 19, 17** (read-only audits, any order) — Proofs verbatim from the source: single
   backend + named parity test cited by file:line or P2 defect filed; one eigenvalue computation
   feeding all functionals; shared backend cited or defect filed; every public `engine` item
   classified with RC-4 as first three entries.
4. **Item 22 (verification half)** — Proof: the classification cited by file:line — typed boundary
   plus real kernel caller, or no-caller finding filed. Now feeds directly into reimplementation
   scoping (§0 ruling), not a decision package. **✅ DONE 2026-07-19** —
   [`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`](AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md):
   no-caller finding filed (zero production callers; `MeshLog`/`MlDsaSigner`/`Signer` bench-only,
   `SignedEntry`/`MeshError`/`HubTransport` uncalled). "Mostly stub above the log layer" — see §A.
5. **Item 3** — Proof: zero heap allocations under a counting allocator test; one `idx_of`
   definition; golden signature and 1e-12 oracle both green.
6. **Item 18 (narrowed)** — Proof: a parity test computing Lu via dense `laplacian()` and via
   `laplacian_spmv` — exhaustive over small graphs plus a large randomized corpus — green to float
   epsilon; `cargo tree` unchanged.
7. **Items 1+13** — Proof: CI fails on any new dependency, allowlist shrinks monotonically; gate
   verdict identical with networking disabled, lockfile hash unchanged.
8. **Item 14** — Proof: a toolchain-bump diff without the spot-check artifact fails CI; a non-bump
   diff never triggers the job.
9. **Item 25 (procedure doc)** — then **Items 4+29 (+JsonWriter)** — Proofs: `cargo tree` drops 13+
   crates, log output byte-compatible, post-mortem readback test (kill -9, restart, recover); event
   schema shows energy/hardware fields as first-class, RAPL-less host shows named absence not silent
   omission.
10. **Item 5** — Proof: `cargo tree` shows zero external crates; existing parsing tests green.

Everything in this batch is now unblocked — no operator ruling stands between it and execution.
