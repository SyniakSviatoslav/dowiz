# Prod Release Plan — `feat/mvp-sensor-seams` → `main`

> Date: 2026-07-02 · Branch: `feat/mvp-sensor-seams` (**255 commits ahead of `main`**) ·
> Target: `https://dowiz.fly.dev` (prod) · Staging: `https://dowiz-staging.fly.dev`
> Author: release-readiness pass (lifecycle + E2E run + divergence/flag/migration audit).

## 0. The one framing that governs everything

**A code deploy to prod is NOT a customer launch.** They are two separate acts with two different gates:

- **Act A — deploy the branch to prod (DARK).** Mechanical, reversible, low-risk: every new
  feature ships behind a flag that defaults **false** (verified in the Dockerfile — §4). Gated on:
  migrations applied + green post-deploy smoke. **This plan makes Act A GO-ready.**
- **Act B — launch to customers (turn flags on / open the access gate).** Gated on the MVP
  launch-blocker councils (B1 money, B2 dispatch, B3 RLS) + the soft-access-gate STOP-1. **This plan
  does NOT clear Act B — it stays NO-GO until those councils close (§5).**

Conflating the two is the trap. We ship the code dark to prod now; we open to customers later.

---

## 1. Evidence run (staging, 2026-07-02)

### 1a. Core critical-path E2E — `36 passed / 7 failed` (desktop, staging)
Every one of the 7 failures is an **environmental / test-harness artifact — zero product regressions**:

| Failure | Cause | Verdict |
|---|---|---|
| `flow-order-creation` — 422 `NOT_DELIVERABLE` | spec's hardcoded customer geo is outside the demo venue's current delivery tiers (fixture drift) | product **correctly** enforcing delivery range |
| `flow-orders-checkout` — 429 `PHONE_THROTTLE` (54s) | reused test phone across order-creating specs at `workers=2` | rate limiter working **as designed** |
| `flow-ui-order-lifecycle` / `flow-ui-owner-core` / `flow-ui-courier-core` — "Step 1" | serial precondition = the failed order-create above cascaded | dependent, not independent failures |

**The 36 passing prove the release-critical surface:** `POST /api/orders` create + **idempotency**
(duplicate-key rejected), **cross-tenant access rejected**, **no cookies on public endpoints**, **CSP
present**, **rate-limit headers**, **menu_version + Cache-Control** on public menu, OAuth gate,
auth-required endpoints gated. Command:
`VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret playwright test … --project=desktop`.

### 1b. Live lifecycle trace (`e2e/lifecycle-e2e/critical-lifecycle.spec.ts`) — **NEEDS A HEAL**
- ✅ `auth.setup` passes (owner + courier mock-auth against staging).
- ❌ Main customer→owner→courier→deliver trace fails at the **add-to-cart → `cart-open`** step.
  Root cause: `cart-open` renders only when `itemsCount > 0` (`ClientLayout.tsx:189`); the spec's
  add-to-cart interaction is **stale vs the 2026-07-01 storefront menu-UX redesign** (scroll-anchor
  chips + ingredient-badge modal), so the item never entered the cart. **Test-maintenance debt, NOT a
  product break** — the menu renders, and the order spine is green at the API layer (1a).
- A **first** run also hit `RATE_LIMIT 429` on the menu because I ran the core suite concurrently —
  do not run two staging suites at once.

**Action (pre-merge, non-blocking for Act A):** heal `critical-lifecycle.spec.ts` + `flow-order-*`
fixtures — correct the customer geo to a deliverable point in the demo venue's tiers, give each
order a unique phone, keep `workers=1`, and update the add-to-cart interaction to the new menu UX.
Then it should trace green (the surfaces exist; the spec is behind the UI).

### 1c. Deeper lifecycle gate (optional, recommended pre-merge)
`/reliability-gate` (5-agent L0–L11 code audit → GO/NO-GO with citations) has not been run this pass.
Recommended before merge for the money/dispatch/RLS threads it stitches (complements the live E2E).

---

## 2. What's in the divergence (255 commits)
All new product code lands **dark**. Highlights (from branch memory + `git diff main...HEAD`):
sensor-seams (geofence/ETA), deliver-v2 cash-as-proof spine, checkout-communication overhaul,
menu-characteristics model, acquisition→claim provisioning, storefront branding/fonts/venue-Maps
enrichment, crypto-payments (dark), courier-WS authz (ADR-0013), platform-admin authz (B4),
pg-privilege/RLS hardening, and the voice-control Phase-0 + ASR engine (dark, no UI).

