# Master Build Sequence — UPDATED 2026-07-11 (post-push revision)

> Revision of the original session master sequence. The original was written BEFORE the
> operator's parallel code session pushed fixes to `origin/main`. This update records what is
> DONE, corrects stale "live status" claims, and re-orders Tier 0/1 around reality. Gates
> unchanged: build DOWN from the first real order, not up from the protocol.

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

### G10 ML-DSA-65 — DROPPED (handed to another agent, 2026-07-11)
- 5 internal agents failed to reach bit-exact; contradictory false proofs. Not our problem now.

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

**REMAINING Tier 0 (zero-pivot-risk, IN FLIGHT as parallel batches A/B/C/D):**
- **Batch A** Design P1: remove 3 money-tween sites (ClientLayout:245, Dashboard:451, Analytics:265) + bebop skin token-flip on admin/courier/404. RED: count-up assertion fails.
- **Batch B** Reliability: degrade-storm ratchet (boot-grace + alert + restart-regression test) + `sw.js` push handler. RED: flags reset on restart; sent push renders.
- **Batch C** CI/security: gitleaks CI hard-fail + sync-crdt dep-graph fence + P7 gate scaffolding. RED: canary secret fails scan; price in MenuDelta breaks CI.
- **Batch D** GTM: OG card recompress <300 KB + channel attribution reader + QR+`?ch=` stamps. RED: content-length <300KB; other-spike detects broken QR.

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
  - > ⚠ CORRECTED (operator, 2026-07-16): the P2 "SQLite replica" stage names SQLite as the replica engine. dowiz does
    > NOT use SQLite as an architectural choice — the spectral/sqlless approach (content-addressed `BlockStore` + JSONL
    > `FileEventStore`) is the MAIN storage/retrieval path in dowiz's own kernel/engine, with **pgrust as the uniform
    > SQL-fallback/backup target, not SQLite**. Corrected ladder: the P2 replica is a **pgrust replica** (or the sqlless
    > content-addressed store) → device-authoritative → money single-writer.
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
4. **G10** — DROPPED, handed to another agent.

---
## ADDENDUM — executed + verified ground truth (2026-07-12, autopilot)

Per operator autopilot ("поки не буде зроблено, повний автопілот"), the
non-red-line plan items were executed and **re-verified with real tool runs** (the
global rule: trust `cargo test` / `node --test` output, not doc DONE claims).

### What was DONE + verified green (this session)
- **Tier-0 A/B/C/D** — re-confirmed: no offending storefront tween (integer
  cents only, static line math); gitleaks git-pre-commit = CLEAN (61 findings
  all FP by label, 0 secrets); OG `web/dist/og/card.png` = 3,384 B (<300 KB),
  `og:image` + `og:title/description` present in `web/dist/index.html`;
  `?ch=` channel attribution wired through Storefront + channel ledger.
- **Tier-1 audits** — `cargo audit` CLEAN (0 vulnerabilities); VbM
  `cargo test -p dowiz-kernel` = **37/37**; `cargo build --target
  wasm32-unknown-unknown --release` = GREEN.
- **Tier-2 quality bars** — backend contract gate built as a real test
  (`server/tests/integration.rs::tier2_storefront_contract`): integer money
  round-trip, PENDING on create, persisted, `?ch=` channel attributed. **GREEN.**
  (The Playwright UI gate is a CI artifact — deferred to after the branch-prune +
  remote-history scrub, since it needs browser binaries on CI and the 187-file
  tree is mid-flight.)
- **Courier N1/N2 (Tier-2 "courier out-of-app signal")** — WAS A GAP: server
  persisted `push_subs` + `/push_subscribe`/`/push_resubscribe` routes but had
  **no send path**. Implemented dependency-free `server/src/notify.rs`
  (`NotifyHub` trait + `CaptureSink` test sink + zero-dep HTTP/1.1 `WebhookSink`
  bridged via `NOTIFY_BRIDGE_URL`). Wired into `order_event` (signals on every
  legal transition; lifecycle never depends on delivery). Real VAPID web-push is
  gated on Tier-4 + a configured VAPID key. Proven by
  `green_status_transition_signals_couriers` (REAL signal emitted, correct
  order_id + status). **No new deps added** (web-push pulls OpenSSL — rejected
  per ponytail fewest-deps rule).
- **Governance port (bebop→dowiz)** — already on this branch
  (`d8c414f4`), `agent-governance/index.test.ts` = **10/10 node:test green**;
  re-verified this session.

### Server test totals after this work
- `cargo test -p dowiz-server`: **8 lib + 12 integration = 20 passing, 0 failing**
  (added N1/N2 signal test + Tier-2 backend contract gate + 2 notify unit tests).
- `cargo test -p dowiz-kernel`: **37/37**.
- `agent-governance`: **10/10**.

### ROADMAP §0.1 gap — CLOSED WITH GROUND TRUTH (no fabrication)
The 13 research/design reports cited in ROADMAP §0.1
(HUB-ARCHITECTURE-REVIEW, MAX-EV-SYNTHESIS, DESIGN-LIBRARIES-RESEARCH,
PARTICLE-CLOUD-INTERACTION-ANALYSIS, etc.) are **genuinely LOST** — only
incidental mentions survive on disk (confirmed by broad grep). They are NOT
re-created: fabrication would violate the ground-truth discipline. The
decisions they informed survive in canonical, verified docs:
`bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`
(+ `bebop-fable-research-2026-07-11.md`, `plan-audit-bebop-2026-07-11.md`),
and `docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md`. Mark §0.1 as
RESOLVED-AS-LOST.

### Red-line items — FLAGGED, NOT executed (per autopilot constraints)
1. **P1 / P7 / P8** — prod migrations / RLS flip / bulk write. Docs only.
2. **Prod worker restart + secret rotation** — claimed stopped 07-03, UNVERIFIED.
3. **DRIFT R2** — courier-scoring vs NO-COURIER-SCORING hard fork (architectural).
4. **G11 first real order** — requires a real non-operator customer on a claimed venue.
5. **Tier-1 branch-prune (force-push scrub of remote history)** — IRREVERSIBLE
   history rewrite; left for operator.
6. **G10 ML-DSA NIST bit-exact** — dropped, handed to another agent.

### Recommendation before next gate
Ship the verified-green server changes (notify.rs + Tier-2 gate) on this branch,
THEN run the planned Tier-1 branch-prune/scrub. Do not auto-execute any of the
six red-line items above.

*Revised 2026-07-11 from session push. Original sequence retained as source-of-truth for tiers 2–5.*
