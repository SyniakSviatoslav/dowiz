# Master Build Sequence — UPDATED 2026-07-11 (post-push revision)

> Revision of `docs/design/MASTER-BUILD-SEQUENCE-2026-07-11.md`. The original was written
> BEFORE the operator's parallel code session pushed fixes to `origin/main`. This update
> records what is DONE, corrects stale "live status" claims, and re-orders Tier 0/1 around
> reality. Gates unchanged: build DOWN from the first real order, not up from the protocol.

## What shipped THIS session (operator + agents, pushed to prod)

### dowiz `origin/main` @ f0bd9966 (PROD LIVE)
- **P6** `/claim` 404 fix — `server.ts:858` SPA_ROUTES + `'/claim'` added. RESOLVES now (was 404).
- **P2** checkout enum/receiver — `packages/shared-types/src/legacy.ts` 3-kinds + `receiver{}`.
- **P3** GDPR storage-DI — `AnonymizerService` now receives `storage` via `BackgroundWorkerDeps`
  (workers.ts:100 + server.ts:346). Photo-purge no longer no-ops.
- **DRIFT** `docs/design/DRIFT-ANALYSIS-2026-07-11.md` — 12 drifts R1–R12.
- **Red-line flags** `docs/ops/P1/P7/P8-*.md` — operator-decision docs, NOT executed.

### bebop `origin/main` @ 5645f4d (PROD LIVE)
- vault flaky fix `d87193d` (6/0 parallel) — pre-existing env race, fixed.
- memory non-destructive tick `cf917ba` (281/0).
- 8 math/security builders merged: L1/L2/B2a/B2b/B2c/B3a/B3b/B3c (all reviewer-APPROVE, lib-green).
- wasm32 G9 `388f90b` merged.

### G10 ML-DSA-65 — IN FLIGHT (not yet green, not pushed)
- Root cause PROVEN by localizer (deleg_98e17e17): `expand_s` seed is 1-byte `rhoprime‖i`;
  FIPS 204 needs 2-byte `rhoprime‖r‖i` (r=0 s1, r=1 s2). Fixer (deleg_fc4285a3) running.
- Crypto red-line: after green, SEPARATE reviewer proves bit-exact (doer≠reviewer). Then merge+push.

## CORRECTIONS to original MASTER-SEQUENCE (stale claims)

| Original claim | Reality (2026-07-11 post-push) | Action |
|---|---|---|
| "/claim still 404s on prod AND staging" (§live, T0) | RESOLVED — pushed f0bd9966 | Strike from Tier 0; mark DONE |
| "Deploy the committed /claim+G03 fixes" (T0) | DONE by operator push | Move to DONE ledger |
| "GDPR trio + DI-fix" (T1) | P3 DI-fix ALREADY in prod; P1/P7/P8 red-line NOT executed (docs only) | T1 now = execute P1/P7/P8 per operator flag, or close as decided |
| "zero real orders / zero validation" (honest-assessment) | Self-corrected by operator: v1 tested on first client NOW | G11 GREEN gate stays; framing updated |
| "prod worker stopped 07-03" | UNVERIFIED — no prod access (red-line) | Needs operator green-light to restart |
| "bebop/bebop2 NOT stable" (local-first SYNTHESIS) | 275/275 + 91/91 GREEN; kernel::decide bypass (pg.rs:863) is bebop (PARKED), not dowiz | Keep as bebop P1-under-everything; not dowiz blocker |

## TIER 0 — NOW (re-ordered: done first, then remaining zero-pivot-risk)

**DONE this session (strike from backlog):**
- Deploy /claim + P2 + P3 → prod ✓
- Protocol library lanes W/A/H: NOT started (bebop parked — see Tier X). Defer.

**REMAINING Tier 0 (zero-pivot-risk, do now):**
- Remove 3 money-tween sites (ClientLayout:245, Dashboard:451, Analytics:265) — Design P1. RED: count-up assertion fails. ~1 session.
- P1 token-flip (bebop skin on admin/courier/404; spectral tokens) — Design P1. RED: contrast AA; ramp-on-text.
- Degrade-storm ratchet (boot-grace + real alert + restart-regression test) — Reliability RG-1. RED: flags reset on restart.
- `sw.js` push handler (served worker push-deaf) — Notifications N0. RED: sent push renders.
- gitleaks install + CI hard-fail; land 3 gate diffs + P7 amendment; close stale GH #9 — G02/G13.
- OG card recompress <300 KB — GTM. (652 KB today → WhatsApp bare link.)
- **[RED-LINE: operator]** Restart prod worker; rotate `PROVISION_OPS_SECRET`; Supabase-cred rotation — G11/G12. Needs explicit go.
- Channel: prod attribution READER ("Orders by channel") + QR+`?ch=` stamps — Channel P1.
- Sync/CRDT fence gates (CI: `sync-crdt` banned from domain/settle/dispatch) — Sync Y0.

