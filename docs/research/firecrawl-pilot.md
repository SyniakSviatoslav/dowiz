# Firecrawl pilot — out-of-tree scrape→markdown candidate

**STATUS: SCAFFOLDED — DO NOT USE. PENDING OPERATOR + COMPLIANCE DECISION.** Out-of-band. Not wired,
not in CI, not a dependency. Registered as a *candidate*, dark.

## What it is
[firecrawl/firecrawl](https://github.com/firecrawl/firecrawl) — crawl/scrape a site and return
LLM-ready markdown/structured data (handles JS rendering, pagination). AGPL-3.0 self-host + a hosted
cloud SaaS. **Verify license before any adoption.**

## Why it's a candidate
The cleaner alternative to CloakBrowser for the `demo-builder` scrape layer: hands back structured
markdown, letting us delete brittle Playwright selector code (the `setDefaultTimeout` stalls noted in
memory). No stealth/bot-evasion posture.

## 🔶 Compliance & boundary (blocks adoption until resolved)
- **Cloud SaaS = a new subprocessor.** Target URLs (and any fetched content) leave the box. This trips the
  `/compliance` subprocessor gate (see `compliance/` + CI privacy-gate). If adopted, it must be the
  **self-hosted AGPL** form on the dev/ops plane — never the cloud plan (which may train on input).
- `firecrawl` / `@mendable/firecrawl-js` are **FORBIDDEN-DEP** — out-of-tree only, never a product dep.
- Scraping conduct gate applies: `node scripts/scrape-pilot/scraping-conduct-attest.mjs <config.json>`
  (public sources, robots.txt, ≥500ms, no PII).
- No dowiz secret in the sidecar env: `node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>`.

## Decision note
Firecrawl (respectful public fetch) and CloakBrowser (stealth evasion) address the same need from opposite
ethical ends. **Pick one lane, dark, and only after the operator resolves the scraping-policy question.**
