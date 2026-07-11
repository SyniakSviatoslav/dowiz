# Parallel Execution Plan — Updated Master Build Sequence (2026-07-11)

> Derived from `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`. Decomposes remaining work into
> PARALLEL-SAFE batches (independent files, zero-pivot-risk, non-red-line) vs SEQUENTIAL gates
> (red-line operator decisions, external validation, tier dependencies). Each parallel batch runs
> in its own git worktree+branch (collision-isolated, per lessons from bebop builder batches).
> VbM discipline: RED test first, then GREEN; doer≠reviewer cert after.

## PARALLEL-SAFE BATCHES (Tier 0 remaining, zero-pivot-risk, NON-red-line)

### BATCH A — Design P1 (frontend tokens + money honesty)
Files: `apps/web/src/.../ClientLayout.tsx:245`, `Dashboard.tsx:451`, `Analytics.tsx:265`,
`apps/web/src/styles/*` (bebop skin tokens, `--status-*` palette, error-handle), admin/courier/404 surfaces.
- Remove 3 money-tween sites (count-up animation). RED: a count-up assertion fails (money never animates).
- P1 token-flip: bebop skin on admin/courier/404; spectral oklch ramps (hue ≤60°); flip error-handle to visible.
  RED: contrast-audit AA; ramp-on-text = RED.
Branch: `feat/design-p1-tokens`. ~2-3 sessions.

### BATCH B — Reliability + Notifications (boot honesty + push)
Files: `apps/api/src/server.ts` (boot-grace), `apps/web/public/sw.js` (push handler),
reliability test harness.
- Degrade-storm ratchet: boot-grace window + real alert + restart-regression test (flags reset on restart → RED).
- `sw.js` push handler (served worker is push-deaf). RED: a sent push renders; silent-drop detected.
Branch: `feat/reliability-push`. ~2-3 sessions.

### BATCH C — CI / Security gates
Files: `.github/workflows/*`, `package.json` (gitleaks), `ci/*.ts` (sync-crdt fence), `docs/ops/P7-*`.
- gitleaks install + CI hard-fail on canary secret. RED: canary secret fails scan.
- Sync/CRDT fence gates: CI forbids `sync-crdt` from domain/settle/dispatch dep-graph. RED: a price field in MenuDelta → compile/CI RED.
- Land 3 gate diffs + P7 amendment; close stale GH #9.
Branch: `feat/ci-security-gates`. ~1-2 sessions.

### BATCH D — GTM / Channel attribution
Files: `apps/api/src/routes/*` (OG card), channel attribution reader, QR+`?ch=` stamp middleware.
- OG card recompress <300 KB (652 KB today → WhatsApp bare link). RED: content-length probe + WhatsApp paste-test.
- Channel prod attribution READER ("Orders by channel" card) + QR+`?ch=` stamps. RED: `other`-spike detects broken QR.
Branch: `feat/gtm-channel`. ~2 sessions.

All four batches are independent (no shared files), can run concurrently. Each agent: isolated worktree,
RED-then-GREEN, no push. Reviewer cert (doer≠reviewer) after all four land.

## SEQUENTIAL GATES (cannot parallelize; need decision or prior tier)

### TIER 1 — prod vehicle (waits on red-line operator decision)
- **P1 / P7 / P8 red-line EXECUTION** — docs written (`docs/ops/P1/P7/P8-*.md`), NOT executed.
  P1 = pause secret re-push loop; P8 = flip BYPASSRLS (103 policies sleep); P7 = gate Rust checkout decide.
  → BLOCKED on operator approval. Do NOT auto-execute.
- Re-verify /claim+P2+P3 green on prod (already pushed — confirm gate).
- Prod OG/demo provisioning — BLOCKED on rotated `PROVISION_OPS_SECRET` (lost; needs operator).

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
2. Prod worker restart + secret rotation (claim "stopped 07-03" UNVERIFIED by me — no prod access).
3. DRIFT R2 — reputation.rs courier-scoring vs NO-COURIER-SCORING HARD FORK (architectural).
4. G10 ML-DSA-65 — parked sinkhole; decisive C-reference probe pending operator go.

## STEP-BY-STEP SPINE
1. Launch BATCH A/B/C/D in parallel (non-red-line, zero-pivot-risk).
2. Reviewer cert each (doer≠reviewer) → merge to main → deploy.
3. Operator decides P1/P7/P8 + prod restart → execute Tier 1.
4. Tier 0+1 green → Tier 2 quality bars → first remote sends.
5. G11 GREEN (real order) → unlock Tier 4/5.
6. G10 decisive probe (if approved) → bit-exact → merge.

*Build downward from the order, not upward from the protocol. Gates are falsifiable, not calendar.*
