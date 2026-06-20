# Onboarding Wizard — QA Report

**Target:** https://dowiz-staging.fly.dev (staging)
**Date:** 2026-06-19
**Surface:** `/admin/onboarding` — the 9-step owner onboarding wizard
**Method:** agent-browser exploratory QA (desktop + mobile viewports) against a **fresh owner** with no location membership, plus a "hater" critical UX pass over all 9 steps. Evidence is screenshots in `qa-shots/` (no video — `ffmpeg` unavailable in sandbox).
**Test prerequisite:** commit `e09ef49` (`test(onboarding): fresh-owner mode in staging mock-auth`) — `POST /api/dev/mock-auth {fresh:true}` mints a brand-new owner with no location so the flow lands on the wizard instead of the dashboard. Dev-guarded by `x-dev-auth-secret`.

**Verification honesty:** Findings citing `file:line` were checked against the working tree. The session-expired bounce (O1) is reported from captured screenshots + commit context and is marked ⚠️ — it was **not** re-run to root cause in this pass. All other findings are reproducible from source.

---

## Summary

| # | Severity | Area | Issue |
|---|----------|------|-------|
| O1 | ⚠️ High | Auth / session | Fresh-owner session deterministically bounces to `/login` with "Your session has expired" (root cause unconfirmed) |
| O2 | Low | i18n / copy | Stepper step 4 renders `"Courier:"` — trailing colon leaked from a reused order-card i18n key |
| O3 | Low | UX / copy | Menu step subhead says "Import your menu from a **PDF**" but the only import card is "Import from **CSV** — Coming soon" |
| O4 | Low | UX | `"Order Flow Test"` exposed as a user-facing wizard step to brand-new owners |
| O5 | Low | UX / mobile | 9-step stepper crammed into 390px — labels wrap/truncate, hard to read on mobile |
| O6 | Info | UX | Full admin bottom tab-bar renders during the wizard on mobile (lets user navigate away mid-onboarding) |
| O7 | Info | Brand | Brand name inconsistent: login card says "DeliveryOS", wizard header says "Dowiz" |

**Positive:** The wizard itself is solid. All 9 steps render and are navigable on both desktop and mobile (`onb-qa-02`, `onb-hater-*`). Step content (Restaurant, Menu, Location & Zone map, Courier setup, Branding, Preview) is well-laid-out, the live storefront Preview (step 6) is accurate, and the SQ/EN/UA switcher works throughout. No console/JS errors observed on the rendered steps.

---

## O1 — Fresh-owner session bounces to login (⚠️ High, root cause unconfirmed)

**What:** A freshly-minted owner (no location membership) lands on the onboarding wizard, but a session-expired bounce to `/login` was captured deterministically — the login card shows the localized banner "Your session has expired. Please sign in again." / "Sesioni juaj ka skaduar. Ju lutem hyni sërish."

**Evidence:** `qa-shots/onb-qa-01-bounced-to-login.png` (EN), `qa-shots/onb-qa-03-deterministic-bounce-session-expired.png` (SQ). The wizard *does* load (`qa-shots/onb-qa-02-wizard-step1.png`, all `onb-hater-*` steps), so the bounce is conditional — likely the fresh mock-auth token is rejected on a subsequent API call / navigation rather than on initial render.

**Impact:** If reproducible for real fresh signups (not just the mock-auth test path), a new owner is ejected from onboarding before they can publish — a hard blocker for first-run activation. If it is **only** an artifact of the `fresh:true` mock-auth token (short TTL / not persisted the way the SPA expects), it is test-infra noise, not a product bug.

**Next step (not done this pass):** Re-run the fresh-owner flow with the network panel capturing the first request that returns 401, and confirm whether the bounce reproduces with a real (non-mock) fresh owner. Until then, treat severity as ⚠️ provisional.

---

## O2 — Stepper label "Courier:" has a trailing colon (Low)

