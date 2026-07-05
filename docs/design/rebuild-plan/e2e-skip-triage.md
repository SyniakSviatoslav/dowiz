# E2E Skip Triage — FROZEN SKIP-LIST (Phase-0, Lane E pre-A blocker item 4)

- **Date:** 2026-07-04 · **Lane:** R6 (rebuild program) · **Type:** read-only census + triage, docs-only
- **Mission:** name every hole in the parity oracle. The 174-spec Playwright net is the rebuild's
  language-independent contract (`REBUILD-MAP.md` §3 Phase 0 item 4; `14-crosscutting-proofnet.md` §7.3
  gap 3, §8d final-acceptance item 1). Every skip below is a **named** hole; none is hidden.
- **Freeze rule (feeds §8d):** the frozen baseline is **167 skip call sites** (160 `test.skip` +
  7 `test.fixme`) as of branch `fix/audit-remediation`, working tree 2026-07-04. At final acceptance
  the full-suite run's skip count must be **≤ this baseline**; any NEW skip is a red flag, not a
  workaround. RETIREs and UNSKIPs may only *lower* the number.

## Methodology

Every mechanism was machine-enumerated, then every call site was read in source context
(4 parallel read-only audit lanes covering all 51 files; each row below is line-verified).

| Extraction | Command | Count |
|---|---|---|
| Spec files | `find e2e -name '*.spec.ts' \| wc -l` | **175** (census said 174 @ `ae9f5360`; +1 = new untracked `e2e/tests/channel-attribution.spec.ts`, 4 tests, 0 skips) |
| `test.skip(` — line-anchored (the census's command) | `grep -rhE "^\s*(test\.skip\()" e2e --include='*.spec.ts' \| wc -l` | **158** |
| `test.skip(` — unanchored (true count) | `grep -rhE "test\.skip\(" e2e --include='*.spec.ts' \| wc -l` | **160** |
| `test.fixme(` | `grep -rhE "test\.fixme\(" e2e --include='*.spec.ts' \| wc -l` | **7** |
| `test.only(` / `describe.skip(` / `.skip` modifier | same grep family | **0 / 0 / 0** |
| Config exclusions (`testIgnore` / `grep:`) in all 3 playwright configs | `grep -n "testIgnore\|grep:" playwright.config.ts playwright.visual.config.ts e2e/lifecycle-e2e/playwright.config.ts` | **0** |
| CI `--grep` exclusions | grep of `.github/workflows/*.yml` | **0** (CI *selects* 4 specs by path — a selection gap, not a skip mechanism) |

**Census-vs-158 reconciliation:** the inventory's "158 skips" is the **line-anchored** `test.skip`
grep. The full mechanism census is **167 call sites** = 158 anchored + **2 non-anchored same-line
conditionals** the anchored grep misses (`e2e/tests/storefront-characteristics.spec.ts:20`,
`e2e/tests/flow-core-lifecycles.spec.ts:443`) + **7 `test.fixme`** (which the census counted
separately: "158 `.skip` + 7 `.fixme`"). Zero delta once decomposed — the census was correct, the
"158" headline just under-scoped the mechanism set.

**Config-scoping facts (not skips, but oracle-shape facts):**
- The root config's `testDir` is `e2e/tests` — `e2e/visual/*` (3 specs) and
  `e2e/lifecycle-e2e/*` (1 spec) run only under their own configs. The "174-spec net" spans 3 configs.
- `webkit-smoke`/`webkit-mobile-smoke` projects use `testMatch: /.*\.smoke\.spec\.ts/` — an
  *inclusion* filter (2 files), not an exclusion of the rest.
- The visual net (162 comparisons) has **0 committed baselines** — it is effectively 100%
  non-running today. That is Phase-0 item 4's *sibling* blocker (lock baselines), tracked there, not
  in this skip-list.
- `e2e/MATRIX.md` is stale (2026-06-10, "28 spec files") — not used as a source here.

**Census unit = call site, not skipped test.** 14 sites live in `beforeAll`/`beforeEach`/module
scope and gate a whole file or group when they fire (e.g. `telegram-full-flow` beforeEach gates 23
tests; `courier-room-authz-isolation` beforeAll gates all 5 ADR-0013 tests). A run's reported
skipped-test count is therefore environment-dependent and always ≥ the fired-site count.

## Counts

**By mechanism (167):**

| Mechanism | Meaning | Count |
|---|---|---|
| `cond` | `test.skip(condition, reason)` inside a test body — runtime-conditional | 134 |
| `cond-true` | `test.skip(true, …)` or bare `test.skip()` reached behind an `if` — hard skip when the guard path is taken | 12 |
| `hook` | `test.skip(...)` in `beforeAll`/`beforeEach`/`describe`-body/module scope — gates the whole file/group | 14 |
| `fixme` | `test.fixme('title', fn)` — statically never runs | 7 |
| static `test.skip('title', fn)` / `describe.skip` / `.only` / config excludes | — | 0 |

**By WHY (167):** needs-staging-data **58** · unknown **58** (sub-classes: chain-guard 48 ·
missing-fixture 3 · not-built 4 · known-a11y-debt 3) · flaky **17** · env-dependent **17** ·
prod-guard **14** · feature-flag-off **3** · dead-feature **0**.

**By disposition (167):** KEEP-SKIP **142** · UNSKIP-CANDIDATE **25** · RETIRE **0**.

**Zero RETIREs is a finding, not an omission:** every candidate was grep-verified against
`apps/api/src` / `apps/web/src` / `packages/*/src` and every exercised route/component is alive
(e.g. `/api/courier/me/shift` → `routes/courier/shifts.ts:15`; `/api/dev/create-assignment` alive).
The 4 "not-built" fixmes cover **never-built** features (GPS surface, BackgroundWarning, WakeLock,
order-closed harness), which fails the RETIRE bar (proof-of-*deadness*, not proof-of-absence).

**The `chain-guard` sub-class (48 sites — the single biggest bucket):** `test.skip(!someId, 'No X
created')` where `someId` was hard-asserted by an earlier step in the same serial file. These are
defensive re-guards, not real coverage holes: when they fire, the suite is already red upstream (or
Playwright's serial-cascade already skipped them). They stay KEEP-SKIP with run-condition
"upstream step green"; a separate dead-guard cleanup is optional and out of scope here.

---

## Frozen skip-list

Columns: `spec_path` (`:line` appended) | `title` | `mechanism` | `why` | `disposition` |
`unblock_condition` (for KEEP-SKIP: the condition under which it runs today; for
UNSKIP-CANDIDATE: the change that unblocks it).

| spec_path | title | mechanism | why | disposition | unblock_condition |
|---|---|---|---|---|---|
| e2e/tests/admin-platform-authz.spec.ts:84 | provisioned platform-admin is admitted (200) | cond | env-dependent | KEEP-SKIP | runs when `QA_PLATFORM_ADMIN_TOKEN` is set (mint via `scripts/platform-admin-grant.ts`) |
| e2e/tests/admin-platform-authz.spec.ts:94 | DR-drill rate-limit: rapid 4th POST /backups/verify → 429 | cond | env-dependent | KEEP-SKIP | runs when `QA_PLATFORM_ADMIN_TOKEN` is set |
| e2e/tests/api-real.spec.ts:196 | POST /api/orders creates order with valid data | cond | flaky | KEEP-SKIP | runs when shared CI/test IP is under the 20/15min velocity gate (429/soft_confirm guard) |
| e2e/tests/api-real.spec.ts:219 | POST /api/orders rejects duplicate idempotency key | cond | flaky | KEEP-SKIP | same velocity-gate condition — 🔴 idempotency coverage silently drops when gated |
| e2e/tests/audit-fix-data-integrity.spec.ts:28 | (beforeAll — gates all 5 tests) | hook | prod-guard | KEEP-SKIP | runs when `VITE_BASE_URL` is non-prod (`isProdTarget` false, e.g. dowiz-staging.fly.dev) |
| e2e/tests/audit-fix-data-integrity.spec.ts:36 | (beforeEach — per-test re-guard) | hook | prod-guard | KEEP-SKIP | same; redundant twin of :28 |
| e2e/tests/capture-delivery.spec.ts:15 | capture courier delivery + tracking (390px) | cond | env-dependent | KEEP-SKIP | manual tool: runs when `CAPTURE=1` (+ `DEV_AUTH_SECRET`) |
| e2e/tests/capture-screens.spec.ts:9 | capture screens | cond | env-dependent | KEEP-SKIP | manual tool: runs when `CAPTURE_SCREENS=1` |
| e2e/tests/capture-states.spec.ts:23 | capture all states | cond | env-dependent | KEEP-SKIP | manual tool: runs when `CAPTURE=1` |
| e2e/tests/courier-room-authz-isolation.spec.ts:137 | (beforeAll — gates all 5 ADR-0013 tests) | hook | flaky | KEEP-SKIP | runs when courier invite-redeem is not 429-rate-limited this run — 🔴 WS-authz proof vanishes green on a 429 |
| e2e/tests/courier-room-authz-isolation.spec.ts:140 | (beforeAll — second courier) | hook | flaky | KEEP-SKIP | same rate-limit condition |
| e2e/tests/cross-tenant-realtime-qa.spec.ts:191 | Role 3 — COURIER dispatch: real online courier drives picked-up → IN_DELIVERY | cond | flaky | KEEP-SKIP | runs when courier seeding is not 429-rate-limited |
| e2e/tests/dashboard-courier-pins.spec.ts:27 | admin dashboard renders a real pin per on-shift courier | cond | env-dependent | KEEP-SKIP | runs when target has `DEV_AUTH_SECRET`/`ALLOW_DEV_LOGIN` (staging; closed on prod per ADR-0003) |
| e2e/tests/dashboard-courier-pins.spec.ts:46 | admin dashboard renders a real pin per on-shift courier | cond | needs-staging-data | UNSKIP-CANDIDATE | seed ≥2 orders via `POST /api/orders` in setup instead of relying on pre-existing `/api/owner/orders` rows |
| e2e/tests/deploy-validation.spec.ts:46 | 0.1 — local login returns a valid owner token | cond | prod-guard | KEEP-SKIP | runs on any non-prod target (by design: prod post-deploy run is read-only smoke) |
| e2e/tests/deploy-validation.spec.ts:100 | 3.1 — settings API returns slug resolving on public endpoints | cond | prod-guard | KEEP-SKIP | same `isProd` guard |
| e2e/tests/deploy-validation.spec.ts:120 | 4.1 — create category via owner API | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/deploy-validation.spec.ts:134 | 4.2 — create product with taste + recipeLines | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/deploy-validation.spec.ts:165 | 4.3 — PATCH product preserves recipeLines | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/deploy-validation.spec.ts:191 | 5.1 — public menu returns attributes with taste+bom | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/deploy-validation.spec.ts:233 | 6.2 — image upload with auth: 200/400, 500 is failure | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/deploy-validation.spec.ts:259 | 7.1 — menu import handles LLM unavailability (no 500) | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/deploy-validation.spec.ts:319 | 11.1 — admin product list includes taste + recipeLines | cond | prod-guard | KEEP-SKIP | same |
| e2e/tests/error-contract.spec.ts:17 | (beforeEach — gates all 6 envelope tests) | hook | env-dependent | KEEP-SKIP | runs on the `desktop` project only (deliberate: avoids 5× shared-IP rate-limit multiplication) |
| e2e/tests/flow-admin-deep.spec.ts:157 | Flow 4: Public — menu API returns product with allergens/taste/BOM | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 3 created the product (serial; guard redundant with serial-cascade) |
| e2e/tests/flow-client-product-images.spec.ts:15 | image added, served webp via proxy, displayed, changeable | hook | env-dependent | KEEP-SKIP | runs when `DEV_AUTH_SECRET` is set (describe-body, collection-time) |
| e2e/tests/flow-core-lifecycles.spec.ts:55 | (beforeEach — gates entire lifecycle describe) | hook | prod-guard | KEEP-SKIP | runs on non-prod target; companion `if (isProd) return;` bails in beforeAll:62 / afterAll:102 |
| e2e/tests/flow-core-lifecycles.spec.ts:171 | Flow 3: Owner — assign courier (tested in Flow 17) | cond-true | unknown | KEEP-SKIP | never runs (`test.skip(true,…)` redundancy stub; feature alive + covered by Flow 17 — candidate for deletion, needs lead sign-off) |
| e2e/tests/flow-core-lifecycles.spec.ts:175 | Flow 4: Owner — mark order as no-show | cond | needs-staging-data | KEEP-SKIP | runs when Flow 1 created `orderId` (serial chain) |
| e2e/tests/flow-core-lifecycles.spec.ts:177 | Flow 4 (state gate) | cond | needs-staging-data | KEEP-SKIP | runs when order still PENDING/CONFIRMED at execution time |
| e2e/tests/flow-core-lifecycles.spec.ts:187 | Flow 5: Owner — verify order detail | cond | needs-staging-data | KEEP-SKIP | runs when Flow 1 green |
| e2e/tests/flow-core-lifecycles.spec.ts:201 | Flow 6: Customer — cancel own order | cond | needs-staging-data | KEEP-SKIP | runs when Flow 1 green |
| e2e/tests/flow-core-lifecycles.spec.ts:203 | Flow 6 (state gate) | cond | needs-staging-data | KEEP-SKIP | runs when order not already REJECTED/CANCELLED |
| e2e/tests/flow-core-lifecycles.spec.ts:259 | Flow 8: Courier — redeem invite | cond | needs-staging-data | KEEP-SKIP | runs when Flow 7 created `inviteId` |
| e2e/tests/flow-core-lifecycles.spec.ts:288 | Flow 10: Courier — refresh token | cond | needs-staging-data | KEEP-SKIP | runs when Flow 8/9 set `courierRefreshToken` |
| e2e/tests/flow-core-lifecycles.spec.ts:301 | Flow 11: Courier — GET /me profile | cond | needs-staging-data | KEEP-SKIP | runs when courier auth chain green |
| e2e/tests/flow-core-lifecycles.spec.ts:312 | Flow 12: Courier — GET /me/audit-log | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-core-lifecycles.spec.ts:323 | Flow 13: Courier — GET /me/earnings + /me/history | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-core-lifecycles.spec.ts:343 | Flow 14: Courier — GET /me/payouts | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-core-lifecycles.spec.ts:359 | Flow 15: Courier — PATCH /me/password validation | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-core-lifecycles.spec.ts:373 | Flow 16: Courier — logout | cond | needs-staging-data | KEEP-SKIP | runs when `courierRefreshToken` set |
| e2e/tests/flow-core-lifecycles.spec.ts:401 | Flow 17: Courier — assignment accept/pickup/deliver/cancel | cond | needs-staging-data | KEEP-SKIP | runs when courier auth green AND in-test `dev/create-assignment` succeeds |
| e2e/tests/flow-core-lifecycles.spec.ts:408 | Flow 17 (accept ≠ 200 bail) | cond-true | flaky | KEEP-SKIP | runs when accept returns 200 first try; **only bare reason-less `test.skip()` in the estate — add a reason string** |
| e2e/tests/flow-core-lifecycles.spec.ts:437 | Flow 18: Courier — shift lifecycle | cond | needs-staging-data | KEEP-SKIP | runs when courier auth green |
| e2e/tests/flow-core-lifecycles.spec.ts:443 | Flow 18 (shift endpoint unavailable) | cond-true | flaky | KEEP-SKIP | runs when `GET /api/courier/me/shift` responds 200 in 5s (route alive: `routes/courier/shifts.ts:15`, unflagged) |
| e2e/tests/flow-core-lifecycles.spec.ts:498 | Flow 21: Owner — update modifier group | cond | needs-staging-data | KEEP-SKIP | runs when Flow 19 created `groupId` |
| e2e/tests/flow-core-lifecycles.spec.ts:509 | Flow 22: Owner — create modifier in group | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-core-lifecycles.spec.ts:522 | Flow 23: Owner — update modifier | cond | needs-staging-data | KEEP-SKIP | runs when Flow 22 created `modifierId` |
| e2e/tests/flow-core-lifecycles.spec.ts:533 | Flow 24: Owner — attach modifier group to product | cond | needs-staging-data | KEEP-SKIP | runs when `groupId` + `productId` both set |
| e2e/tests/flow-core-lifecycles.spec.ts:655 | Flow 30: Owner — product translations CRUD | cond | needs-staging-data | KEEP-SKIP | runs when beforeAll product POST returned 201 |
| e2e/tests/flow-courier-deep.spec.ts:46 | Flow 2: Courier — GET invite details before activation | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 1 green (guard structurally dead: hard assert + serial-cascade precede it) |
| e2e/tests/flow-courier-deep.spec.ts:65 | Flow 3: Owner — list active courier invites | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-courier-deep.spec.ts:179 | Flow 10: Owner — revoke courier invite | cond | unknown(chain-guard) | KEEP-SKIP | same; NOTE: file defaults BASE to prod with no `requireStaging()` — fails loudly at mock-auth, recommend adding the guard |
| e2e/tests/flow-customer-track-link.spec.ts:71 | order mint returns trackUrl with opaque ?t= grant | cond-true | needs-staging-data | KEEP-SKIP | runs when `POST /api/orders` for `sushi-durres` returns 201 (seeded location must pass range/min-order/velocity) |
| e2e/tests/flow-customer-track-link.spec.ts:89 | track/exchange returns customer JWT (no auth header) | cond | needs-staging-data | KEEP-SKIP | runs when TEST 1 minted a track code |
| e2e/tests/flow-customer-track-link.spec.ts:116 | trackUrl in fresh context renders order, not "session expired" | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-geo-tracking.spec.ts:56 | (beforeAll — gates whole geo-contract describe) | hook | needs-staging-data | KEEP-SKIP | proceeds when order-create for `sushi-durres` returns 201; **when it fires the WHOLE file skips — check "0 ran" reports** |
| e2e/tests/flow-geo-tracking.spec.ts:71 | customer status endpoint healthy, carries route when deployed | cond | needs-staging-data | KEEP-SKIP | runs when beforeAll order + track exchange green |
| e2e/tests/flow-geo-tracking.spec.ts:90 | (same test — route-field-absent branch) | cond-true | env-dependent | UNSKIP-CANDIDATE | likely STALE: `route` ships in `routes/customer/orders.ts:110-164` + migration 036 — verify on staging, then remove the skip branch |
| e2e/tests/flow-geo-tracking.spec.ts:96 | order page renders with geo changes (no regression) | cond | needs-staging-data | KEEP-SKIP | runs when beforeAll green |
| e2e/tests/flow-ingredients.spec.ts:85 | Flow 2: list modifier groups returns created group | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 1 green (hard-asserted 201 upstream) |
| e2e/tests/flow-ingredients.spec.ts:102 | Flow 3: update modifier group name + constraints round-trip | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ingredients.spec.ts:119 | Flow 4: create modifier with integer price_delta | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ingredients.spec.ts:137 | Flow 5: update modifier price_delta + availability | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 4 green |
| e2e/tests/flow-ingredients.spec.ts:152 | Flow 6: attach modifier group to product | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 1 + beforeAll green |
| e2e/tests/flow-ingredients.spec.ts:173 | Flow 7: delete modifier, verify gone | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 4 green |
| e2e/tests/flow-ingredients.spec.ts:192 | Flow 8: create modifier with zero price_delta + sort_order | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 1 green |
| e2e/tests/flow-modifiers-promotions.spec.ts:147 | Flow 3: GET order returns items with modifier details | cond | flaky | KEEP-SKIP | runs when Flow 1 returned clean 201 (soft_confirm anti-fraud outcome documented as expected) |
| e2e/tests/flow-modifiers-promotions.spec.ts:169 | Flow 5: Owner confirm order via API | cond | flaky | KEEP-SKIP | same |
| e2e/tests/flow-offline-phone-fallback.spec.ts:69 | offline banner surfaces tel: link to restaurant phone | cond-true | flaky | KEEP-SKIP | runs when order-create for `sushi-durres` returns 201 |
| e2e/tests/flow-order-creation.spec.ts:335 | duplicate idempotency key: same body replayed → 200/201 | cond-true | flaky | KEEP-SKIP | runs when first POST returns clean 201 (velocity/business-rule gate) — 🔴 idempotency coverage drops when gated |
| e2e/tests/flow-order-creation.spec.ts:381 | duplicate idempotency key: different body → 422 IDEMPOTENCY_KEY_REUSED | cond-true | flaky | KEEP-SKIP | same — 🔴 |
| e2e/tests/flow-orders-checkout.spec.ts:134 | Flow 2: GET order by ID returns full order with items | cond | flaky | KEEP-SKIP | runs when Flow 1 returned clean 201 (not soft_confirm/422) |
| e2e/tests/flow-orders-checkout.spec.ts:159 | Flow 3: Admin — PATCH status → CONFIRMED | cond | flaky | KEEP-SKIP | same |
| e2e/tests/flow-regulatory-settlements.spec.ts:124 | Flow 3: GDPR — get request detail | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 1 created the GDPR request (hard-asserted) |
| e2e/tests/flow-regulatory-settlements.spec.ts:157 | Flow 5: Owner — get settlement detail | cond | needs-staging-data | UNSKIP-CANDIDATE | seed ≥1 pending settlement for the tenant (file's own TODO at :59-60) |
| e2e/tests/flow-regulatory-settlements.spec.ts:191 | Flow 8: Owner — acknowledge signal | cond | needs-staging-data | UNSKIP-CANDIDATE | seed ≥1 active signal |
| e2e/tests/flow-regulatory-settlements.spec.ts:203 | Flow 9: Owner — dismiss signal | cond | needs-staging-data | UNSKIP-CANDIDATE | same seed |
| e2e/tests/flow-regulatory-settlements.spec.ts:216 | Flow 10: Owner — acknowledge alert | cond | needs-staging-data | UNSKIP-CANDIDATE | seed ≥1 active alert |
| e2e/tests/flow-regulatory-settlements.spec.ts:242 | Flow 12: Owner — settlement approve/pay/dispute | cond | needs-staging-data | UNSKIP-CANDIDATE | seed ≥1 pending settlement — highest-value row in this file (approve/dispute/reopen in one test) |
| e2e/tests/flow-regulatory-settlements.spec.ts:295 | Flow 14: Owner — update courier status | cond-true | needs-staging-data | UNSKIP-CANDIDATE | seed ≥1 courier (reuse invite+redeem helper flow) |
| e2e/tests/flow-sensor-delivery-baseline.spec.ts:24 | delivered order writes §1.2 normalised baseline into delivery_trace | hook | env-dependent | KEEP-SKIP | runs when `DEV_AUTH_SECRET` set (module-level, sole test in file) |
| e2e/tests/flow-sensor-geofence.spec.ts:29 | courier crossing venue geofence fires the sensor path | hook | env-dependent | KEEP-SKIP | runs when `DEV_AUTH_SECRET` set (module-level, sole test in file) |
| e2e/tests/flow-simpl-s6-claim.spec.ts:11 | §6 authenticated owner claims a shadow via fragment-token page | cond | needs-staging-data | UNSKIP-CANDIDATE | mint the claim invite/token programmatically in beforeAll instead of out-of-band `E2E_CLAIM_TOKEN` env — sole test in file, 100% skip rate today |
| e2e/tests/flow-ui-admin-branding.spec.ts:215 | Step 7: /branding-preview/{slug} loads without JS errors | cond | unknown(chain-guard) | KEEP-SKIP | runs when Step 2 resolved `locationSlug` (hard-asserted) |
| e2e/tests/flow-ui-admin-branding.spec.ts:253 | Step 8: /s/{slug} renders the client menu | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-branding.spec.ts:278 | Step 9: public theme endpoint reflects updated primary color | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-branding.spec.ts:291 | Step 10: UI — select logo, preview img appears in form | cond | unknown(missing-fixture) | UNSKIP-CANDIDATE | commit `dubin-logo.jpg` at repo root (or repoint `LOGO_PATH`) — `find` proves file missing repo-wide, so this **fires unconditionally in every env today** |
| e2e/tests/flow-ui-admin-branding.spec.ts:326 | Step 11: API — POST logo to theme/logo (locationId guard) | cond | unknown(chain-guard) | KEEP-SKIP | runs when Step 2 resolved `locationId` |
| e2e/tests/flow-ui-admin-branding.spec.ts:327 | Step 11 (fixture guard — the one that fires) | cond | unknown(missing-fixture) | UNSKIP-CANDIDATE | same fixture fix as :291 |
| e2e/tests/flow-ui-admin-branding.spec.ts:347 | Step 11 (upload endpoint 500 bail) | cond-true | env-dependent | KEEP-SKIP | unreachable until :327 is fixed; then runs unless upload endpoint 500s (sharp/storage) |
| e2e/tests/flow-ui-admin-branding.spec.ts:360 | Step 11b: logo upload to non-owned locationId is rejected | cond | unknown(missing-fixture) | UNSKIP-CANDIDATE | same fixture fix — **an IDOR negative test that never runs today** |
| e2e/tests/flow-ui-admin-branding.spec.ts:382 | Step 12: public theme returns logoUrl after upload | cond | unknown(chain-guard) | KEEP-SKIP | runs when Step 2 green (but its logo assertions are soft-gated on `logoUploaded` — see partial-skip note) |
| e2e/tests/flow-ui-admin-branding.spec.ts:399 | Step 13: /branding-preview shows logo when logoUrl set | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-dashboard.spec.ts:107 | prepare order status via API | cond | unknown(chain-guard) | KEEP-SKIP | runs when beforeAll created the order (hard-asserted) |
| e2e/tests/flow-ui-admin-menumanager.spec.ts:75 | create product via API | cond | unknown(chain-guard) | KEEP-SKIP | runs when fresh GET lists the just-created category (read-after-write consistency) |
| e2e/tests/flow-ui-admin-menumanager.spec.ts:145 | delete test product via API cleanup | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-menumanager.spec.ts:160 | delete test category via API cleanup | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-product-bom.spec.ts:251 | Step 2: open first product edit form via Menu Manager | cond | unknown(chain-guard) | KEEP-SKIP | runs when Step 1 green (guard structurally dead under serial-cascade) |
| e2e/tests/flow-ui-admin-product-bom.spec.ts:302 | Step 3: add recipe lines via RecipeEditor | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-product-bom.spec.ts:424 | Step 4: click Save Changes, verify success toast | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-product-bom.spec.ts:499 | Step 5: GET product, verify recipeLines saved | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-product-bom.spec.ts:528 | Step 6: reload /admin/menu → allergen chips on card | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-supply-library.spec.ts:194 | Step 3: add new ingredient via Add Supply form | cond | unknown(chain-guard) | KEEP-SKIP | structurally dead (Step 1 always sets `uiIngredient`, incl. explicit fallback) |
| e2e/tests/flow-ui-admin-supply-library.spec.ts:301 | Step 4: new supply card shows allergens + kcal | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-admin-supply-library.spec.ts:349 | Step 5: localStorage — bulk-inject remaining ingredients | cond | needs-staging-data | KEEP-SKIP | runs when live catalog matches >1 new `INGREDIENT_DB` entry (genuine content dependency) |
| e2e/tests/flow-ui-client-checkout.spec.ts:219 | Flow 5: admin dashboard shows the order | cond | unknown(chain-guard) | KEEP-SKIP | runs when Flow 2 green (serial + `requireStaging` in beforeAll) |
| e2e/tests/flow-ui-courier-actions.spec.ts:99 | assign courier via API | cond | unknown(chain-guard) | KEEP-SKIP | runs when beforeAll order-create green (hard-asserted) |
| e2e/tests/flow-ui-courier-core.spec.ts:139 | Flow 2: Owner — assign courier to order | cond | unknown(chain-guard) | KEEP-SKIP | structurally dead (beforeAll `expectUuid` throws first) |
| e2e/tests/flow-ui-courier-core.spec.ts:152 | Flow 3: Courier — accept task via UI | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-courier-full.spec.ts:125 | Step 4: UI — courier navigates invite link and registers | cond | unknown(chain-guard) | KEEP-SKIP | runs when Step 3 created `inviteId` (hard-asserted) |
| e2e/tests/flow-ui-courier-full.spec.ts:169 | Step 5: UI — courier app pages all load | cond | unknown(chain-guard) | KEEP-SKIP | runs when Step 4 minted `courierJwt` |
| e2e/tests/flow-ui-courier-full.spec.ts:195 | Step 6: API — /courier/me/shift returns 200 | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-courier-full.spec.ts:208 | Step 7: UI — courier starts a shift | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-courier-full.spec.ts:279 | Step 10: API — courier login with email+password | cond | unknown(chain-guard) | KEEP-SKIP | same (reason string is stale copy-paste: "No courier created") |
| e2e/tests/flow-ui-menu-interactions.spec.ts:17 | detail modal opens on product click, shows product info | cond | needs-staging-data | KEEP-SKIP | runs when /s/demo renders ≥1 menu-item card (guards the empty-menu/pool-starvation class) |
| e2e/tests/flow-ui-menu-interactions.spec.ts:112 | cart FAB updates count when items added | cond | needs-staging-data | KEEP-SKIP | runs when ≥1 product exposes an add button (not all sold-out) |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:140 | Step 2: customer places order via API | cond | needs-staging-data | KEEP-SKIP | runs when beforeAll product creation green (beforeAll uses `requireStaging` — throws, not skips, off-staging) |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:177 | Step 3: admin dashboard shows the new order | cond | needs-staging-data | KEEP-SKIP | runs when Step 2 green |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:198 | Step 4: admin confirms order via API | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:211 | Step 4b: dashboard shows CONFIRMED | cond | needs-staging-data | KEEP-SKIP | same + `ownerToken` |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:229 | Step 5: admin assigns courier to confirmed order | cond | needs-staging-data | KEEP-SKIP | runs when `orderId` + `courierId` set |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:243 | Step 5b: couriers page shows our courier | cond | needs-staging-data | KEEP-SKIP | runs when beforeAll auth green (largely redundant guard) |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:260 | Step 6: courier tasks page shows assignment | cond | needs-staging-data | KEEP-SKIP | runs when courier redeem green |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:276 | Step 6b: get assignment ID from API | cond | needs-staging-data | KEEP-SKIP | chain-critical: sole point populating `assignmentId` (from a list GET with no retry) |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:299 | Step 7: courier accepts assignment via API | cond | needs-staging-data | UNSKIP-CANDIDATE | one-line fix: capture `assignBody.id` (already `expectUuid`-validated in Step 5 at :237) into `assignmentId` — removes the fragile Step-6b list dependency for 5 tests |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:310 | Step 7b: delivery page loads after accepting | cond | needs-staging-data | UNSKIP-CANDIDATE | same fix |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:330 | Step 8: courier marks order picked up | cond | needs-staging-data | UNSKIP-CANDIDATE | same fix |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:342 | Step 9: courier marks order delivered | cond | needs-staging-data | UNSKIP-CANDIDATE | same fix |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:353 | Step 9b: delivery page shows completed state | cond | needs-staging-data | UNSKIP-CANDIDATE | same fix |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:370 | Step 10: order shows DELIVERED via API | cond | needs-staging-data | KEEP-SKIP | runs when Step 2 green; **hard-FAILS (not skips) if the assignmentId chain stalled** — upstream partial-skip surfaces as downstream failure here |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:386 | Step 10b: customer order status page shows order | cond | needs-staging-data | KEEP-SKIP | runs when Step 2 green |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:403 | Step 11: courier earnings page shows completed delivery | cond | needs-staging-data | KEEP-SKIP | runs when courier token set |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:419 | Step 12: courier history page shows delivery record | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/flow-ui-order-lifecycle.spec.ts:435 | Step 13: analytics page reflects completed order | cond | needs-staging-data | KEEP-SKIP | runs when owner token set |
| e2e/tests/flow-ui-owner-core.spec.ts:142 | Flow 4: admin confirm order via API (status transition) | cond | unknown(chain-guard) | KEEP-SKIP | structurally dead (beforeAll hard-asserts 201 first) |
| e2e/tests/flow-ui-owner-crud.spec.ts:93 | Flow 4: update product via API + round-trip | cond | unknown(chain-guard) | KEEP-SKIP | runs when fresh GET lists Flow-3's product; file lacks `requireStaging()` — recommend adding |
| e2e/tests/flow-ui-owner-crud.spec.ts:112 | Flow 5: delete test product via API | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-owner-crud.spec.ts:137 | Flow 6: delete test category via API | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-proof-comprehensive.spec.ts:342 | courier tasks page shows assignment with order info | cond | unknown(chain-guard) | KEEP-SKIP | runs when beforeAll invite+redeem green (hard-asserted) |
| e2e/tests/flow-ui-proof-comprehensive.spec.ts:366 | courier API assignments endpoint returns our assignment | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-proof-comprehensive.spec.ts:386 | courier accepts assignment; delivery page loads with map | cond | needs-staging-data | UNSKIP-CANDIDATE | add a poll/retry around the `assignmentId` lookup in beforeAll (single un-retried GET right after assign today) |
| e2e/tests/flow-ui-proof-comprehensive.spec.ts:415 | courier shift page shows timer and controls | cond | unknown(chain-guard) | KEEP-SKIP | runs when beforeAll green |
| e2e/tests/flow-ui-proof-comprehensive.spec.ts:436 | courier earnings page loads with summary | cond | unknown(chain-guard) | KEEP-SKIP | same |
| e2e/tests/flow-ui-settings-promotions.spec.ts:151 | toggle promotion active/inactive via API | cond | flaky | KEEP-SKIP | runs when the earlier create-promotion test created the fixture (no serial mode — guard genuinely reachable) |
| e2e/tests/flow-ui-settings-promotions.spec.ts:167 | delete promotion via API | cond | flaky | KEEP-SKIP | same |
| e2e/tests/live-smoke.spec.ts:105 | no critical a11y violations on menu page | fixme | unknown(known-a11y-debt) | UNSKIP-CANDIDATE | fix the 3 documented storefront a11y defects (unnamed icon button, sub-44px targets, unlabeled inputs) then delete `.fixme` — file comment admits these **fail against prod today**; carried debt, not not-built |
| e2e/tests/live-smoke.spec.ts:114 | touch targets on menu page meet 44px minimum | fixme | unknown(known-a11y-debt) | UNSKIP-CANDIDATE | same |
| e2e/tests/live-smoke.spec.ts:122 | form inputs have accessible labels | fixme | unknown(known-a11y-debt) | UNSKIP-CANDIDATE | same |
| e2e/tests/notif-categories-local.spec.ts:22 | preference-centre renders 3 categories, persists toggle | hook | env-dependent | KEEP-SKIP | runs when `LOCAL_UI_PROOF=1` + vite preview of a `VITE_TG_CATEGORY_GATING=true` build (module-level) |
| e2e/tests/onboarding-copy-qa.spec.ts:23 | O1: fresh owner lands on wizard, not bounced to login | cond | env-dependent | KEEP-SKIP | runs when target has `ALLOW_DEV_LOGIN=true` + `DEV_AUTH_SECRET` (ADR-0003; true on staging) |
| e2e/tests/onboarding-copy-qa.spec.ts:41 | O2+O4: stepper labels "Couriers"/"Test order" | cond | env-dependent | KEEP-SKIP | same |
| e2e/tests/onboarding-copy-qa.spec.ts:59 | O3: menu step subhead does not promise PDF import | cond | env-dependent | KEEP-SKIP | same |
| e2e/tests/owner-fixes-batch.spec.ts:39 | session: refresh round-trip returns fresh ~7d access token | cond | flaky | KEEP-SKIP | runs when login response includes `refresh_token` — two real server omission paths traced: dev-bypass Path-1 never returns one (`routes/auth/local.ts:70`); Path-2 silently omits it if the `auth_refresh_tokens` INSERT fails (`local.ts:149-165`). AUTH-09-adjacent — flag to auth council |
| e2e/tests/prod-smoke.spec.ts:60 | dev-login backdoor is closed (prod) | cond | env-dependent | KEEP-SKIP | inverse guard: runs against prod/non-staging (asserts 401); skips only when BASE matches `/staging/` — by design |
| e2e/tests/soft-access-gate.spec.ts:29 | consent not pre-checked; submit disabled until ticked | cond-true | feature-flag-off | KEEP-SKIP | runs when `VITE_ACCESS_GATE_PUBLIC_ENABLED=true` (FE build) AND `ACCESS_GATE_PUBLIC_ENABLED=true` (server, `config/index.ts:195`) — dark by design (FLAG-C01/S11) |
| e2e/tests/soft-access-gate.spec.ts:50 | /privacy link in consent label resolves (not 404) | cond-true | feature-flag-off | KEEP-SKIP | same flag pair |
| e2e/tests/storefront-characteristics.spec.ts:20 | (beforeEach — gates all 3 COMPARE/FILTER tests) | hook | feature-flag-off | KEEP-SKIP | runs when build carries `VITE_MENU_CHARACTERISTICS_COMPARISON=true` + `_FILTER=true` (`MenuPage.tsx:36-37`; ADR-0014, dark; staging-deploy build-args lesson applies) |
| e2e/tests/storefront-fonts-owner.spec.ts:65 | D: public storefront theme read returns owner font ids | cond | needs-staging-data | KEEP-SKIP | runs when the mock-auth owner fixture has an active location (non-empty `slug` from /owner/settings) |
| e2e/tests/storefront-fonts-owner.spec.ts:72 | E: /s/{slug} renders the non-base font | cond | needs-staging-data | KEEP-SKIP | same |
| e2e/tests/telegram-full-flow.spec.ts:63 | (beforeEach — gates all 23 P1–P9 tests) | hook | prod-guard | KEEP-SKIP | runs on staging (bot injection is staging-only); on prod all 23 report skipped-green by design |
| e2e/tests/telegram-webhook.spec.ts:29 | (beforeEach — gates all 11 tests) | hook | prod-guard | KEEP-SKIP | same |
| e2e/tests/ws-courier-assignment.spec.ts:57 | courier receives task_assigned via WS after dev creates assignment | cond | unknown(chain-guard) | KEEP-SKIP | structurally dead (beforeAll hard-asserts auth 200 first) |
| e2e/visual/courier-path.visual.spec.ts:103 | tasks — gps permission denied | fixme | unknown(not-built) | UNSKIP-CANDIDATE | build a GPS-status surface on TasksPage first (grep: zero `geolocation` hits in `courier/TasksPage.tsx`) — never-built, not dead |
| e2e/visual/courier-path.visual.spec.ts:192 | delivery — order closed banner | fixme | unknown(not-built) | UNSKIP-CANDIDATE | easiest fixme: DOM + WS handler already exist (`DeliveryPage.tsx:505` testid `courier-order-closed`, handler :174) — only a WS-inject test hook is missing |
| e2e/visual/courier-path.visual.spec.ts:216 | delivery — background warning | fixme | unknown(not-built) | UNSKIP-CANDIDATE | build a BackgroundWarning surface first (grep: zero hits repo-wide) |
| e2e/visual/courier-path.visual.spec.ts:224 | delivery — wake lock failure | fixme | unknown(not-built) | UNSKIP-CANDIDATE | build WakeLock acquisition + failure UI first (grep: zero hits) |

---

## Non-standard partial-skip risks (NOT in the frozen count — a separate defect class)

These are `if (...) return;` / `if (visible) { assert }` patterns that silently truncate assertions
while the test reports green. They are **not** skip mechanisms (Playwright counts them as passes),
but they are the same oracle-hole class one layer down. Named here so the rebuild's per-surface E2E
slices don't inherit them blind; candidates for the test-integrity guardrail family (AGENTS.md
banned-classes).

| Location | Pattern | Risk |
|---|---|---|
| e2e/tests/flow-ui-order-lifecycle.spec.ts:285 | `assignmentId` only set inside `if (assignments.length > 0)` | **Highest impact**: empty list → 5 tests cascade-skip and Step 10 then hard-fails; fixed by the Step-5 capture (UNSKIP rows above) |
| e2e/tests/flow-ui-admin-branding.spec.ts:387-394, 421-429 | logo assertions gated on `logoUploaded`/`hasLogo` | Always false today (missing fixture) → Step 12/13 logo assertions never execute |
| e2e/tests/flow-ui-admin-branding.spec.ts:471 | `if (!baseline \|\| !ownerToken) return;` in afterAll | Branding restore silently no-ops if baseline capture failed |
| e2e/tests/flow-core-lifecycles.spec.ts:350, 670, 720 | shape asserts inside `if (list.length > 0)` | Payout-detail / translation-locale / notification-target checks silently vanish on empty lists |
| e2e/tests/flow-orders-checkout.spec.ts:165-168 | `if (status !== 'PENDING') { log; return; }` | CONFIRM-transition test can pass without asserting the transition |
| e2e/tests/flow-ui-courier-actions.spec.ts:149-166 | `hasCashInput`/`hasAction` computed, **never asserted** | Cash-input/action-button render coverage is decorative |
| e2e/tests/flow-ui-proof-comprehensive.spec.ts:352-356, 468-473 | `hasDeliveryInfo` computed never asserted; price check gated on cache timing | Same class |
| e2e/tests/flow-ui-admin-dashboard.spec.ts:125-127; flow-ui-courier-full.spec.ts:219-230 | computed-but-unasserted stat cards / `shiftStarted` | Same class |
| Systemic: `flow-ui-*` family | `if (await X.isVisible().catch(() => false)) { expect(...) }` — dozens of sites | Selector regression leaves tests green on the trailing "no JS errors" check alone; flagged as a pattern, not enumerated per-line |
| e2e/tests/flow-regulatory-settlements.spec.ts:149-153, 277-283; flow-ui-admin-product-bom.spec.ts:568-571; flow-ui-admin-supply-library.spec.ts:340-404; flow-ui-admin-menumanager.spec.ts (4 tests) | empty-list / visibility-gated assertion blocks | Same class |

---

## Parity-oracle impact — surfaces losing coverage from KEEP-SKIPs, ranked

Ranking weighs: number of sites × depth of chain × red-line weight × whether the skip is
by-design (prod-guard/flag-off = healthy) or accidental (flaky/missing-data = rot).

**1. Courier + delivery lifecycle — the worst hole (≈55 of 167 sites).**
The courier accept→pickup→deliver oracle lives almost entirely inside fragile serial chains
(`flow-core-lifecycles` Flows 8–18, `flow-ui-order-lifecycle` Steps 5–13, `flow-ui-courier-full`,
`flow-ui-proof-comprehensive`, `flow-courier-deep`, `flow-ui-courier-core/actions`,
`dashboard-courier-pins`) plus 4 never-built visual states (`courier-path.visual` fixmes) and the
sensor-seam specs (dev-secret-gated). One upstream 429 or one un-retried list GET can silently
drop the entire courier slice from a run while the suite reports green. **Rebuild consequence:**
the S7 courier/dispatch cutover (🔴) will have the *thinnest* E2E slice of any surface unless the
Step-5 `assignmentId` capture fix + courier seed fixtures land first — they are the cheapest,
highest-leverage oracle repairs on this list.

**2. Realtime/WS authz (🔴) — small count, red-line weight.**
`courier-room-authz-isolation` (the ADR-0013 positive-control suite, 5 tests) is gated by a
`beforeAll` that skips the WHOLE file on a courier-auth 429 — the WS-authz proof can vanish
**green** on any rate-limited run. `ws-courier-assignment` and `cross-tenant-realtime-qa` Role-3
share the class. **Rebuild consequence:** the S6 realtime cutover DoD (E2E slice green) can be
satisfied by a run in which its core authz proof never executed. Mitigation before Phase B: make
the courier-auth seeding retry-after-cooldown, or make these skips FAIL the slice gate (skip ≠
green for red-line suites).

**3. Orders/money (🔴) — idempotency + regulatory actions.**
The idempotency-key contract (both `flow-order-creation` tests + both `api-real` tests) skips
whenever the shared-IP velocity gate fires — 🔴 money-adjacent coverage that degrades exactly when
CI traffic is high. The settlements/signals/alerts *action* endpoints
(`flow-regulatory-settlements` ×6) have **never run** absent seeded data (the file's own TODO).
**Rebuild consequence:** S5 orders/money cutover needs either a velocity-gate bypass for the E2E
principal (dev-gated) or dedicated idempotency runs, plus the settlement/signal/alert seed — 
otherwise the Rust port's settlement actions ship with zero E2E proof.

**4. Admin/owner surface — one real zero-coverage pocket.**
Branding **logo upload has zero effective E2E coverage today**: 3 sites (incl. an IDOR negative
test) gate on a `dubin-logo.jpg` fixture that does not exist anywhere in the repo — they fire
unconditionally in every environment, and two downstream soft-gated assertion blocks never
execute. The platform-admin positive leg (200-admittance + DR-drill 429) runs only with
`QA_PLATFORM_ADMIN_TOKEN`. The rest of the admin skips are benign chain-guards.
**Rebuild consequence:** S3 catalog/admin and S4 media cutovers inherit an untested logo-upload
path; commit the fixture before Phase B.

**5. Storefront — carried a11y debt + dark flags (mostly healthy).**
3 `live-smoke` fixmes are **admitted, currently-failing a11y defects on prod** (unnamed icon
button, sub-44px targets, unlabeled inputs) — the only "proven regression carried silently" in the
estate; they should be fixed-and-unfixmed on the Node stack so the Astro port has an honest
baseline. Characteristics (ADR-0014) and soft-access-gate skips are flag-off by design — they run
when the flags flip and belong in the dark-feature port slices. The geo-tracking `route`-absent
skip (:90) is likely stale (feature shipped) — a 5-minute staging check retires it.

**6. Auth — thinnest hole, but one live signal.**
Core auth E2E (`flow-security-contracts`, `no-cookies-invariant`, `admin-platform-authz` negative
legs, `owner-revocation`) carries **zero** skips. The one soft spot: the owner refresh round-trip
(`owner-fixes-batch:39`) self-skips on two real server omission paths (dev-bypass never returns a
refresh token; a swallowed INSERT failure silently omits it) — AUTH-09/AUTH-GAP-adjacent; feed to
the S2 auth council packet. The prod-guard/dev-secret skips (deploy-validation, onboarding,
sensors, telegram) are healthy by design: they run fully on staging, which is the oracle's target.

**Cross-cutting note for §8d:** the frozen number for final acceptance is **167 call sites**
(this doc), not "158". A full-suite staging run's *skipped-test* count will be higher than the
fired-site count (hook skips fan out) and environment-dependent — the stable, comparable metric is
this call-site census, re-derivable with the methodology commands above.
