# CloakBrowser pilot — out-of-tree scraping-unblock candidate

**STATUS: SCAFFOLDED — DO NOT USE. PENDING OPERATOR + ETHICS DECISION.** Out-of-band. Not wired into
the app, not in CI, not a dependency. Added to the registry as a *candidate*, dark.

## What it is
[CloakHQ/CloakBrowser](https://github.com/CloakHQ/CloakBrowser) — a stealth Chromium that presents as a
drop-in Playwright/Puppeteer replacement with source-level fingerprint patches, built to pass bot-detection.
Wrapper (Python + JS) is MIT; the Chromium binary uses a delayed-free-release model (prior major free on
GitHub Releases, latest major behind a Pro subscription). **Verify license terms before any adoption.**

## Why it's a candidate (the real pain it targets)
The `demo-builder` loop scrapes Google-Maps / Wolt venue data via headless Playwright, and hits
sign-in-gated galleries + bot detection (see memory: `storefront-venue-data-maps-scrape`,
`demo-builder-loop`). CloakBrowser is the tool that would unblock those gated fetches.

## 🔴 Ethics & authorization (non-negotiable — Ethics Charter; blocks adoption)
- Stealth bot-detection evasion against **third-party** properties (Google, Wolt) is ToS-violating and
  legally gray. The charter forbids turning tools against the people/systems they were learned from.
  **This is a human/operator call — not an agent decision.** Do NOT run it until that call is made and
  recorded.
- If ever authorized: public, non-auth-walled sources only; robots.txt honored; rate-limited (≥500ms);
  NO PII harvest. Reuse the scraping-conduct gate: `node scripts/scrape-pilot/scraping-conduct-attest.mjs`.
- No dowiz secret in the sidecar env: `node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>`.

## Boundary
- `cloakbrowser` is **FORBIDDEN-DEP** — out-of-tree only, never a product dependency/import.
- Alternative to weigh first: **firecrawl** (see `firecrawl-pilot.md`) — a subprocessor-shaped fetch layer
  that avoids stealth-evasion entirely. Pick ONE scraping lane; do not run both.
