# Resolution — TMA menu-button wiring

Conductor synthesis of breaker-findings.md + counsel-opinion.md against proposal.md.
0 CRITICAL, 1 HIGH, 3 MEDIUM, 3 LOW breaker findings; 0 ETHICAL-STOP + 4 non-blocking
counsel notes. All resolved below — hard-exit criteria met, no second attack round needed
(every resolution is either a doc/DoD correction, an already-structurally-enforced
invariant, or a small localized code hardening that adds no new surface).

## Breaker findings

| # | Sev | Resolution | Disposition |
|---|---|---|---|
| HIGH B-OPS/config — `APP_BASE_URL` may resolve to `api.dowiz.org` (API host) not the storefront | HIGH | **fix (process, not code):** promoted from "accept-risk" to a **blocking pre-flip gate**. TMA-VALIDATION.md §Pre-flip Checklist requires the operator to `curl -sI "$APP_BASE_URL/s/<real-slug>"` and confirm it returns the SSR storefront (200, `<title>…Order Online \| Dowiz</title>`) — TMA_ENABLED must not flip to `true` until this passes. Code reuses the **exact same** `APP_BASE_URL` convention already used by `spa-shell.ts:66` and `spa-proxy.ts:155` for the identical purpose (building the public storefront URL for OG tags) — this finding is a **pre-existing repo-level config risk**, not something this change introduces or diverges on; fixing `APP_BASE_URL` (if wrong) also fixes OG-tag correctness today. Owner: Backend/Ops, tracked as a pre-flip gate, not a code change in this PR. | accept-risk + mandatory operational gate |
| MEDIUM B-FAIL/CONSIST — "retry on next /start" is false (token is single-use) | MED | **fix (docs):** proposal §6/§7 language corrected — recovery requires a **new** connect token (owner re-opens "Connect Telegram" in /admin), not the same deep-link. Code comment at the call site states this explicitly. Still an accepted best-effort degradation (owner sees no error either way; matches existing `TG_STOREFRONT_ACTION`/`TG_CATEGORY_GATING` best-effort precedent) — not upgraded to a durable-retry mechanism (Option B was already rejected on proportionality grounds). | fix (doc correction) + accept-risk (no durable retry) |
| MEDIUM B-FAIL/product — Telegram WebView unvalidated beyond page-load (OAuth/crypto redirects known-fragile in-webview) | MED | **accept-risk:** both `GOOGLE_OAUTH_ENABLED` and crypto payments (ADR-0017) are **already dark/flag-off by default** in this deployment, so the specific breakage modes cited are currently moot. TMA-VALIDATION.md's manual test script scopes validation to the COD/manual-checkout happy path only, and explicitly documents OAuth-login and crypto-redirect as **not validated in-WebView** — do not promise they work; revisit if either feature is ever flipped on for TMA-opened sessions. Owner: Product (flag the caveat before any future flip of those features). | accept-risk, documented caveat |
| MEDIUM B-SCALE/FAIL — unbounded fetch pins a pooled operational connection across two sequential no-timeout calls | MED | **fix (code):** the new `setChatMenuButton` call is wrapped in a ~5s timeout (`Promise.race` against the existing helper, mirroring the `AbortSignal.timeout(5000)` pattern already used in `notifications/adapters/telegram.ts`) so it can no longer be the long pole on the held operational client. Does not touch the shared `callTelegramApi` helper (keeps blast radius to this one call site) — a repo-wide timeout on that helper is a separate, larger ticket (out of scope). | fixed (localized timeout wrap) |
| LOW B-SEC — omitting `chat_id` makes the button bot-global | LOW | **already structurally prevented:** `buildSetChatMenuButtonRequest` throws if `chatId` is falsy — cannot construct a global-button request through this builder. Unit test asserts the throw. | fixed (already in design) |
| LOW B-OPS — failure is invisible (warn log only, no metric) | LOW | **accept-risk:** matches the repo's existing best-effort-swallow convention (`console.warn` on every other best-effort branch in this file). A metric/health signal is a repo-level observability improvement, not scoped to this micro-change. | accept-risk |
| LOW doc-drift — R4 says `TMA_ENABLED` absent from EnvSchema; it is now present | LOW | **resolved:** `TMA_ENABLED` was added to `packages/config/src/index.ts` in this same PR (line ~50) before the breaker re-checked HEAD. R4 downgraded from "precondition" to "done." | resolved |

## Counsel non-blocking notes

1. **Consent line in `start.connected`.** Acknowledged as valuable and cheap. **Deferred** (not implemented this PR) to keep the diff minimal per the original scope ("minimal wiring"); flagged as a fast-follow in TMA-VALIDATION.md. No i18n-catalog.ts touch either way (consent copy would live in `bot-strings.ts`, a separate store, not the FE catalog under this lane's no-touch constraint).
2. **Label truthfulness.** Adopted directly — default menu-button text is `"My Storefront"` (honest, first-person, matches that the button opens the owner's own public page), not a generic "Menu"/"Open".
3. **Strategic framing (distribution-probe, not launch-step).** Acknowledged — stated explicitly in TMA-VALIDATION.md's header framing.
4. **Owner-side/customer-facing boundary is porous once owners share the link; `?ch=telegram-tma` would then silently attribute customer traffic.** Acknowledged as a real, unresolved Product question — noted alongside R1 in TMA-VALIDATION.md. No code implication for this PR: the query param is inert (this build adds no analytics pipeline reading `ch=`), so there is no undisclosed data collection today; the open question is for Product to resolve together with R1 before any future customer-facing expansion.

## Ethical-stops

0. Counsel's pass found none; conductor concurs — no red line crossed (no auth/schema/money/PII/courier-dignity line touched; fully reversible via flag flip; additive only).

## Hard-exit check

- 0 unresolved CRITICAL/HIGH — the HIGH is resolved via accept-risk + a mandatory, named pre-flip operational gate (a valid resolution class per the council framework).
- 0 unresolved ETHICAL-STOP.
- Aesthetic/strategic advice addressed-or-acknowledged (table above).
- Back-of-envelope holds (proposal §2, unchallenged besides the host-resolution correctness, now gated).
- Invariants intact: zero schema/migration, zero auth change, zero money-path change, zero new dependency, default OFF, best-effort/non-blocking.
- Artifacts present: `docs/adr/ADR-tma-menu-button-wiring.md`, `proposal.md`, `breaker-findings.md`, `counsel-opinion.md`, this `resolution.md`.

**Verdict: APPROVED (STOP-DESIGN-B).** No re-attack round required — every fix is either a doc/DoD correction, an already-enforced invariant, or a small localized hardening (timeout wrap) that introduces no new surface for the breaker to re-attack.