**What:** Step 4 of the stepper reads `"Courier:"` (with colon) instead of "Couriers" / "Postierë". Visible in `qa-shots/onb-hater-mobile-step3-location.png`, `…-step4-courier.png`, and the desktop shots.

**Root cause:** `OnboardingPage.tsx:50` builds the stepper label with `t('admin.courier', 'Courier')`. That key's value is `'admin.courier': 'Courier:'` (`packages/ui/src/lib/i18n.ts:1411`) — it is **shared** with `OrderCard.tsx:147`, where the colon is intentional ("Courier: John"). Reusing it as a standalone step label drags the colon in.

**Fix:** Give the stepper its own key (e.g. `admin.courier_step` → "Couriers" / "Postierë" / "Кур'єри") rather than reusing the order-card label. (Step 4's own heading at `OnboardingPage.tsx:389` already correctly uses `admin.courier_setup` = "Courier Setup".)

---

## O3 — Menu step: "PDF" subhead vs "CSV" card (Low)

**What:** On step 2 (`qa-shots/onb-hater-mobile-step2-menu.png`) the subhead reads "Import your menu from a PDF" while the only import option is the card "Import from CSV — Coming soon". PDF ≠ CSV.

**Root cause:** `admin.import_menu_desc` = `'Import your menu from a PDF'` (`i18n.ts:1552`) and `admin.import_csv` = `'Import from CSV'` (`i18n.ts:1553`), both consumed at `OnboardingPage.tsx:324` and `:329`.

**Fix:** Pick one format and make subhead + card agree (the import surface is "Coming soon" anyway, so align the copy to whatever import will actually ship). The working "Add manually" and "Demo menu" cards are fine.

---

## O4 — "Order Flow Test" exposed as a wizard step (Low)

**What:** The stepper's 9th step is labelled "Order Flow Test" (`i18n.ts:1395` `admin.flow_test`, used as a step label at `OnboardingPage.tsx:55`; step body at `:615`). It runs a real order through the full lifecycle to verify the setup.

**Assessment:** This is an intentional feature, not a leaked internal screen — but "Order Flow Test" reads as developer/QA language to a first-time restaurant owner. Consider a customer-facing label ("Test your first order" / "Provo një porosi"). Low priority; flagged for copy review, not a bug.

---

## O5 — 9-step stepper is cramped on mobile (Low)

**What:** At 390px the stepper packs all 9 dots+labels across the top; labels like "Location & Zone" and "Order Flow Test" wrap to 2–3 lines in a tiny font (`qa-shots/onb-hater-mobile-step2-menu.png` … `-step6-preview.png`). Legible but noisy.

**Fix idea:** On mobile, collapse the stepper to a compact "Step N of 9 — <current label>" progress bar (the footer already shows "Step 2 of 9") and drop the full dot rail, or make it horizontally scrollable with only the active label expanded.

---

## O6 — Admin bottom tab-bar renders during onboarding (Info)

**What:** On mobile, the full admin bottom navigation (rocket / grid / clipboard / fork / More) is pinned during the wizard (`onb-hater-mobile-step2..6`). A user can tap away to Dashboard/Orders/Menu mid-onboarding.

**Assessment:** May be intentional admin chrome. But during a focused first-run wizard it invites abandonment. Consider hiding global nav until the wizard is completed/skipped. Info-level — confirm intent before changing.

---

## O7 — Brand name inconsistency (Info)

**What:** The login card header says **"DeliveryOS"** (`onb-qa-01`/`onb-qa-03`) while the in-app wizard header says **"Dowiz"** (`onb-hater-*`). Two product names on adjacent screens of the same flow.

**Fix:** Settle on one brand string and use it in both the auth shell and the admin shell header.

---

## Environment limitations

- Sandbox cannot deploy/migrate staging directly via the app; staging is warmed via the QA harness `wakeStaging()` (Fly auto-stops → cold-start 503s are expected, not bugs). See memory `deploy-topology`.
- No video (`ffmpeg` not installed) — evidence is screenshots only.
- O1 was captured but not root-caused in this pass (see its "Next step").

## Suggested fix order

1. **O1** — confirm reproducibility, then fix or downgrade to test-infra noise (blocks first-run if real).
2. **O2 + O3 + O4** — small i18n/copy edits in `OnboardingPage.tsx` + `i18n.ts`, each provable with a Playwright `toContainText` assertion on the staging wizard.
3. **O5 / O6 / O7** — UX polish, batch after the copy fixes.

---

## Resolution (2026-06-19, branch `feat/v1-hardening`)

**O1 — root-caused and fixed (was the High).** Not test-infra noise — a real first-run blocker. `getLocationId()` (`apps/api/src/routes/spa-proxy.ts:64`) returns `null` for both "no/expired token" *and* "valid owner with no location yet". `GET /api/owner/settings` then returned **401** for a fresh owner, and `apiClient` (`apps/web/src/lib/apiClient.ts:84`) treats any 401 under `/admin` as session-expiry → wipes the token → bounces to `/login`. Fix: a new `isValidOwnerToken()` helper lets `/api/owner/settings` return `200 {id:null}` for a valid owner with no location, so `AdminHome` (`AdminRoutes.tsx:230`) routes them to `/admin/onboarding`. Unauthenticated callers still get 401. Proof: `e2e/tests/onboarding-copy-qa.spec.ts` (O1 case, deploy-gated).

**O2 — fixed.** New dedicated key `admin.courier_step` ("Couriers" / "Postierë" / "Кур'єри") at `i18n.ts`, used at `OnboardingPage.tsx:50`. `admin.courier` (with colon) stays the OrderCard label. Proof: `packages/ui/src/lib/__tests__/i18n.test.ts` "O2: admin.courier_step … has no trailing colon" — **passing**.

**O3 — fixed.** `admin.import_menu_desc` is now format-neutral ("Import your existing menu or add items manually." + SQ/UA) — no more "PDF" vs "CSV" mismatch. Proof: i18n test "O3: … no longer mentions PDF" — **passing**.

**O4 — fixed.** New `admin.flow_test_step` ("Test order" / "Provo porosinë" / "Тестове замовлення") for the onboarding stepper label + heading (`OnboardingPage.tsx:55,615`); the dev-only `admin.flow_test` ("Order Flow Test") is untouched. Proof: i18n test "O4: admin.flow_test_step resolves" — **passing**.

**O5 / O6 / O7 — not addressed this pass** (UX polish; deferred).

### Reliability hardening (server / sockets — out of original QA scope, addressed same branch)
Driven by a "service falls down sometimes / realtime not reliable" report. See diagnostic in the change summary; fixes:
- **Process guards** (`server.ts`): `unhandledRejection`/`uncaughtException` now logged + sent to Sentry instead of crashing the single web process (which dropped every WebSocket).
- **Liveness probe** (`health.ts` `GET /livez`): cheap endpoint so Fly's health check no longer rides the heavy 11-query `/health` (+5s Telegram) that could exceed the 3s timeout and restart the machine. ⚠️ **Manual step:** point `fly.toml`'s `http_service.checks.path` at `/livez` (file is in a protected zone — not auto-edited).
- **Realtime reconnect** (`message-bus.ts`, `useWebSocket.ts`): removed the permanent give-up after 5 reconnect attempts on **both** the server LISTEN/NOTIFY bus and the browser socket; the client also resumes on `online`/`focus`/`visibilitychange`. Prevents silent realtime death after a deploy/blip.
- **NOTIFY safety** (`message-bus.ts`): publish on the pool (not the dedicated LISTEN client) + 8000-byte payload guard that truncates to `{type, data.id, _truncated}` instead of silently dropping the event. Proof: `packages/platform/tests/message-bus-notify.test.ts` — **passing**.
- **WS authorization** (`websocket.ts`): owner `subscribe` now verifies location membership (closes a cross-tenant live-feed leak); courier `courier:*` scoped to their own room.
</content>
