# audit-gate — run memory

## 2026-06-27 · health-check pass (backend-only session)

**Scope:** post backend-only changes (orders.ts/server.ts decomposition + loop-harness tooling, no FE changes) → storefront + admin health-check on staging (https://dowiz-staging.fly.dev).

**Artifacts (live browser, chromium):**
- `audit/audit-gate/storefront-1280.png` — desktop
- `audit/audit-gate/storefront-390.png` — mobile

### Verdict — STOREFRONT: PASS ✓ (with artifacts)
- **A tokens:** PASS — `grep` hex in packages/ui = 59 hits, ALL acceptable: `PaperIllustration.tsx` uses token-first `var(--token, #fallback)` (fallbacks, not drift); `allergenColors.ts` is a deliberate semantic palette w/ documented WCAG ratios (6.0–7.0:1). No naked colors bypassing tokens.
- **B unification:** PASS — consistent card/button/chip/filter styling across both breakpoints.
- **C integrations (live, network):** PASS — every API/SSR route 200, incl. the ones moved into `bootstrap/routes.ts` this session: `/v1/rates`, `/public/locations/demo/menu`, `/public/locations/demo/info`, `/api/public/theme/demo`. Live cross-validation of the route-registration extraction.
- **D polish/responsive:** PASS — clean at 390 + 1280; brand tokens (warm cream/red) applied; 2-col→1-col grid correct.
- **F rare states:** PASS — missing images degrade gracefully to the branded placeholder glyph (no broken-image UI).

### FLAG-ONLY (not fixed — not FE code, not a regression)
- 2 console/network **404s** for staging demo images (`/images/locations/<demoLoc>/logo.webp` + 1 product `.webp`). Cause: demo location's images are not in staging object storage (staging-data/seeding gap), pre-existing, unrelated to this session's backend refactors. FE handles it correctly (placeholder). → seeding/storage concern, not a gate blocker.

### NOT COVERED (env barrier)
- **Admin surfaces** — staging `/api/dev/mock-auth` returns 404 (dev-auth route gated off on staging; same limitation hit earlier this session). Cannot reach `/admin/*` live without an owner session. Admin audit deferred until a staging owner login path exists, or run against a local dev build with `?dev=true`.

**Net:** storefront live-verified healthy post-decomposition; the only finding is a known staging-asset gap (flag-only). No cosmetic inline-fixes needed (no FE changes this session).