## TIER 1 — prod vehicle (curated main-lineage merge)

- **P1/P7/P8 red-line EXECUTION** (currently docs-only): operator must decide. P1 = pause secret re-push loop; P8 = flip BYPASSRLS (103 policies sleep); P7 = gate Rust checkout decide.
  → If operator approves: execute + tests. If not: close as decided, keep flags.
- Re-verify /claim + P2 + P3 green on prod (already pushed — this is a confirm gate).
- Prod OG cards + prod demo provisioning (blocked on rotated `PROVISION_OPS_SECRET`).
- Then: remote-history scrub + branch prune (G02 Wave 3) — scheduled, mirror-bundle first.

## TIER 2 — quality-first bars (falsifiable checklists)

- Design "stable enough to send" = 13-item checklist + storefront zero-diff Playwright gate (Design §5.7). ~7–10 sessions.
- GTM "genuinely working to send" = 8-point per-venue gate (data-not-shell, claim e2e, <300 KB unfurl, channel render, freshness, ≤60s order-loop alert, honest-inert shadow, human checklist). Build 5 missing demos (needs rotated secret).
- Courier out-of-app signal (Notifications N1/N2) — real order reaches a courier.
- → First remote sends (warm WhatsApp → referral → owner's Facebook page; no auto-send).

## TIER 3 — VALIDATION (hinge)

`G11 GREEN` = one real order from non-operator customer on claimed venue. Pre-committed RED:
0 claims after defined contact set → stop/pivot (~€1,800 info value). Walk-in EV+ even at 5% claim.

## TIER 4 — local-first substrate (gated on G11 GREEN + reliability)

- Protocol production: R node (`dowiz-node` single-writer sequencer = kernel::decide-bypass fix),
  X settlement/dispute (COD ledger, counter-signed handoffs, NO courier scoring — CI-enforced).
- Migration ladder P2→P4 (SQLite replica → device-authoritative → money single-writer).
- Channel registry + sync-CRDT menu lane (price always vendor-signed).
- Reliability gate LD0–LD11 GO before any venue cutover.

## TIER 5 — earn-it (each behind named gate)

- Money-bearing bebop2 crypto → gated on crypto audit ladder (Wycheproof→FIPS→ACVP→constant-time→audit). Until then: hybrid-only.
- Messenger transport → gated on G7 survey + red-line council.
- Astro/Svelte port → gated on arbiter doc + FE-0.1 budget.
- Anonymous .onion tier → gated on vendor node (Tier 4).
- Multi-venue mesh (iroh-gossip, P5 decommission Supabase/Node/Fly) → gated on soak.

## TIER X — PARKED (dated, re-entry criteria)

- **bebop as protocol**: capture-protect only (commit/push/1 demo/memory ~1h). Not funded until product carries it. npm 404 today; keyless-demo ~80% ready → make-it-trialable session.
- **field-sim**: parked. Salvage heat-kernel/FFT/VSA(pow2) as libs; fix sign bug (`u += dt·c·L·u` → anti-diffusion, ‖u‖→4.7e31) if revived. Green tests MASK it = VbM violation, low priority.
- **sovereign-core cutover (G04)**: mothball unless full gate sheet banked; keep kernel-honesty slice (= Protocol R).
- **B3 RLS flip**: last, red-line.

## Spine (unchanged)

**Tier 0 stabilizes v1 → Tier 1 ships prod truth → Tier 2 hits quality bars → Tier 3 = first real
order → only then Tier 4 rewrites substrate.** Build downward from the order, not upward from the
protocol. Gates are falsifiable conditions, not calendar dates.

## Open items needing OPERATOR decision (red-line, not auto-executed)

1. **P1/P7/P8 execution** — docs written, flagged. Approve to execute?
2. **Prod worker restart + secret rotation** — claimed stopped 07-03, UNVERIFIED by me.
3. **DRIFT R2** — reputation.rs courier-scoring vs NO-COURIER-SCORING HARD FORK. Architectural.
4. **G10 force-push** — after fixer green + separate crypto reviewer cert.

*Revised 2026-07-11 from session push. Original sequence retained as source-of-truth for tiers 2–5.*
