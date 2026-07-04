# Route → Surface Map (generated) — REV-C1 fix for breaker CRIT-2

> **GENERATED FILE — do not hand-edit.** Source of truth: `matcher/route-templates.generated.ts`.
> Regenerate with `npx tsx docs/design/rebuild-cutover-harness/matcher/generate-route-map.ts > docs/design/rebuild-cutover-harness/route-surface-map.generated.md`.
> This is the machine-derived replacement for the hand-authored map that breaker-findings.md's
> CRIT-2 found to be phantom (it claimed `POST /orders`; the real registered route is
> `POST /api/orders`, mounted under the `/api` prefix — `apps/api/src/routes/orders.ts:73` +
> `apps/api/src/bootstrap/routes.ts:96`).

## How this was derived (reproducible, not hand-maintained)

1. **Extract every literal route registration** (file:line, method, in-file path):
   ```
   cd apps/api && grep -rnE "^\s*(fastify|server)\.(get|post|put|patch|delete|all|head|options|route)\(" src --include="*.ts" | grep -vE "\.test\.|\.spec\."
   ```
2. **Resolve each file's mount prefix** by reading its `fastify.register(plugin, { prefix: '...' })`
   call in `apps/api/src/bootstrap/routes.ts` (`registerCoreRoutes`, the load-bearing order) and
   the tail registrations in `apps/api/src/server.ts` (product-media.ts, refunds.ts, spa-proxy.ts,
   mock-auth.ts, acquisition/route.ts, admin/index.ts). This step is NOT grep-derivable from the
   route file alone — the prefix lives in the *caller*, which is exactly the class of bug that
   produced the original CRIT-2 phantom map (a path read out of context, without its mount prefix).
3. **Compute full path = prefix + in-file path** (self-prefixed files already embed the full path
   in-file and register with no `prefix` option at all — verified per-file).
4. **Cross-check against the independent census** in
   `docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md` (same grep command, run separately
   for that document) — 236/236, zero delta between the two independent extractions, and both were
   spot-verified directly against live source for a sample of files during this pass (2026-07-04).
5. **Assign a surface** (S1..S10 / UNMAPPED / INFRA_NEVER_FLIPS) per row from
   `docs/design/rebuild-plan/REBUILD-MAP.md`'s surface definitions +
   `docs/design/rebuild-cutover-harness/proposal.md` §4's illustrative ownership rows +
   `docs/design/rebuild-cutover-harness/breaker-findings.md`'s explicit family groupings. Every row
   that required a judgment call (no explicit prior source to cite) carries a `flag` saying so.
6. **Prove the partition mechanically** — `matcher/cutover-matcher.test.ts`'s "disjointness proof"
   suite synthesizes one concrete example path per template and asserts it matches EXACTLY that one
   template among the full 236-row set (no other template also matches). This is not a prose claim;
   it is an executable, currently-green test (`npx tsx --test matcher/cutover-matcher.test.ts`).

## Partition summary

**Total registered HTTP routes: 236** (matches `docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md` §1 "HTTP routes 236" — independently re-verified against live source during this pass, zero delta).

| Surface | Count |
|---|---|
| S1 storefront-read | 21 |
| S2 auth | 14 |
| S3 catalog CRUD | 58 |
| S4 media | 7 |
| S5 orders/money 🔴 | 30 |
| S6 realtime WS 🔴 | 0 |
| S7 courier/dispatch 🔴 | 38 |
| S8 jobs/notifications | 18 |
| S9 GDPR/compliance 🔴 | 5 |
| S10 platform-admin | 27 |
| UNMAPPED (taxonomy gap — always Node) | 15 |
| INFRA (never flips — always Node, by design) | 3 |
| **sum** | **236** |

Partition check: 236 === 236 → **PASS — every route maps to exactly one bucket (a surface, UNMAPPED, or INFRA_NEVER_FLIPS)**.

**59 of 236 rows carry an explicit `flag`** — a judgment call, a duplicate-implementation hazard, a path anomaly, or a cross-surface surprise. Nothing is silently forced into a surface; every non-obvious assignment says so inline.

## Full route → surface table

