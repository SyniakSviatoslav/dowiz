# Regression Ledger — compact index

> **Generated navigation aid, NOT the source of truth.** One line per row in
> `REGRESSION-LEDGER.md` (id + one-line symptom + guardrail path) so a critic can
> `grep` this ~9 KB file instead of full-Reading the 28K+-token ledger. The ratchet
> rule (never shrink/delete ledger content) applies to the ledger, not this index —
> this file is fully regenerable and carries no independent authority. Full row detail
> (root cause / guardrail proof / RED→GREEN evidence) lives ONLY in the ledger row
> itself — `grep -n "^| <id> |" docs/regressions/REGRESSION-LEDGER.md` (or `grep -A0`
> then read that one line) once you know which id is relevant.
>
> Regenerate after any ledger append (id · truncated-symptom · first path-like
> backtick-span in root-cause/guardrail-type/where/date, preferring one containing `/`):
> ```
> python3 -c "
> import re
> rows=[l.rstrip() for l in open('docs/regressions/REGRESSION-LEDGER.md',encoding='utf-8')
>       if l.startswith('| ') and not re.match(r'^\|[-\s|]+\|\s*\$',l)][1:]
> def gp(s):
>     sp=re.findall(r'`([^`]+)`',s)
>     return next((x for x in sp if '/' in x and not x.startswith('http')), sp[0] if sp else '(see row)')
> def tr(s,n=88):
>     s=s.strip(); return s if len(s)<=n else s[:n].rsplit(' ',1)[0]+'…'
> for r in rows:
>     f=r.split('|'); rid=f[1].strip(); sym=tr(f[2]); g=gp('|'.join(f[3:]))
>     print('| ' + rid + ' | ' + sym + ' | `' + g + '` |')
> "
> ```

85 rows indexed (ledger is append-only — this count only grows).

| # | Symptom (one line) | Guardrail path |
|---|---------------------|-----------------|
| 56b | The JWT verifier's alg/kid pinning (RS256-only + known-kid required, ADR-0003) could be… | `@deliveryos/platform` |
| 55 | The bearer-only (cookie-less) posture — the property that makes state-changing… | `scripts/guardrail-no-set-cookie.mjs` |
| 54 | `/auth/refresh` could silently widen the minted access token's TTL beyond the ADR-0004… | `apps/api/tests/auth-refresh-race.test.ts` |
| 53 | The argon2id password-hash parameters (memoryCost 65536 / timeCost 3 / parallelism 4,… | `apps/api/tests/argon2-params-lock.test.ts` |
| 38 | "Integrate STORM + Scrapling and scrape the food-delivery market" risked (a) a 3rd… | `ethics/conduct-gate` |
| 37 | Three external AI tools requested for integration (ByteDance DeerFlow, PurpleAILAB… | `scripts/guardrail-license.mjs` |
| 36 | The AI menu-parser re-spent LLM tokens on identical inputs — a re-import of the same… | `apps/api/src/lib/ai-ocr-parser.ts` |
| 35 | A tenant who picks a **pale brand primary** ships an illegible storefront: the CTA fill… | `packages/ui/src/theme/palette.ts` |
| 34 | A legitimate **frequent customer** dead-ends at checkout: `POST /api/orders` returns a… | `e2e/tests/flow-simpl-s1-velocity-frictionless.spec.ts` |
| 33 | A NEW migration adds a `SECURITY DEFINER` Postgres function WITHOUT a pinned… | `scripts/guardrail-definer-search-path.mjs` |
| 32 | A `MultiEdit` to a `protect-paths` zone (`.claude/**`,… | `.claude/settings.json` |
| 31 | The out-of-band Skyvern pilot (AGPL, out-of-tree sidecar) could be handed a dowiz… | `scripts/skyvern-pilot/no-credential-attest.mjs` |
| 30 | Integrating external tooling could silently import (a) a copyleft (AGPL/GPL)… | `scripts/compliance-gate.ts` |
| 29 | The AI menu-parser ingests UNTRUSTED scraped/OCR content concatenated into a Claude… | `apps/api/` |
| 28 | flow-simplification §5 honest-dispatch (no-trap red-line F1) had no deterministic guard… | `apps/api/src/lib/dispatch.ts::attemptHonestDispatch` |
| 26 | deliver v2 (Cash-as-Proof) red-lines: (a) GDPR GPS-crumb retention silently anonymizes… | `lib/deliveryCompletion.ts::completeDelivery` |
| 25 | P6 claim phase (shadow→owner) could (a) let a logged-in attacker claim a shadow they… | `routes/owner/menu-confirm.ts` |
| 24 | P6-3 menu extraction→render could (a) leak third-party free-text NAMES… | `apps/api/src/lib/menu-region.ts` |
| 23 | P6-2 shadow-spine write could (a) be admitted WITHOUT a valid one-time token… | `apps/api/src/modules/acquisition/provisioning.ts` |
| 22 | P6-1 acquisition pipeline could (a) create duplicate shadow lifecycles for one… | `apps/api/src/modules/acquisition/state-machine.ts` |
| 21 | Tests that pass while the feature is broken — full-surface sweep found 2023 blind-spots… | `tools/eslint-plugin-local/src/index.js` |
| 20 | §4 order-velocity throttle was phone-only — `clientIpHash` computed but never gated, so… | `apps/api/src/routes/orders.ts` |
| 19 | §1.2/§1.3/§1.4 sensor capture could silently regress: (a) a delivered order's… | `e2e/tests/flow-sensor-delivery-baseline.spec.ts` |
| 18 | §1.1 sensor runtime could silently regress three ways: (a) a "frozen" promise that… | `apps/api/tests/eta-synthesis.test.ts` |
| 17 | Checkout shows a delivery total that differs from what the server charges (worst at the… | `/info` |
| 1 | Live prod auth backdoor: `POST /api/auth/local/login` minted a real owner JWT;… | `scripts/migrate-runner.ts` |
| 2 | Predictable / forgeable security tokens, OTPs, nonces, session ids (same trust-boundary… | `local/no-insecure-random` |
| 3 | Prod outage — API crash-loops at boot, both machines `stopped`, 000 timeouts | `release_command` |
| 4 | Prod boot hangs before `listen` — `/livez` critical, never binds `:8080` | `start()` |
| 5 | Out-of-order WebSocket frames flip order status backwards; reconnect-storm bugs | `local/no-direct-websocket` |
| 7 | Cart silently carries stale prices → checkout hard-block ambush (F9); owner card… | `e2e/tests/polish-debt-logic.spec.ts` |
| 6 | Storefront cards unreadable — dark text on dark surface (~1.08:1 contrast) on light… | `local/no-hardcoded-color` |
| 7b | Money rendered/stored with float drift | `eslint` |
| 8 | Hardcoded user-visible strings ship untranslated; Albanian diacritics lost | `local/no-hardcoded-string` |
| 11 | "Fixed A, broke B" — a shared-component change silently regresses another screen;… | `e2e/visual/` |
| 10 | Visually-wrong-but-valid UI ships (off-scale spacing/off-token colour via arbitrary… | `local/no-arbitrary-tailwind` |
| 9 | A key routed through `t()` is added to one locale but **silently missing** in `sq`/`uk`… | `packages/ui/src/lib/i18n-catalog.ts` |
| 9b | Raw SQL string interpolation (SQLi surface) | `local/no-raw-sql` |
| 10b | Cross-courier / cross-tenant IDOR (assignment-accept) | `local/require-auth-hook` |
| 11b | Permissive test assertions hide contract regressions | `local/no-permissive-status-assertion` |
| 12 | Unhandled `localStorage` access crashes the SPA (private mode / storage off) | `localStorage` |
| 14 | Render-time defects invisible to text gates (dark-on-dark contrast #6, silently-dropped… | `e2e/tests/behavioural-invariants.spec.ts` |
| 13 | A whole CSS rule silently dropped by the browser — present in file/served/dist CSS,… | `*/` |
| 16 | Removed/downgraded owner keeps tenant read/WRITE access for the full token life — owner… | `scripts/guardrail-owner-active-membership.mjs` |
| 15 | Storefront "blinks empty" under load — `/public/locations/:slug/menu` returns HTTP 500… | `e2e/tests/menu-load.spec.ts` |
| 49 | **Plane telemetry was invisible and its egress unguarded**: maintainer runs left no… | `scripts/plane-telemetry.mjs` |
| 50 · **PENDING** | **C1 anonymous RLS policies fail-open — DESIGNED-NOT-SHIPPED (already orphaned once, in… | `GET /orders/:id` |
| 39 | LIVE cross-tenant + cross-courier WebSocket leak (B6): the courier `subscribe` branch… | `apps/api/src/lib/courier-room-authz.ts` |
| 40 | ADR-0013 DoD: an ADMITTED courier later UN-bound (owner-reassign / decline / sweep)… | `apps/api/src/lib/courier-relay-guard.ts` |
| 41 | **B4 BOLA on `/api/admin/*`** (ADR-admin-platform-authz, APPROVED): every admin… | `apps/api/src/lib/platform-admin.ts` |
| 42 | **WS JWT leaked into logs via `?token=`** (P1 of the WS-token-in-URL escalation,… | `apps/api/src/lib/logger.ts` |
| 43 | **Storefront allergen surface was recipe-only on two LIVE surfaces**… | `apps/web/src/pages/client/MenuPage.tsx` |
| 44 | **Menu Characteristics layer — compare/filter could fabricate a verdict or mis-rank… | `apps/web/src/pages/client/MenuComparePanel.tsx` |
| 45 | **Voice engine's "holds zero write capability" was import-grep, not enforced against a… | `tools/eslint-plugin-local/src/index.js` |
| 46 | **The test suite was a paper gate: `test:unit` + `verify:all` ran NOWHERE automatically… | `@deliveryos/db` |
| 47 | **The governance gates were silently DISARMED for 11 days while looking registered:**… | `.claude/hooks/{serious-gate,red-line-doubt-gate,guard-bash}.sh` |
| 51 | **A prod deploy health-failed 3× in a row (single-machine outage):**… | `apps/api/tests/worker-boot-budget-lock.test.ts` |
| 52 | **The 2026-07-03 275-commit merge-to-prod deploy failed serially across 6 patterns, all… | `.github/workflows/ci.yml` |
| 56 | **LC1 — inclusive-tax double-charge, and the mirror-oracle test that CERTIFIED it**… | `apps/api/tests/vectors/order-total-vectors.ts` |
| 57 | **Cross-tenant IDOR class — the tenant predicate was per-author discipline, forgotten… | `docs/design/audit-fix-authz/resolution.md §2` |
| 58 | **LC2 owner PATCH `/orders/:id/status` cross-tenant IDOR + LC3 customer cancel 500**… | `UPDATE orders SET status` |
| 59 | **LC6 — terminal refund obligation was structural-hole, not code** (audit-money C3,… | `docs/design/audit-fix-money/migration-drafts/` |
| 60 | **LC9 — fetch-failure fallbacks rendered/saved FABRICATED data as real**… | `unit` |
| 61 | **LC4 + N1 — GDPR erasure could strand forever OR silently false-complete**… | `workers/anonymizer-gdpr.ts` |
| 62 | **G1b — nothing stopped `packages/voice` (the voice ENGINE, R2-F/M3: must hold zero… | `local/no-voice-app-import` |
| 63 | **G3/G4 — the capability-table's exhaustiveness and money/dietary/settling exclusions… | `packages/voice/src/__tests__/capability-table.test.ts` |
| 64 | **LC7 — the backup restore-DRILL was a false-green nest that verified nothing it… | `backup-drill-integrity.test.ts` |
| 65 | **Closed-venue orders were accepted server-side** (Albania GTM gap): the storefront… | `unit` |
| 66 | **Two live money-DISPLAY defects + ignored `default_locale`** (Albania GTM gap). (a)… | `/100` |
| 67 | **A decorrelated verification sweep found audit fixes shipped with proofs that could… | `CheckoutPage:680/709` |
| 48 | **The self-improvement loop's advisory arm was structurally unenforced and unmeasured**… | `scripts/guardrail-ledger-integrity.mjs` |
| 68 | **The design nutrition/BOM product-card + Cyrillic-font-fallback fix shipped (commit… | `apps/web\|packages/ui` |
| 69 | **~1,800 lines of council-reviewed voice FE were one `git worktree remove --force` from… | `scripts/guardrail-sandbox-staleness.test.mjs` |
| 70 | **L5 self-modification had no enforced boundary** — a system that reads its own… | `scripts/meta-controller.test.mjs` |
| 71 | **TMA theme attributes were minted from an untrusted bridge object without validation**… | `apps/web/src/lib/__tests__/tma.test.ts` |
| 72 | **cart money field was a bare `number` in the Rust-rebuild Astro cart store** —… | `rebuild/web` |
| 73 | **Rust owner-catalog writes could seat the WRONG GUC family** — the S3 build brief said… | `rebuild/crates/api/src/db.rs` |
| 74 | **GDPR erasure left the customer's doorway photo alive** — `anonymizeOrder` nulled the… | `apps/api/tests/anonymizer-order-photo-purge.test.ts` |
| 75 | **Telegram webhook secret-token 2nd-layer failed OPEN** — `routes/telegram-webhook.ts`… | `e2e/tests/telegram-webhook.spec.ts` |
| 76 | **LIVE Art.17 under-erasure: a GDPR customer-erasure never reached the subject's own… | `apps/api/tests/gdpr-erasure-completeness.test.ts` |
| 77 | **The Rust-cutover SQL class: passes as a psql literal / text-render, FAILS as a sqlx… | `docs/design/ci-rust-live-pg/` |
| 78 | **HTTP status-code parity drift in the port: a Node business-400 became a Rust 422.**… | `sendError(400,…)` |
| 79 | **A dead OrderStatus enum value can sit in the contract forever with NO writer and NO… | `apps/api/scripts/verify-state-machine-coverage.ts` |
| 80 | **Every storefront page rendered UNSTYLED on staging** — the cutover front-door… | `/_astro/` |
| 81 | **Marathon sessions burn quadratically and nothing stops them** — 24h audit 2026-07-05:… | `docs/operating-model/proposed-hooks/context-budget-guard.sh` |
