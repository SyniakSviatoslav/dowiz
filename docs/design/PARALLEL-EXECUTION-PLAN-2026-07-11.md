# Parallel Execution Plan — Updated Master Build Sequence (2026-07-11)

> Derived from `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`. Decomposes remaining work into
> PARALLEL-SAFE batches (independent files, zero-pivot-risk, non-red-line) vs SEQUENTIAL gates
> (red-line operator decisions, external validation, tier dependencies). Each parallel batch runs
> in its own git worktree+branch (collision-isolated). VbM: RED test first, then GREEN; doer≠reviewer
> cert after.

## PARALLEL-SAFE BATCHES (Tier 0 remaining, zero-pivot-risk, NON-red-line) — IN FLIGHT

### BATCH A — Design P1 (frontend tokens + money honesty)
Branch: `feat/design-p1-tokens`. Files: ClientLayout.tsx:245, Dashboard.tsx:451, Analytics.tsx:265,
styles (bebop skin tokens, `--status-*`, error-handle), admin/courier/404 surfaces.
- Remove 3 money-tween sites (count-up). RED: count-up assertion fails.
- P1 token-flip: bebop skin on admin/courier/404; spectral oklch ramps (hue ≤60°); error-handle visible. RED: contrast AA; ramp-on-text.
~2-3 sessions.

### BATCH B — Reliability + Notifications
Branch: `feat/reliability-push`. Files: server.ts (boot-grace), public/sw.js (push handler), reliability harness.
- Degrade-storm ratchet: boot-grace + real alert + restart-regression test (flags reset → RED).
- sw.js push handler (push-deaf today). RED: sent push renders; silent-drop detected.
~2-3 sessions.

### BATCH C — CI / Security gates
Branch: `feat/ci-security-gates`. Files: .github/workflows, package.json (gitleaks), ci/*.ts (sync-crdt fence), docs/ops/P7-*.
- gitleaks CI hard-fail (canary secret fails → RED).
- Sync/CRDT fence: CI forbids sync-crdt from domain/settle/dispatch. RED: price in MenuDelta → CI RED.
- P7 gate scaffolding (NOT decide bypass). Close stale GH #9.
~1-2 sessions.

### BATCH D — GTM / Channel attribution
Branch: `feat/gtm-channel`. Files: OG route, channel analytics route, QR gen, ?ch= middleware.
- OG card recompress <300 KB (652 KB today). RED: content-length <300KB.
- Channel attribution READER + QR+`?ch=` stamps. RED: other-spike detects broken QR.
- Do NOT provision demos (blocked on lost PROVISION_OPS_SECRET).
~2 sessions.

All four independent (no shared files), running concurrently, isolated branches. Reviewer cert (doer≠reviewer) after all land.

## SEQUENTIAL GATES (cannot parallelize; need decision or prior tier)

### TIER 1 — prod vehicle (waits on red-line operator decision)
- **P1 / P7 / P8 red-line EXECUTION** — docs written, NOT executed. BLOCKED on operator approval.
- Re-verify /claim+P2+P3 green on prod (confirm gate).
- Prod OG/demo provisioning — BLOCKED on rotated PROVISION_OPS_SECRET (lost; needs operator).

### TIER 2 — quality-first bars (waits on Tier 0 done)
- Design "stable enough to send" = 13-item checklist + storefront zero-diff Playwright gate (Design §5.7).
- GTM "genuinely working to send" = 8-point per-venue gate + build 5 missing demos (needs rotated secret).
- Courier out-of-app signal (Notifications N1/N2).
→ First remote sends AFTER both bars green.

### TIER 3 — VALIDATION (external, not code)
- `G11 GREEN` = one real order from non-operator customer on claimed venue. Gates everything below.

### TIER 4 / 5 — substrate rewrite (waits on G11 GREEN)
- Protocol production R/X, migration ladder, channel registry, sync-CRDT menu lane, reliability LD0–LD11.
- Money crypto audit ladder, messenger transport, Astro port, .onion tier, multi-venue mesh.

## RED-LINE HELD (operator decision, NOT dispatched)
1. P1/P7/P8 execution (docs only).
2. Prod worker restart + secret rotation (claim "stopped 07-03" UNVERIFIED — no prod access).
3. DRIFT R2 — reputation.rs courier-scoring vs NO-COURIER-SCORING HARD FORK (architectural).
4. G10 ML-DSA-65 — DROPPED, handed to another agent.

## STEP-BY-STEP SPINE
1. Batches A/B/C/D in parallel (non-red-line, zero-pivot-risk). ✓ IN FLIGHT
2. Reviewer cert each (doer≠reviewer) → merge to main → deploy.
3. Operator decides P1/P7/P8 + prod restart → execute Tier 1.
4. Tier 0+1 green → Tier 2 quality bars → first remote sends.
5. G11 GREEN (real order) → unlock Tier 4/5.

*Build downward from the order, not upward from the protocol. Gates are falsifiable, not calendar.*