| # | Method | Path template | Surface | Source (file:line) | Flag |
|---|---|---|---|---|---|
| 1 | POST | `/api/owner/locations/:locationId/products` | S3 | products.ts:15 |  |
| 2 | GET | `/api/owner/locations/:locationId/products` | S3 | products.ts:52 |  |
| 3 | GET | `/api/owner/locations/:locationId/products/:id` | S3 | products.ts:99 |  |
| 4 | PATCH | `/api/owner/locations/:locationId/products/:id` | S3 | products.ts:117 |  |
| 5 | DELETE | `/api/owner/locations/:locationId/products/:id` | S3 | products.ts:168 |  |
| 6 | PUT | `/api/owner/locations/:locationId/products/:id/translations/:locale` | S3 | products.ts:187 |  |
| 7 | GET | `/api/owner/locations/:locationId/products/:id/translations` | S3 | products.ts:239 |  |
| 8 | DELETE | `/api/owner/locations/:locationId/products/:id/translations/:locale` | S3 | products.ts:263 |  |
| 9 | PUT | `/api/owner/locations/:locationId/products/:id/modifier-groups` | S3 | products.ts:289 |  |
| 10 | GET | `/api/owner/locations/:locationId/products/:id/modifier-groups` | S3 | products.ts:346 |  |
| 11 | GET | `/api/owner/menu/products` | S3 | products.ts:372 |  |
| 12 | POST | `/api/owner/menu/products` | S3 | products.ts:396 |  |
| 13 | PATCH | `/api/owner/menu/products/:productId` | S3 | products.ts:440 |  |
| 14 | DELETE | `/api/owner/menu/products/:productId` | S3 | products.ts:501 |  |
| 15 | POST | `/api/owner/locations/:locationId/categories` | S3 | categories.ts:20 |  |
| 16 | GET | `/api/owner/locations/:locationId/categories` | S3 | categories.ts:51 |  |
| 17 | GET | `/api/owner/locations/:locationId/categories/:id` | S3 | categories.ts:88 |  |
| 18 | PATCH | `/api/owner/locations/:locationId/categories/:id` | S3 | categories.ts:112 |  |
| 19 | DELETE | `/api/owner/locations/:locationId/categories/:id` | S3 | categories.ts:149 |  |
| 20 | GET | `/api/owner/menu/categories` | S3 | categories.ts:189 |  |
| 21 | POST | `/api/owner/menu/categories` | S3 | categories.ts:211 |  |
| 22 | DELETE | `/api/owner/menu/categories/:id` | S3 | categories.ts:233 |  |
| 23 | GET | `/api/owner/locations/:locationId/settlements` | S5 | settlements.ts:14 |  |
| 24 | GET | `/api/owner/locations/:locationId/settlements/:id` | S5 | settlements.ts:75 |  |
| 25 | POST | `/api/owner/locations/:locationId/settlements/:id/approve` | S5 | settlements.ts:110 |  |
| 26 | POST | `/api/owner/locations/:locationId/settlements/:id/pay` | S5 | settlements.ts:162 |  |
| 27 | POST | `/api/owner/locations/:locationId/settlements/:id/dispute` | S5 | settlements.ts:206 |  |
| 28 | POST | `/api/owner/locations/:locationId/settlements/:id/reopen` | S5 | settlements.ts:257 |  |
| 29 | POST | `/api/owner/locations/:locationId/settlements/regenerate` | S5 | settlements.ts:301 | handler processes ALL locations, not just :locationId (cross-tenant blast radius per census note) — a per-surface flip does not bound this route's effect to one tenant |
| 30 | POST | `/api/owner/locations/:locationId/modifier-groups` | S3 | modifier-groups.ts:14 |  |
| 31 | GET | `/api/owner/locations/:locationId/modifier-groups` | S3 | modifier-groups.ts:48 |  |
| 32 | PATCH | `/api/owner/locations/:locationId/modifier-groups/:id` | S3 | modifier-groups.ts:73 |  |
| 33 | DELETE | `/api/owner/locations/:locationId/modifier-groups/:id` | S3 | modifier-groups.ts:117 |  |
| 34 | POST | `/api/owner/locations/:locationId/modifier-groups/:groupId/modifiers` | S3 | modifier-groups.ts:136 |  |
| 35 | PATCH | `/api/owner/locations/:locationId/modifiers/:id` | S3 | modifier-groups.ts:172 |  |
| 36 | DELETE | `/api/owner/locations/:locationId/modifiers/:id` | S3 | modifier-groups.ts:214 |  |
| 37 | GET | `/api/owner/locations/:locationId/dashboard/snapshot` | S5 | dashboard.ts:23 |  |
| 38 | POST | `/api/owner/locations/:locationId/orders/:orderId/confirm` | S5 | dashboard.ts:193 |  |
| 39 | POST | `/api/owner/locations/:locationId/orders/:orderId/reject` | S5 | dashboard.ts:203 |  |
| 40 | POST | `/api/owner/locations/:locationId/orders/:orderId/assign-courier` | S5 | dashboard.ts:215 | SURFACE-OWNERSHIP SURPRISE: assigns a courier (S7 dispatch effect) via an S5-owned path (.../orders/:orderId/*) — path-ownership assigns S5, but the write touches courier-binding state S7 also depends on. Flag for both surfaces' cutover DoD. |
| 41 | POST | `/api/owner/locations/:locationId/orders/:orderId/pickup` | S5 | dashboard.ts:379 |  |
| 42 | POST | `/api/owner/locations/:locationId/orders/:orderId/deliver` | S5 | dashboard.ts:447 |  |
| 43 | GET | `/api/owner/locations/:locationId/orders/:orderId/verify` | S5 | dashboard.ts:539 |  |
| 44 | GET | `/api/owner/promotions` | S3 | promotions.ts:36 | not explicit in proposal.md §4's S1..S10 illustrative rows — assigned S3 (catalog/pricing config) by analogy; not council-confirmed |
| 45 | POST | `/api/owner/promotions` | S3 | promotions.ts:91 |  |
| 46 | POST | `/api/owner/promotions/validate` | S3 | promotions.ts:143 |  |
| 47 | GET | `/api/owner/promotions/:id` | S3 | promotions.ts:224 |  |
| 48 | PATCH | `/api/owner/promotions/:id` | S3 | promotions.ts:253 |  |
| 49 | DELETE | `/api/owner/promotions/:id` | S3 | promotions.ts:329 |  |
| 50 | GET | `/api/owner/locations/:locationId/signals` | S8 | signals.ts:20 | borderline S5(fraud)/S8(monitoring) — assigned S8 by elimination (risk-signal monitoring, not a money transaction itself) |
| 51 | GET | `/api/owner/locations/:locationId/signals/compute` | S8 | signals.ts:105 |  |
| 52 | POST | `/api/owner/locations/:locationId/signals/:signalId/acknowledge` | S8 | signals.ts:129 |  |
| 53 | POST | `/api/owner/locations/:locationId/signals/:signalId/dismiss` | S8 | signals.ts:167 |  |
| 54 | POST | `/api/owner/locations/:locationId/orders/:orderId/mark-no-show` | S5 | signals.ts:198 | SURFACE-OWNERSHIP SURPRISE: the SAME FILE (signals.ts) produces routes split across TWO surfaces (S8 for the first 4 rows, S5 for this one) — because this one route's path falls under .../orders/:orderId/*, not .../signals/*. File-level intuition ("this is the signals file → S8") would have mis-mapped this row; only the per-route template match is safe. |
| 55 | POST | `/api/owner/menu/products/:productId/media/presign` | S4 | product-media.ts:82 |  |
| 56 | POST | `/api/owner/menu/products/:productId/media/confirm` | S4 | product-media.ts:178 |  |
| 57 | POST | `/api/owner/menu/products/:productId/media/:mediaId/set-primary` | S4 | product-media.ts:271 |  |
| 58 | POST | `/api/owner/menu/products/:productId/media/reorder` | S4 | product-media.ts:303 |  |
| 59 | PATCH | `/api/owner/menu/products/:productId/media/:mediaId` | S4 | product-media.ts:330 |  |
| 60 | POST | `/api/owner/onboarding/start` | S10 | onboarding.ts:35 | borderline S2 (bootstrap_owner() SECURITY DEFINER mints the first membership — an auth-shaped operation) vs S10 (REBUILD-MAP §Phase-B lists "provisioning" under S10) — assigned S10; needs explicit council confirmation before either S2 or S10 flips |
| 61 | GET | `/api/owner/onboarding/:locationId/state` | S10 | onboarding.ts:144 |  |
| 62 | POST | `/api/owner/onboarding/:locationId/step/complete` | S10 | onboarding.ts:174 |  |
| 63 | POST | `/api/owner/onboarding/:locationId/step/:stepNum/skip` | S10 | onboarding.ts:247 |  |
| 64 | GET | `/api/owner/onboarding/:locationId/complete` | S10 | onboarding.ts:315 |  |
| 65 | GET | `/api/owner/locations/:locationId/notifications/targets` | S8 | notifications.ts:16 |  |
| 66 | GET | `/api/owner/locations/:locationId/notifications/status` | S8 | notifications.ts:32 |  |
| 67 | POST | `/api/owner/locations/:locationId/notifications/telegram/connect-init` | S8 | notifications.ts:54 |  |
| 68 | POST | `/api/owner/locations/:locationId/notifications/test` | S8 | notifications.ts:81 |  |
| 69 | PUT | `/api/owner/locations/:locationId/notifications/targets/:targetId` | S8 | notifications.ts:118 |  |
| 70 | POST | `/api/owner/locations/:locationId/gdpr-requests` | S9 | gdpr.ts:33 |  |
| 71 | GET | `/api/owner/locations/:locationId/gdpr-requests` | S9 | gdpr.ts:139 |  |
| 72 | GET | `/api/owner/locations/:locationId/gdpr-requests/:requestId` | S9 | gdpr.ts:199 |  |
| 73 | GET | `/api/owner/locations/:locationId/settings/retention` | S9 | gdpr.ts:257 |  |
| 74 | PUT | `/api/owner/locations/:locationId/settings/retention` | S9 | gdpr.ts:272 |  |
| 75 | GET | `/api/owner/locations/:locationId/couriers` | S7 | owner/couriers.ts:22 |  |
| 76 | PATCH | `/api/owner/locations/:locationId/couriers/:courierId` | S7 | owner/couriers.ts:79 |  |
| 77 | GET | `/api/owner/locations/:locationId/couriers/live` | S7 | owner/couriers.ts:147 |  |
| 78 | GET | `/api/owner/locations/:locationId/orders/:orderId/route` | S7 | owner/couriers.ts:205 | THE textbook CRIT-1 case: same "/orders/:orderId/" infix as every S5 order-action route, but the trailing literal segment "route" (not "deliver"/"confirm"/etc.) makes this S7, not S5. A longest-prefix router could never separate this from dashboard.ts's S5 rows sharing the identical prefix. |
| 79 | GET | `/api/owner/locations/:locationId/couriers/:courierId/details` | S7 | owner/couriers.ts:251 |  |
| 80 | PATCH | `/api/owner/locations/:locationId/kitchen-busy` | S3 | menu-availability.ts:22 |  |
| 81 | GET | `/api/owner/locations/:locationId/menu-schedules` | S3 | menu-availability.ts:76 |  |
| 82 | POST | `/api/owner/locations/:locationId/menu-schedules` | S3 | menu-availability.ts:95 |  |
| 83 | DELETE | `/api/owner/locations/:locationId/menu-schedules/:id` | S3 | menu-availability.ts:142 |  |
| 84 | GET | `/api/owner/locations/:locationId/theme` | S3 | themes.ts:17 |  |
| 85 | PUT | `/api/owner/locations/:locationId/theme` | S3 | themes.ts:46 |  |
| 86 | POST | `/api/owner/locations/:locationId/theme/logo` | S3 | themes.ts:119 | borderline S4 (file upload) — assigned S3 per breaker-findings.md's explicit grouping of theme+logo together |
| 87 | POST | `/api/owner/locations/:locationId/push/subscribe` | S8 | owner/push.ts:23 |  |
| 88 | POST | `/api/owner/locations/:locationId/push/unsubscribe` | S8 | owner/push.ts:66 |  |
| 89 | GET | `/api/owner/locations/:locationId/push/state` | S8 | owner/push.ts:81 |  |
| 90 | POST | `/api/owner/menu/import/preview` | S3 | menu-import.ts:24 | REV-7 two-writer: menu-import stays on NODE even after S3 flips to Rust — never actually flips despite the S3 assignment |
| 91 | POST | `/api/owner/menu/import/anonymous` | S3 | menu-import.ts:173 | REV-7 two-writer (stays Node); also PUBLIC/unauthenticated — no owner JWT at all |
| 92 | POST | `/api/owner/menu/import/commit` | S3 | menu-import.ts:231 | REV-7 two-writer (stays Node); mode=replace mass-deletes menu rows (🔴 bulk-edit) |
| 93 | GET | `/api/owner/locations/:locationId/settings/fallback` | S3 | owner/fallback.ts:21 | borderline S8 (fallback-channel = notification degradation) — assigned S3 per breaker-findings.md's explicit grouping |
| 94 | PUT | `/api/owner/locations/:locationId/settings/fallback` | S3 | owner/fallback.ts:43 |  |
| 95 | GET | `/api/owner/locations/:locationId/degradation` | S3 | owner/fallback.ts:68 | borderline S8 |
| 96 | POST | `/api/owner/locations/:locationId/courier-invites` | S7 | owner/courier-invites.ts:27 |  |
| 97 | GET | `/api/owner/locations/:locationId/courier-invites` | S7 | owner/courier-invites.ts:87 |  |
| 98 | DELETE | `/api/owner/locations/:locationId/courier-invites/:inviteId` | S7 | owner/courier-invites.ts:104 |  |
| 99 | GET | `/api/owner/locations/:locationId/alerts` | S8 | alerts.ts:16 |  |
| 100 | POST | `/api/owner/locations/:locationId/alerts/:alertId/acknowledge` | S8 | alerts.ts:106 |  |
| 101 | POST | `/api/owner/locations/:locationId/alerts/acknowledge-all` | S8 | alerts.ts:151 |  |
| 102 | GET | `/api/owner/activation/:locationId/status` | S10 | activation.ts:58 | borderline S3 (per-location config) — assigned S10 by analogy with onboarding (tenant-lifecycle gate: draft→live) |
| 103 | POST | `/api/owner/activation/:locationId/pickup` | S10 | activation.ts:72 |  |
| 104 | POST | `/api/owner/activation/:locationId/publish` | S10 | activation.ts:89 |  |
| 105 | GET | `/api/owner/:locationId/refunds` | S5 | refunds.ts:17 | PATH ANOMALY: missing the "locations/" segment present on every sibling S5/S3/S7/S8/S9 owner route (should be /api/owner/locations/:locationId/refunds). A template built by analogy with the sibling shape would MISS this route entirely — it needed its own literal template, discovered only by reading the file. |
| 106 | POST | `/api/owner/:locationId/refunds/:paymentId/sent` | S5 | refunds.ts:43 | same path anomaly as above |
| 107 | GET | `/api/owner/locations/:locationId/settings/dwell` | S3 | dwell-settings.ts:22 |  |
| 108 | PUT | `/api/owner/locations/:locationId/settings/dwell` | S3 | dwell-settings.ts:41 |  |
| 109 | POST | `/api/owner/locations/:locationId/orders/:orderId/reveal-customer-contact` | S5 | reveal-contact.ts:15 | borderline S9 (PII disclosure) — assigned S5 per breaker-findings.md's explicit grouping ("...reveal-customer-contact → S5 ... and reveal-contact (PII)") |
| 110 | PATCH | `/api/owner/locations/:locationId/orders/:orderId/metadata` | S5 | order-meta.ts:13 |  |
| 111 | POST | `/api/owner/locations/:id/menu/translate` | S3 | menu-translate.ts:10 | likely-dead route — no FE caller found (census grep of apps/web/src) |
| 112 | POST | `/api/owner/locations/:locationId/products/:productId/confirm-allergens` | S3 | menu-confirm.ts:10 | likely-dead route (no FE caller found) AND a food-safety/liability field — escalate before dropping, per Task-Exit Rule |
| 113 | PATCH | `/api/owner/locations/:locationId` | S3 | owner/locations.ts:9 | money-adjacent (tax_rate/delivery_fee_flat feed the order-total calc) but itself config, not a transaction |
| 114 | GET | `/api/courier/me/assignments` | S7 | courier/assignments.ts:74 |  |
| 115 | GET | `/api/courier/assignments/:id` | S7 | courier/assignments.ts:102 |  |
| 116 | POST | `/api/courier/assignments/:id/accept` | S7 | courier/assignments.ts:125 |  |
| 117 | POST | `/api/courier/assignments/:id/reject` | S7 | courier/assignments.ts:178 |  |
| 118 | POST | `/api/courier/assignments/:id/picked-up` | S7 | courier/assignments.ts:239 |  |
| 119 | POST | `/api/courier/assignments/:id/delivered` | S7 | courier/assignments.ts:292 | STATE + MONEY (cash-as-proof) — money-adjacent but path-owned S7 |
| 120 | POST | `/api/courier/assignments/:id/cancel` | S7 | courier/assignments.ts:413 |  |
| 121 | POST | `/api/courier/assignments/:id/abort` | S7 | courier/assignments.ts:482 |  |
| 122 | POST | `/api/courier/assignments/:id/decline` | S7 | courier/assignments.ts:535 |  |
| 123 | GET | `/api/courier/me` | S7 | courier/me.ts:36 |  |
| 124 | PATCH | `/api/courier/me/messenger` | S7 | courier/me.ts:76 |  |
| 125 | GET | `/api/courier/me/audit-log` | S7 | courier/me.ts:94 |  |
| 126 | PATCH | `/api/courier/me/password` | S7 | courier/me.ts:110 | AUTH-class op (password change + full session revoke) but path-owned S7 — see courier/auth.ts note below on the same cross-surface pattern |
| 127 | GET | `/api/courier/me/earnings` | S7 | courier/me.ts:177 | MONEY-adjacent (payout figures) but path-owned S7 — see courier/settlements.ts note (money is not a single atomic surface) |
| 128 | GET | `/api/courier/me/history` | S7 | courier/me.ts:249 |  |
| 129 | GET | `/api/courier/me/shift` | S7 | courier/shifts.ts:15 |  |
| 130 | POST | `/api/courier/me/shift/start` | S7 | courier/shifts.ts:60 |  |
| 131 | POST | `/api/courier/me/shift/end` | S7 | courier/shifts.ts:111 |  |
| 132 | POST | `/api/courier/shifts/transition` | S7 | courier/shifts.ts:173 |  |
| 133 | POST | `/api/courier/shifts/ping` | S7 | courier/shifts.ts:305 |  |
| 134 | POST | `/api/courier/auth/invites/:inviteId/redeem` | S7 | courier/auth.ts:23 | SURFACE-OWNERSHIP SURPRISE: mints a courier JWT (an S2-class auth operation) but is path-owned by S7 (falls under /api/courier/*). Carries the SAME cross-stack JWT-verification-parity obligation as S2 (REV-C4 body-kid round-trip) yet is NOT covered by S2's cutover DoD gate. Recommend: S7's cutover DoD explicitly inherits S2's JWT-parity gate before S7 flips, or these 5 routes get pulled into S2's gate scope regardless of path. |
| 135 | GET | `/api/courier/auth/invites/:inviteId` | S7 | courier/auth.ts:159 |  |
| 136 | POST | `/api/courier/auth/login` | S7 | courier/auth.ts:219 | same S2-class-obligation note as invites/redeem |
| 137 | POST | `/api/courier/auth/refresh` | S7 | courier/auth.ts:354 | same S2-class-obligation note |
| 138 | POST | `/api/courier/auth/logout` | S7 | courier/auth.ts:479 |  |
| 139 | GET | `/api/courier/me/payouts` | S7 | courier/settlements.ts:12 | SURFACE-OWNERSHIP SURPRISE: money (payout reads, 🔴 MONEY-tagged in the census) is NOT a single atomic surface — the OWNER side of the identical settlement/payout entity (owner/settlements.ts) is S5, while the COURIER-side read of the same money data is path-owned S7. A single "money flips atomically" mental model does not hold across roles. |
| 140 | GET | `/api/courier/me/payouts/:id` | S7 | courier/settlements.ts:51 | same money-split note |
| 141 | GET | `/api/customer/orders/:orderId/status` | S5 | customer/orders.ts:21 |  |
| 142 | POST | `/api/customer/orders/:orderId/rating` | S5 | customer/orders.ts:219 |  |
| 143 | POST | `/api/customer/orders/:orderId/cancel` | S5 | customer/orders.ts:259 |  |
| 144 | POST | `/api/customer/push/subscribe` | S8 | customer/push.ts:21 |  |
| 145 | POST | `/api/customer/push/unsubscribe` | S8 | customer/push.ts:64 |  |
| 146 | POST | `/api/customer/locations/:slug/otp/send` | S2 | customer/otp.ts:34 | CRIT-2 correction: proposal.md §4 claimed "POST /api/customer/otp/*"; the real registered path is /api/customer/locations/:slug/otp/send (prefix /api/customer + in-file /locations/:slug/otp/send) |
| 147 | POST | `/api/customer/locations/:slug/otp/verify` | S2 | customer/otp.ts:112 | same CRIT-2 correction |
| 148 | POST | `/api/customer/track/exchange` | S2 | customer/track.ts:28 |  |
| 149 | GET | `/api/auth/google` | S2 | auth.ts:34 |  |
| 150 | GET | `/api/auth/google/callback` | S2 | auth.ts:62 |  |
| 151 | POST | `/api/auth/exchange` | S2 | auth.ts:173 |  |
| 152 | POST | `/api/auth/telegram/start` | S2 | auth.ts:191 |  |
| 153 | GET | `/api/auth/telegram/poll` | S2 | auth.ts:202 |  |
| 154 | POST | `/api/auth/refresh` | S2 | auth.ts:235 |  |
| 155 | POST | `/api/auth/logout` | S2 | auth.ts:325 |  |
| 156 | POST | `/api/auth/courier/activate` | S2 | auth.ts:339 |  |
| 157 | POST | `/api/orders` | S5 | orders.ts:73 | CRIT-2 correction: proposal.md §4 claimed "POST /orders" (phantom — verified live-grep, real registration is fastify.post('/orders',...) at orders.ts:73 mounted under prefix "/api") |
| 158 | GET | `/api/orders/:id` | S5 | orders.ts:735 |  |
| 159 | PATCH | `/api/orders/:id/status` | S5 | orders.ts:864 |  |
| 160 | POST | `/api/orders/:orderId/messages` | S5 | order-messages.ts:32 | borderline S5/S8 (order-lifecycle messaging) — assigned S5 by path-ownership: proposal.md §4's S5 row explicitly claims the whole /api/orders/* namespace |
| 161 | GET | `/api/orders/:orderId/messages` | S5 | order-messages.ts:124 |  |
| 162 | POST | `/api/orders/:orderId/messages/read` | S5 | order-messages.ts:161 |  |
| 163 | POST | `/couriers/invites` | S7 | couriers.ts:8 | MAJOR SURPRISE: this path does NOT start with /api at all. Any path-ownership map keyed on "/api/..." prefixes (as the original phantom map effectively assumed) would silently MISS this route entirely — same failure class as CRIT-2's phantom paths, found independently here. Census flags it as likely-orphaned (no FE caller found); the courier-invite UI exclusively calls the DIFFERENT route /api/owner/locations/:locationId/courier-invites. |
| 164 | POST | `/api/auth/local/login` | S2 | auth/local.ts:36 |  |
| 165 | GET | `/images/*` | S1 | spa-proxy.ts:158 | explicit in proposal.md §4's S1 row despite being served by the legacy-named spa-proxy.ts file |
| 166 | GET | `/media/*` | S1 | spa-proxy.ts:184 |  |
| 167 | POST | `/api/owner/menu/products/:productId/image` | S4 | spa-proxy.ts:213 | DUPLICATE-IMPLEMENTATION hazard: a second, independent image-upload path (sharp-resize, single-shot) alongside product-media.ts's presign/confirm flow (same surface S4, but two maintained code paths for "set a product image" — a code-duplication risk INSIDE one surface, not just across surfaces). |
| 168 | POST | `/api/public/entry-photo` | S4 | spa-proxy.ts:268 |  |
| 169 | GET | `/api/owner/analytics` | UNMAPPED | spa-proxy.ts:296 | breaker-findings.md MEDIUM: reads the S5 orders/order_items money tables but is not itself S5-owned by any proposal.md row — "S5's whole family flips atomically" is false while this stays on Node. Needs an explicit S5 sub-scope decision or a permanent-Node carve-out, not a silent default. |
| 170 | GET | `/api/owner/analytics/product-orders` | UNMAPPED | spa-proxy.ts:375 | same gap as /api/owner/analytics |
| 171 | GET | `/api/owner/orders` | S5 | spa-proxy.ts:393 | the ONE spa-proxy.ts row that DOES match the phantom map's literal "GET\|PATCH /api/owner/orders/*" claim — but note there is no PATCH at this path; the owner order-ACTION routes (confirm/reject/deliver/etc.) live at a completely different path (dashboard.ts, /api/owner/locations/:locationId/orders/:orderId/*), so the phantom map's "/api/owner/orders/*" pattern was still wrong for everything except this one GET. |
| 172 | GET | `/api/owner/couriers` | S7 | spa-proxy.ts:452 |  |
| 173 | GET | `/api/public/theme/:slug` | S1 | spa-proxy.ts:506 |  |
| 174 | GET | `/api/owner/brand` | S3 | spa-proxy.ts:528 |  |
| 175 | PUT | `/api/owner/brand` | S3 | spa-proxy.ts:562 |  |
| 176 | POST | `/api/owner/brand/generate` | S3 | spa-proxy.ts:616 |  |
| 177 | GET | `/api/owner/settings` | S3 | spa-proxy.ts:667 |  |
| 178 | PUT | `/api/owner/settings` | S3 | spa-proxy.ts:701 |  |
| 179 | POST | `/api/owner/courier-invites` | S7 | spa-proxy.ts:742 | NAMING/PATH COLLISION hazard: a second, differently-shaped courier-invite-mint endpoint (flat /api/owner/courier-invites) coexists with owner/courier-invites.ts's /api/owner/locations/:locationId/courier-invites — same surface S7, two independent implementations, real maintenance/security-parity hazard (a fix to one can miss the other). |
| 180 | POST | `/api/owner/onboarding` | S10 | spa-proxy.ts:758 | NAMING/PATH COLLISION with onboarding.ts's /api/owner/onboarding/start family. ALSO an untracked cross-stack two-writer on products/locations/location_themes per breaker-findings.md MEDIUM finding — writes tables S3-Rust is supposed to own exclusively. |
| 181 | GET | `/api/owner/customers` | UNMAPPED | spa-proxy.ts:838 | CRM — no clean fit in S1..S10; breaker-findings.md MEDIUM names this exact route as an "invisible un-migratable route" |
| 182 | GET | `/api/owner/customers/:id/analytics` | UNMAPPED | spa-proxy.ts:856 | same gap as /api/owner/customers |
| 183 | GET | `/s/:slug/cart` | S1 | client-flow.ts:15 |  |
| 184 | GET | `/s/:slug/checkout` | S1 | client-flow.ts:16 |  |
| 185 | GET | `/s/:slug/order/:id` | S1 | client-flow.ts:17 |  |
| 186 | GET | `/s/:slug/orders/:orderId` | S1 | client-flow.ts:18 | legacy alias — keep as an alias/redirect in the Rust port, not a duplicate handler |
| 187 | GET | `/robots.txt` | S1 | seo.ts:45 |  |
| 188 | GET | `/sitemap.xml` | S1 | seo.ts:90 |  |
| 189 | GET | `/sitemap-locations-:shard.xml` | S1 | seo.ts:121 |  |
| 190 | GET | `/public/locations/:locationIdOrSlug/menu` | S1 | menu.ts:231 |  |
| 191 | GET | `/public/locations/:slug/info` | S1 | menu.ts:312 |  |
| 192 | GET | `/public/locations/:slug/products/:productId/media` | S1 | menu.ts:418 |  |
| 193 | POST | `/api/claim/accept` | S10 | claim.ts:17 | borderline S2 (mints reauth via verifyAuth) / S10 (the public-facing half of the acquisition/provisioning pipeline) — assigned S10 |
| 194 | POST | `/api/claim/request` | S10 | claim.ts:49 |  |
| 195 | POST | `/api/claim/decline` | S10 | claim.ts:69 |  |
| 196 | POST | `/api/telemetry` | UNMAPPED | telemetry.ts:37 | breaker-findings.md MEDIUM-named gap. Also structurally NOT read-only, so force-fitting it into S1 would break the proven "S1 = zero writes" invariant (breaker: "Confirmed sound: S1 read-only holds"). |
| 197 | POST | `/api/telemetry/abuse` | UNMAPPED | telemetry.ts:84 | same as /api/telemetry |
| 198 | GET | `/api/public/voice-config` | S1 | voice-config.ts:11 |  |
| 199 | GET | `/api/push/vapid-public-key` | S1 | vapid.ts:5 |  |
| 200 | GET | `/public/locations/:locationId/theme.css` | S1 | theme.ts:10 |  |
| 201 | GET | `/s/:slug` | S1 | ssr.ts:18 |  |
| 202 | GET | `/v1/rates` | S1 | rates.ts:14 |  |
| 203 | GET | `/s/:slug/manifest.webmanifest` | S1 | pwa.ts:7 |  |
| 204 | POST | `/api/funnel` | UNMAPPED | funnel.ts:34 | breaker-findings.md MEDIUM-named gap; same not-read-only issue as /api/telemetry |
| 205 | GET | `/api/public/locations/:slug/fallback-config` | S1 | fallback-config.ts:9 |  |
| 206 | GET | `/branding-preview/:slug` | S1 | branding-preview.ts:6 |  |
| 207 | POST | `/api/access-requests` | UNMAPPED | access-requests.ts:58 | public PII-capture write, flag-gated at REGISTRATION time (only mounted when ACCESS_GATE_PUBLIC_ENABLED=true) — no clean S1..S10 fit |
| 208 | POST | `/dev/mock-auth` | S2 | mock-auth.ts:14 | test-only token minter, dark behind ALLOW_DEV_LOGIN+DEV_AUTH_SECRET — RECOMMEND EXCLUDING all /dev/* and /api/dev/* routes from the cutover harness's scope entirely (never a production surface to flip), rather than folding them into S2 |
| 209 | POST | `/dev/create-assignment` | UNMAPPED | mock-auth.ts:122 | test-only DB-seeding, not an auth operation — recommend excluding from harness scope |
| 210 | POST | `/dev/seed-telegram-target` | UNMAPPED | mock-auth.ts:184 | test-only — recommend excluding from harness scope |
| 211 | POST | `/dev/repair-test-owner` | UNMAPPED | mock-auth.ts:204 | test-only — recommend excluding from harness scope |
| 212 | POST | `/dev/seed-visual-state` | UNMAPPED | mock-auth.ts:583 | test-only — recommend excluding from harness scope |
| 213 | POST | `/api/dev/seed-visual-state` | UNMAPPED | mock-auth.ts:584 | alias of /dev/seed-visual-state (same handler, two paths) — collapse to one in the Rust port; test-only, exclude from harness scope |
| 214 | GET | `/api/admin/backups` | S10 | admin/backups.ts:13 |  |
| 215 | POST | `/api/admin/backups/verify` | S10 | admin/backups.ts:73 |  |
| 216 | GET | `/api/admin/backups/dr-report` | S10 | admin/backups.ts:100 |  |
| 217 | GET | `/api/admin/fallback/health` | S10 | admin/fallback.ts:13 |  |
| 218 | POST | `/api/admin/fallback/r2-check` | S10 | admin/fallback.ts:47 |  |
| 219 | GET | `/api/admin/notification-audit` | S10 | admin/notification-audit.ts:17 |  |
| 220 | POST | `/internal/acquisition` | S10 | acquisition/route.ts:62 | SURFACE-OWNERSHIP SURPRISE: S10 (platform-admin) spans BOTH /api/admin/* AND /internal/* — two entirely different top-level path families own the same surface. "Surface = one path prefix" breaks even for S10, not only for the owner/locations family CRIT-1 targets. |
| 221 | POST | `/internal/acquisition/extract` | S10 | acquisition/route.ts:77 |  |
| 222 | POST | `/internal/acquisition/provision/mint` | S10 | acquisition/route.ts:90 |  |
| 223 | POST | `/internal/acquisition/provision/spine` | S10 | acquisition/route.ts:107 |  |
| 224 | POST | `/internal/acquisition/provision/hard-delete` | S10 | acquisition/route.ts:130 |  |
| 225 | POST | `/internal/acquisition/claim/verify` | S10 | acquisition/route.ts:142 |  |
| 226 | POST | `/internal/acquisition/claim/mint` | S10 | acquisition/route.ts:159 |  |
| 227 | POST | `/internal/acquisition/complaint` | S10 | acquisition/route.ts:186 |  |
| 228 | POST | `/internal/acquisition/retention/sweep` | S10 | acquisition/route.ts:199 |  |
| 229 | POST | `/api/dev/mock-auth` | S2 | server.ts:549 | DUPLICATE-IMPLEMENTATION of /dev/mock-auth (mock-auth.ts:14), plus a `fresh:true` mode the twin lacks — same "exclude from harness scope" recommendation |
| 230 | POST | `/api/dev/create-assignment` | UNMAPPED | server.ts:653 | DUPLICATE-IMPLEMENTATION of /dev/create-assignment (mock-auth.ts:122) — test-only, exclude from harness scope |
| 231 | POST | `/api/dev/seed-data` | UNMAPPED | server.ts:701 | test-only, no twin — exclude from harness scope |
| 232 | GET | `/livez` | INFRA_NEVER_FLIPS | health.ts:61 | liveness probe MUST stay a zero-dependency handler on whichever process is actually live — never a flippable business surface |
| 233 | GET | `/health` | INFRA_NEVER_FLIPS | health.ts:65 |  |
| 234 | POST | `/webhook/telegram/:secret` | S8 | telegram-webhook.ts:36 |  |
| 235 | POST | `/webhook/payments/plisio` | S5 | payments-webhook.ts:13 |  |
| 236 | GET | `/metrics` | INFRA_NEVER_FLIPS | metrics.ts:134 |  |

## Surface-ownership surprises (found while building this map, not hypothesized in advance)

1. **Money is not one atomic surface.** Owner-side settlement/payout data (`owner/settlements.ts`,
   under `/api/owner/locations/:locationId/settlements*`) is S5. The COURIER-side read of the
   IDENTICAL payout data (`courier/settlements.ts`, `/api/courier/me/payouts*`) is path-owned S7. A
   single "flip S5 and all money moves atomically" mental model does not hold — courier payout reads
   would stay on Node (or flip) independently of the owner-side ledger.
2. **Auth-class operations hide inside non-S2 surfaces.** `courier/auth.ts` (`/api/courier/auth/*`)
   mints/refreshes/revokes courier JWTs — the same class of operation S2 owns for owners/customers —
   but is path-owned S7 (falls under `/api/courier/*`). It carries the SAME cross-stack JWT-verification-
   parity obligation as S2 (REV-C4's body-`kid` round-trip) without being covered by S2's cutover DoD
   gate. `courier/me.ts`'s password-change route has the identical pattern.
3. **The infix problem recurs even where the breaker's own illustrative list didn't flag it.**
   `owner/couriers.ts:205` — `GET /api/owner/locations/:locationId/orders/:orderId/route` — shares the
   *exact* prefix-through-UUID as every S5 order-action route in `dashboard.ts` (same file family that
   motivated CRIT-1), yet is S7 (dispatch), not S5, because the trailing literal segment is `route` not
   `deliver`/`confirm`/etc. A longest-prefix router would have collapsed this into whichever rule owns
   the shared prefix — textbook CRIT-1, found in a THIRD file beyond the breaker's own settlements/
   couriers/notifications/gdpr/theme illustration.
4. **One file can straddle two surfaces.** `owner/signals.ts` registers 5 routes: 4 are S8 (risk-signal
   monitoring) and the 5th (`POST .../orders/:orderId/mark-no-show`) is S5 (it drives an order-status
   transition). "This is the signals file, so it's S8" would have mis-mapped one row in five.
5. **Not every route lives under `/api`.** `routes/couriers.ts:8` registers `POST /couriers/invites`
   with NO prefix at all — a route-ownership map keyed on `/api/...` prefixes (as the original phantom
   map implicitly assumed by only ever citing `/api/...` examples) would silently miss it entirely. Same
   failure class as CRIT-2's phantom paths, found independently in this pass. (Census flags this route as
   likely-orphaned — no FE caller found — but it is still a REGISTERED, reachable route today.)
6. **S10 spans two unrelated top-level prefixes.** `/api/admin/*` (backups/fallback/notification-audit)
   and `/internal/acquisition/*` (a completely different mount, gated by a different secret,
   deliberately decoupled from the dev-login family per breaker finding B4) are BOTH S10. "Surface = one
   path prefix" breaks even for the platform-admin surface, not only for the owner/locations family
   CRIT-1 was scoped against.
7. **S6 (WebSocket) cannot be expressed as a `(method, path)` rule at all.**
   `apps/api/src/websocket.ts:192` — `new WebSocketServer({ server: fastify.server })` — is
   constructed with NO `path` option, so the `ws` package intercepts every HTTP Upgrade request on the
   shared server regardless of URL. The FE always connects to `/ws`
   (`apps/web/src/lib/useWebSocket.ts:6`), but the SERVER does not enforce that — a path template for
   `/ws` would be a phantom precision the real server does not have. The matcher (`cutover-matcher.ts`)
   special-cases this: `isWebSocketUpgrade()` (checks the `Upgrade` header) is evaluated BEFORE any
   path-template matching, exactly mirroring the real server's actual (lack of) discrimination.
8. **Two independent implementations mint the same capability on different paths — twice.**
   (a) Courier-invite minting exists at BOTH `owner/courier-invites.ts`'s
   `/api/owner/locations/:locationId/courier-invites` and `spa-proxy.ts:742`'s flat
   `/api/owner/courier-invites` — both S7, same effect, two maintained code paths. (b) Product-image
   upload exists at BOTH `product-media.ts`'s presign/confirm flow and `spa-proxy.ts:213`'s single-shot
   sharp-resize POST — both S4, same effect, two maintained code paths. (c) Tenant onboarding exists at
   BOTH `owner/onboarding.ts`'s `/api/owner/onboarding/start` family and `spa-proxy.ts:758`'s flat
   `/api/owner/onboarding` — both assigned S10, and the second one is also breaker-findings.md's
   untracked `products`/`location_themes` two-writer. None of these are ROUTING collisions (the literal
   paths differ, so the matcher resolves each unambiguously) — they are maintenance/security-parity
   hazards: a fix to one implementation can silently miss its twin.
9. **A real taxonomy gap exists, confirmed mechanically, not just asserted by the breaker.** 15 routes
   (owner analytics ×2, owner customers/CRM ×2, `/api/telemetry` ×2, `/api/funnel`,
   `/api/access-requests`, and 8 dev/test-infra routes) have no clean home in S1..S10 as currently
   defined. They are marked `UNMAPPED` rather than forced into a nearby surface — they always resolve
   to `NODE_UNMAPPED`-equivalent behavior (stay on Node), which is safe, but the taxonomy gap itself
   needs an architect decision (either extend S1..S10 or explicitly retire these routes), not a silent
   default.

## Regeneration recipe (copy-paste)

```bash
# 1. Re-extract the raw census (sanity check against the hardcoded template list below):
cd apps/api && grep -rnE "^\s*(fastify|server)\.(get|post|put|patch|delete|all|head|options|route)\(" src --include="*.ts" | grep -vE "\.test\.|\.spec\." | wc -l
# Expect: 236

# 2. Edit docs/design/rebuild-cutover-harness/matcher/route-templates.generated.ts if routes changed.

# 3. Re-render this document:
npx tsx docs/design/rebuild-cutover-harness/matcher/generate-route-map.ts > docs/design/rebuild-cutover-harness/route-surface-map.generated.md

# 4. Re-prove disjointness + re-run every scenario test:
npx tsx --test docs/design/rebuild-cutover-harness/matcher/cutover-matcher.test.ts
```
