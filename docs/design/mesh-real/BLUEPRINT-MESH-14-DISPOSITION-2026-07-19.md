# MESH-14 — Resolve-contradictions + RED-suite + status-from-live-test CI-lint

> GAP-A1 disposition (2026-07-19): the ONE genuine gap in the whole roadmap was 12
> un-homed arc sub-units. MESH-14 is the only one that needs a real small blueprint.
> Per `GAP-A1-DISPOSITION-AUDIT-2026-07-19.md`, this is **docs + CI-lint only**
> ("НЕ ЧІПАЄМО — продакшн" — does NOT touch production code). Its CI-lint sub-part
> (part 5) folds into Q1's claim-verification checkpoint rather than being duplicated.
>
> Source of truth for the work items: `mesh-real/BLUEPRINTS-MESH-REAL.md:184-194`
> (§Шар-крос — RED + суперечності, G-RED wave). This file is the short blueprint the
> audit said was owed; it does not invent new scope.

## Scope boundary (load-bearing)

- **IN:** reconcile notes (D3-DTN, migration diagram, ADR ratify), workspace/README
  phantom-dir fix, and the CI-lint rule. All are documentation / `bebop2` workspace
  metadata / CI config — none alter `dowiz-kernel` or `engine` runtime behavior.
- **OUT (operator-gated, NOT done here):** any production code path in either repo.
  The `bebop2-root-workspace-Cargo.toml` + `kernel/cli/reloop` phantom-dir fix lives
  in `bebop-repo` (separate gate); this blueprint records the *intent + checklist*,
  the edit is landed by the bebop lane owner.

## Part 1 — D3-DTN / BPv7 vs built-WSS reconcile-note

Reconcile the stale "D3-DTN / BPv7" protocol prose with the actually-built
`iroh`-over-WSS transport. Captured conclusion (already the built reality):
**iroh = QUIC-connection-layer UNDER BPv7**; the G6 "build-locked-stack" note agrees.
Deliverable: a one-paragraph reconcile note appended to `mesh-real/RESEARCH-CONSPECT.md`
stating the canonical transport stack so future docs don't re-litigate it.

## Part 2 — MIGRATION-PLAN diagram adds dowiz-kernel reuse

The migration-plan diagram must show `dowiz-kernel` as a reuse target (not a rewrite)
for the mesh adapter. Deliverable: add one diagram node + edge in the existing
migration-plan section noting `feat/p34-mesh-adapter` already consumes bebop2 protocol
types through `dowiz-kernel` (merged 2026-07-19).

## Part 3 — ratify ADR-0007 + ADR-0008-update (SQLite → pgrust)

- ADR-0007: ratify as-is (the decision held).
- ADR-0008: update the storage backing from SQLite to `pgrust` (the landed
  `pgrust` feature; see `internal-retrieval-living-memory-arc` + kernel `pgrust`
  gate). Deliverable: two ADR files under `mesh-real/` (or `docs/design/adr/`)
  reflecting the ratified state.

## Part 4 — workspace Cargo.toml + README phantom-dir fix (bebop-repo, gated)

`bebop2-root-workspace-Cargo.toml` still lists `kernel/`, `cli/`, `reloop/` as members
that no longer exist at those paths → phantom members + broken README links.
Deliverable (lands in `bebop-repo`, separate gate): remove the three phantom
workspace members and fix the README cross-links. Checklisted here; not executed in
dowiz under autopilot.

## Part 5 — CI-lint: status only from a live test (FOLDS INTO Q1)

**Rule:** any doc/ledger "CLOSED" claim MUST cite a matching live-path test name
(red-team lesson: stale docs were trusted because prose, not a test, asserted done).
A `CLOSED`-claim without a resolvable `verified-by` pointer → CI RED.

This is the **same mechanism** Q1's claim-verification checkpoint already installs
(`DONE-VERIFIED` + `verified-by`, `P56 `StaleGround`` catch). Therefore MESH-14 does
NOT re-specify a CI-lint — it is recorded as **one consumed instance of Q1**. See
`BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md` Q1 row.

## RED-suite (part of G-RED wave)

Every line reachable red→green, regression-ledger row, zero `expect(true)`/skip.
`CLOSED`-claim-without-live-test → CI-red. Wave: G-RED. 🔴

## Disposition status

- Parts 1–3: doable as docs in this repo (autopilot-authorized, docs-only).
- Part 4: bebop-repo, operator-gated lane — checklist only.
- Part 5: folded into Q1; no separate implement.
- GAP-A1 thereby CLOSED for the can-be-closed-today items; IP-06 (future blueprint),
  IP-17/18 (operator crypto gate), IP-21 (downstream scaffolding) correctly remain
  open per their own disposition rows.