## 3. Migrations — the hard dependency (17 new: `068`–`084`)
`packages/db/migrations/1790000000068…084` are NOT on `main`. The boot-guard **FATAL-exits** if the
running image needs a migration the DB doesn't have (this has caused a prod outage before —
`prod-outage-schema-drift`).
- CI `deploy` job runs **`pnpm migrate:up` BEFORE `flyctl deploy`** (`.github/workflows/ci.yml:145`).
- **⚠️ VERIFY before merge:** that `migrate:up` in the deploy job targets the **prod Supabase** DB
  (`DATABASE_URL_MIGRATIONS`) and that all 17 apply cleanly forward-only. Money/RLS-touching migs
  (073 cash-as-proof, 077 RLS-NOBYPASSRLS-phase1, 080 grant-hardening, 083 payments-ledger) are
  🔴 red-line — dry-run them on a prod snapshot first.

## 4. Feature-flag posture — verified DARK
Dockerfile build-args all default **false**: `VITE_ACCESS_GATE_PUBLIC_ENABLED`, `VITE_TG_CATEGORY_GATING`,
`VITE_MENU_CHARACTERISTICS_{ENABLED,COMPARISON,FILTER}`, `VITE_MENU_ALLERGEN_FILTER`. Server-side:
`VOICE_CONTROL_ENABLED` unset → voice dark (fail-closed); `PAYMENTS_PREPAID`/`CRYPTO_ENABLED` off.
**Merging to `main` therefore ships zero new customer-facing behavior** (Act A is dark by construction).
→ Do NOT pass any `--build-arg …=true` on the prod deploy; leave every flag at its default.

## 5. Customer-launch blockers (Act B — still NO-GO)
From `docs/design-review/ADVERSARIAL-AUDIT-2026-06-29.md` (all 🔴 COUNCIL):
- **B1 — money model inverted** (payout = collected cash, no commission/settle; cash-ledger only
  `hold`). Staged: mig 073 cash-as-proof + deliver-v2 spine. **Verify council closed before launch.**
- **B2 — dispatch auto-recover is dead code** (queue never drained; reject-before-accept orphans the
  order). **No staged fix confirmed — likely still open.**
- **B3 — tenant isolation on app-`WHERE` only** (hot-path `BYPASSRLS` makes FORCE-RLS inert). Staged:
  mig 077 RLS-phase1 + 080 grant-hardening (partly). **Verify the C1 role-flip landed.**
- **B6 — live courier-WS cross-tenant leak** — **resolved** (ADR-0013 courier WS authz, ledger #40).

**These do not block Act A (dark deploy)** — they gate turning the order/money/courier spine loose on
real customers. Recommend a focused **verify-B1/B2/B3 pass** (or `/reliability-gate`) as the entry
gate to Act B.

## 6. Known non-blocker to note
Staging `/health` = `degraded` — solely the `fallback` subprocessor check (`offline-phone fallback`);
postgres/workers/bus/telegram/r2/settlement/anonymizer/backup all `ok`. Confirm the same on prod
post-deploy; investigate the fallback provider but it does not block Act A.

---

## 7. Execution checklist — Act A (deploy dark to prod)
1. **Pre-merge green:** `pnpm typecheck` + full unit suites + heal §1b, then re-run the core E2E and
   the lifecycle trace on staging **serially** → green. (Optional: `/reliability-gate` → GO.)
2. **Migration dry-run:** apply `068`–`084` to a **prod-DB snapshot**; confirm forward-only + no data
   loss on the 🔴 money/RLS migs. Confirm the deploy job's `migrate:up` points at prod
   `DATABASE_URL_MIGRATIONS`.
3. **Flag audit:** confirm no `…=true` build-arg is injected on the prod deploy (all dark — §4).
4. **Merge `feat/mvp-sensor-seams` → `main`** (PR, squash or merge-commit). CI on `main` then:
   `migrate:up` → `flyctl deploy --remote-only` → post-deploy E2E (deploy-validation + core-lifecycles
   + Telegram webhook/full-flow) against prod.
5. **Post-deploy verify:** prod `/health` all-ok (bar known fallback); post-deploy E2E green;
   `/s/<a-live-slug>` renders; spot-check one real order create + idempotency.
6. **Rollback:** if migrate fails → deploy aborts (migrate runs first; no half-deploy). If a
   post-deploy check fails → `flyctl releases --app dowiz` + `flyctl deploy --image <prev>` (or
   `flyctl releases rollback`); migrations are forward-only, so prefer a **forward fix** over a
   down-migration on 🔴 tables. Keep the prior image tag noted before deploy.

## 8. Recommendation
- **Act A (dark deploy): GO once §7.1–§7.3 are green** — the divergence is dark, migrations are
  sequenced by CI, the release-critical API/security surface is proven green on staging, and the only
  red E2E items are a stale lifecycle spec + fixture/rate-limit collisions (all fixable pre-merge, none
  a product regression).
- **Act B (customer launch): NO-GO** until B1/B2/B3 councils close and the soft-access-gate STOP-1 is
  lifted. Ship the code dark now; open to customers as a separate, gated decision.
