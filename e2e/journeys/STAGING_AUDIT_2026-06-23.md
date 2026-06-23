# Staging full audit — 2026-06-23 (lifecycle + 5 adversarial agents)

Target: `https://dowiz-staging.fly.dev` · branch `feat/product-media-seam` · HEAD `0ce7187b`.
Driven by: lifecycle trace (owner-API) + 5 parallel agents — system-breaker (hater),
test-scout (QA), UX/critique (browser), security-sentinel, invariant-guardian.

## ✅ Lifecycle reliability gate — GREEN
Owner-API trace on staging (order-create rate-limited 5/min/IP, so spaced):
- Happy delivery: create→PENDING → CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED — all 200, total 1250 consistent.
- REJECT terminal ✓ · CANCEL terminal ✓.
- Invalid jump PENDING→DELIVERED → **400 IllegalTransitionError** (state machine guards hold).
- Soft access gate fired correctly (velocity → soft_confirm; cleared via acknowledged_codes).
- Courier claim/accept/pickup/deliver NOT exercised (no courier on shift on staging) — GAP, see below.

## ✅ Fixed + deployed + proven THIS pass
| Sev | Finding (source) | Fix | Commit |
|-----|------------------|-----|--------|
| CRIT | Menu pool starvation → storefront blinks empty under load (orig blocker) | F1 cache + F2 drop redundant query + F3 pool 8→20 + F4 set-based availability | d120a914, 57f32e11 |
| CRIT | In-process menu cache unbounded + keyed on attacker-controlled ?locale → instance OOM (hater + security) | FIFO cap (MENU_CACHE_MAX_ENTRIES=500) + locale normalization | 0f3781f9 |
| HIGH | stale-on-error served a cached copy of ANY age during outage (hater) | floor at MENU_STALE_ON_ERROR_MAX_MS=1h | 0f3781f9 |
| CRIT | Live Telegram bot token + webhook secret hardcoded in 6 e2e files (security) | env-only sourcing, all literals removed (⚠️ token MUST be rotated — in git history) | 72561420 |
| HIGH | Intermittent HTTP 500 on first owner login under load (UX) | guard db.connect() → graceful 503 (matches order-create) | 32c9c123 |
| MED | Customer page could revert DELIVERED→IN_DELIVERY on reordered WS frame (hater) | terminal-state lock in order.status handler | 0ce7187b |

Proof: menu-load.spec GREEN 3/3 on staging (+ distinct-locale fan-out arm); 20/20 burst → 200;
F4 equivalence SQL green (18 cases); preflight 17/17 + ui tests green; typecheck green; lifecycle green.

## ✅ Clean (verified, no action)
- invariant-guardian: PASS, no flags (RLS/money-integer/migration-reversible/auth all sound).
- security: SQLi none (parameterized), RLS scoping correct, no cross-tenant leak, media flag-gated, no PII in logs/cache.
- hater confirmed solid: server status transitions race-safe (status-guarded UPDATE + 409), OTP no double-order, courier /delivered row-guarded, 404s never cached, SEAM_SCORECARD F1-F11 genuinely closed.

## ⚠️ Remaining backlog — triaged (needs decision / next pass)

### Needs a product/ops decision (not code I should land unilaterally)
- **CRIT-1 (UX) demo test data is customer-facing**: storefront default tab is `Test-Cat-1782148864348` / "Pita Test Sushi Updated". It's the shared demo fixture — delete/relabel the test category+product on the demo location, or seed a clean demo. (Data, not code.)
- **HIGH-2 (UX) logo 404**: `/images/locations/28239442-.../logo.webp` 404s on every storefront+branding load — clear the dangling logo URL or upload the asset. (Data.)
- **Telegram token rotation**: the leaked bot token (now removed from code) is still in git history — rotate it.

### FE polish (real, customer-facing; batch in a follow-up)
- HIGH-1 hero title contrast (white on light-pink gradient — WCAG-AA fail).
- HIGH-3 checkout "Shënime" field keeps red error ring when validly filled.
- MED i18n gaps (admin: "Enable sound", "Mark kitchen busy", "Choose File"…), mobile filter/category overflow (no scroll cue), broken-image glyph vs branded placeholder, bare 404 page, map null-coord console warnings, KPI alarming-zero colors.

### Server / hardening (defense-in-depth, low urgency)
- HIGH (security best-practice) read_public_menu SECURITY DEFINER lacks `SET search_path = public` — add in a future migration (currently safe: unqualified names + parameterized).
- MED (hater) courier `cashCollected` input is discarded (sends task.total); server enforces ==total so over/under cash is never recorded — cash-audit field is a no-op.
- MED-4 (UX) dashboard WS showed "disconnected" on staging — confirm live-board push works (may be staging-only).
- LOW: /info isOpen uses server tz not venue tz; OPERATIONAL_POOL_SIZE no upper bound; no composite index on menu_schedules (perf only when schedules exist); menuInflight dedup per-key (mitigated by locale normalization + LRU).

### Test coverage gaps
- No automated courier-on-shift lifecycle E2E (claim→accept→pickup→deliver).
- No unit test for the cache eviction/locale-normalization internals (covered indirectly by the E2E fan-out arm).
