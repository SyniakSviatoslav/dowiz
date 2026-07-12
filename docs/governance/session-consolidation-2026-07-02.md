# Session consolidation — 2026-07-02 (meta self-improving loop system)

One index for the day's work, wired to the self-improving loop: SENSE (telemetry/scouts/audits) →
DIAGNOSE (councils/analysis) → HEAL (fixes) → guardrail+ledger → REFLECT/calibrate. Everything below
is advisory-input to the plane; deterministic gates + human remain authority (pattern #4).

## Built + verified (on the working tree, uncommitted — awaiting ship batches)
| Area | Artifact | Proof |
|---|---|---|
| Telemetry (richer) | `scripts/plane-telemetry.mjs` +query/duration/severity/host + rollup digest | 22/22 tests, live-dogfooded |
| Telemetry closed loop | `telemetry/plane` branch, inbox, cloud→local round-trip | verified E2E; Telegram live |
| Meta liveness wiring | `plane-guard.mjs` `*-liveness` family (prediction/inbox/scout/health) | red→green, hard 12/12, advisory-forever ok |
| Reinforced-swarm read-back | `scripts/experience-index.mjs` (Stage-1, report-only) | 6/6 red→green; data-starvation flagged |
| Scouts (code-integrated) | `scripts/asset-surface-scan.mjs` (crt.sh), `scripts/scout-feeds.mjs` (releases) | 11/11 + 9/9, live-smoked |
| Security Tier-1 fixes | orders #1/#8, websocket #4/#5, spa-proxy #6, couriers #7 | 26 behavioral tests red→green, typecheck, regressions intact |

## Designed / analyzed (docs — feed the councils + backlogs)
- **Security council** (APPROVED-pending-operator): `docs/design/security-hardening-2026-07/` + ADR. 9 findings, converged 0 HIGH.
- **Whole-project hardening backlog**: `docs/design-review/PROJECT-HARDENING-BACKLOG-2026-07-02.md` (Tiers A–D).
- **Meta-loop gap map**: `docs/governance/meta-loop-gap-map-2026-07-02.md` (dead hops + wiring plan).
- **Reinforced-swarm plan**: `docs/design/reinforced-swarm/plan.md` (bandit router + failure retrieval, staging-first).
- **Red-team**: `docs/security/redteam-toolset-analysis-*` + `redteam-runbook.md` + `hardening-findings-*` + security-redblue loop.
- **Competitive**: `docs/research/durres-wolt-competitive-scout-2026-07-02.md` (~80 vendors; pizza-template leverage).
- **Token throughput**: `docs/research/token-throughput-2026-07-02.md`; **lane capacity**: `docs/operating-model/lane-capacity.md`.
- **Infusion**: `docs/governance/song-of-singularity.md` ledger (w27 verse + charter reflection).

## 🔴 SHIP BLOCKER (structural — needs your release-strategy decision)
`origin/main` (`a23a1592`) is **273 commits behind** `feat/phase0-safety-hardening` (`f137b1d7`) — 1383
files differ, 138 in `apps/api/src` alone. The whole integration branch (dispatch, order lifecycle,
courier, payments, money/RLS libs incl. `courier-room-authz.ts`/`courier-relay-guard.ts`) has NEVER
merged to main. Consequence: the Tier-1 security fixes depend on ~12 lib files absent from main, so they
CANNOT ship as an isolated diff against main (non-building tree). The security fixes are IMPLEMENTED +
VERIFIED on the integration branch (60 tests green, typecheck clean, plane-guard 12/12) but UNSHIPPED.
Options (your call — merging to main = prod deploy, red-line, not auto-done):
1. **Merge the integration branch to main first** (its own large red-line ship), then the security batch
   is trivial/already-included. Clean, but a big separate release.
2. **Ship the security batch off the integration branch** (base on f137b1d7) — builds + tests + E2E work,
   but the PR diff shows ~1383 files, not an isolated security diff.
3. Hand-pick the transitive closure onto main — NOT recommended (drags money/RLS/payments into a
   "Tier-1 authz" commit; closure unbounded by the file list).
Recommendation: **Option 1** (get main current — the 273-commit divergence is the actual risk hiding
everything), or Option 2 if the security fixes must ship now. This divergence is itself the highest-
priority finding: main is a stale release surface and every "ship" hits this wall.

## Operator-gated (staged, NOT applied — protect-path or decision)
1. **OR-1**: confirm deployed `DATABASE_URL_OPERATIONAL` role/bypassrls — LIVE-EXPOSURE gate (if NOBYPASSRLS today, #2 promotes to Tier-1 live PII siphon).
2. **OR-10**: Fly edge-ingress guarantee → then #9 rate-limit real-IP auth-limiter rekey (server.ts).
3. **Migrations (protect-path)**: #3 definer `SET search_path` pin; #2 C1 anon fail-open narrow (B3-coupled, anti-orphan artifact ships with Tier-1).
4. **CI (protect-path)**: wire `rls-adversarial`/phase5 tests into the fresh-provision job (they never run today — highest-value free fix); dep/vuln gate + `tmp` override; Dockerfile pin `npm install` + `USER node`.
5. **PAT** `pull_requests: write`; **Telegram** already live; **token rotation** (bot token exposed in-session).
6. **Ship batches**: (a) SAFE mechanical (dead-code + god-file decomposition, ~1,500L); (b) Tier-1 security (this session's 5 fixes + behavioral tests) → own branch → staging deploy → E2E; (c) meta-wiring (liveness + experience-index + scouts).

## The convergent root (3 lanes agree — the one refactor that retires a bug class)
Owner tenant-context is re-derived across 6 resolver copies + ~15 hand-set RLS GUCs. The Tier-1 fixes
patch the symptoms per-route; the durable fix (C1 in the backlog, council-gated) is ONE
`request.ownerContext` preHandler (verify → live-membership recheck → set `app.current_tenant`).

## Next self-improving-loop increments (from the gap map + swarm plan)
- Route the high-volume loops (demo-builder, acquisition) through `finalize` + add `arm`/`signature`/
  `fake_green_caught` fields → fills `metrics.jsonl` → experience-index rankings become trustworthy
  (measurement precedes the ratchet, pattern #12). THEN Stage-2 shadow router.
- Enact the banked guardrail proposals (gate-release-hygiene, remote-ref-integrity) — needs-council.
- Resolve the standing predictions (calibration loop hop-1 is now visible via prediction-resolution-liveness).
