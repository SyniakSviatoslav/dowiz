/**
 * GENERATED DATA — route → surface assignments for the cutover matcher (REV-C1/REV-C2).
 *
 * DO NOT hand-edit surface assignments without re-running the regeneration recipe in
 * `../route-surface-map.generated.md` §"How to regenerate". This file and that markdown
 * table are ONE source of truth (this array); the markdown is rendered FROM it (or, until
 * the render script is wired into CI, kept in lockstep by hand — see the note there). This
 * is the whole point of REV-C1: the map must never again silently diverge from the router
 * config, because after this change there is only one artifact to diverge from itself.
 *
 * Extraction method (reproducible):
 *   1. `cd apps/api && grep -rnE "^\s*(fastify|server)\.(get|post|put|patch|delete|all|head|options|route)\(" src --include="*.ts" | grep -vE "\.test\.|\.spec\."`
 *      → every literal in-file path, with file:line.
 *   2. Resolve each file's mount prefix from `apps/api/src/bootstrap/routes.ts` (registerCoreRoutes)
 *      + the tail registrations in `apps/api/src/server.ts` (product-media.ts, refunds.ts,
 *      spa-proxy.ts, mock-auth.ts, acquisition/route.ts, admin/index.ts) — prefix is a
 *      `fastify.register(plugin, { prefix: '...' })` option, NOT grep-derivable from the
 *      route file alone, so it is resolved by reading the register call once per file.
 *   3. full path = prefix + in-file path (self-prefixed files already embed the full path
 *      in-file and have no `prefix` option — verified per-file, see per-entry `source`).
 *   4. 236/236 cross-checked against `docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md`
 *      (independently grep-derived census, same command) AND independently re-verified by
 *      direct grep against live source for every file below during this pass (2026-07-04) —
 *      zero delta between the two independent extractions.
 *   5. Surface assignment (S1..S10 / UNMAPPED / INFRA_NEVER_FLIPS) applied per-row using the
 *      REBUILD-MAP.md surface definitions + proposal.md §4's illustrative ownership rows +
 *      breaker-findings.md's explicit family groupings (settlements/refunds→S5,
 *      couriers/route/courier-invites→S7, notifications/alerts/signals/push→S8,
 *      gdpr-requests→S9, theme/dwell/fallback→S3). Every row that required a judgment call
 *      (no explicit prior source) carries a `flag` explaining the call — nothing is silently
 *      forced into a surface.
 *
 * Total: 236 registered HTTP routes (matches REBUILD-MAP.md §1 "HTTP routes 236"). WS (S6) is
 * NOT in this array — see cutover-matcher.ts `isWebSocketUpgrade`/`matchSurfaceForRequest` and
 * the "S6 is not a path-routable surface" finding in route-surface-map.generated.md.
 */

import type { RouteTemplate } from './matcher.js';

export const ROUTE_TEMPLATES: readonly RouteTemplate[] = [
  // ============================================================================================
  // OWNER — routes/owner/*.ts (113) — prefix resolution: bootstrap/routes.ts::registerCoreRoutes
  // ============================================================================================

  // -- products.ts (self-prefixed; 14) — S3 catalog CRUD
  { method: 'POST', template: '/api/owner/locations/:locationId/products', surface: 'S3', source: 'products.ts:15' },
  { method: 'GET', template: '/api/owner/locations/:locationId/products', surface: 'S3', source: 'products.ts:52' },
  { method: 'GET', template: '/api/owner/locations/:locationId/products/:id', surface: 'S3', source: 'products.ts:99' },
  { method: 'PATCH', template: '/api/owner/locations/:locationId/products/:id', surface: 'S3', source: 'products.ts:117' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/products/:id', surface: 'S3', source: 'products.ts:168' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/products/:id/translations/:locale', surface: 'S3', source: 'products.ts:187' },
  { method: 'GET', template: '/api/owner/locations/:locationId/products/:id/translations', surface: 'S3', source: 'products.ts:239' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/products/:id/translations/:locale', surface: 'S3', source: 'products.ts:263' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/products/:id/modifier-groups', surface: 'S3', source: 'products.ts:289' },
  { method: 'GET', template: '/api/owner/locations/:locationId/products/:id/modifier-groups', surface: 'S3', source: 'products.ts:346' },
  { method: 'GET', template: '/api/owner/menu/products', surface: 'S3', source: 'products.ts:372' },
  { method: 'POST', template: '/api/owner/menu/products', surface: 'S3', source: 'products.ts:396' },
  { method: 'PATCH', template: '/api/owner/menu/products/:productId', surface: 'S3', source: 'products.ts:440' },
  { method: 'DELETE', template: '/api/owner/menu/products/:productId', surface: 'S3', source: 'products.ts:501' },

  // -- categories.ts (self-prefixed; 8) — S3
  { method: 'POST', template: '/api/owner/locations/:locationId/categories', surface: 'S3', source: 'categories.ts:20' },
  { method: 'GET', template: '/api/owner/locations/:locationId/categories', surface: 'S3', source: 'categories.ts:51' },
  { method: 'GET', template: '/api/owner/locations/:locationId/categories/:id', surface: 'S3', source: 'categories.ts:88' },
  { method: 'PATCH', template: '/api/owner/locations/:locationId/categories/:id', surface: 'S3', source: 'categories.ts:112' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/categories/:id', surface: 'S3', source: 'categories.ts:149' },
  { method: 'GET', template: '/api/owner/menu/categories', surface: 'S3', source: 'categories.ts:189' },
  { method: 'POST', template: '/api/owner/menu/categories', surface: 'S3', source: 'categories.ts:211' },
  { method: 'DELETE', template: '/api/owner/menu/categories/:id', surface: 'S3', source: 'categories.ts:233' },

  // -- settlements.ts (prefix /api/owner/locations; 7) — S5 money 🔴
  { method: 'GET', template: '/api/owner/locations/:locationId/settlements', surface: 'S5', source: 'settlements.ts:14' },
  { method: 'GET', template: '/api/owner/locations/:locationId/settlements/:id', surface: 'S5', source: 'settlements.ts:75' },
  { method: 'POST', template: '/api/owner/locations/:locationId/settlements/:id/approve', surface: 'S5', source: 'settlements.ts:110' },
  { method: 'POST', template: '/api/owner/locations/:locationId/settlements/:id/pay', surface: 'S5', source: 'settlements.ts:162' },
  { method: 'POST', template: '/api/owner/locations/:locationId/settlements/:id/dispute', surface: 'S5', source: 'settlements.ts:206' },
  { method: 'POST', template: '/api/owner/locations/:locationId/settlements/:id/reopen', surface: 'S5', source: 'settlements.ts:257' },
  {
    method: 'POST',
    template: '/api/owner/locations/:locationId/settlements/regenerate',
    surface: 'S5',
    source: 'settlements.ts:301',
    flag: 'handler processes ALL locations, not just :locationId (cross-tenant blast radius per census note) — a per-surface flip does not bound this route\'s effect to one tenant',
  },

  // -- modifier-groups.ts (self-prefixed; 7) — S3
  { method: 'POST', template: '/api/owner/locations/:locationId/modifier-groups', surface: 'S3', source: 'modifier-groups.ts:14' },
  { method: 'GET', template: '/api/owner/locations/:locationId/modifier-groups', surface: 'S3', source: 'modifier-groups.ts:48' },
  { method: 'PATCH', template: '/api/owner/locations/:locationId/modifier-groups/:id', surface: 'S3', source: 'modifier-groups.ts:73' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/modifier-groups/:id', surface: 'S3', source: 'modifier-groups.ts:117' },
  { method: 'POST', template: '/api/owner/locations/:locationId/modifier-groups/:groupId/modifiers', surface: 'S3', source: 'modifier-groups.ts:136' },
  { method: 'PATCH', template: '/api/owner/locations/:locationId/modifiers/:id', surface: 'S3', source: 'modifier-groups.ts:172' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/modifiers/:id', surface: 'S3', source: 'modifier-groups.ts:214' },

  // -- dashboard.ts (prefix /api/owner/locations; 7) — S5 order state-machine 🔴
  { method: 'GET', template: '/api/owner/locations/:locationId/dashboard/snapshot', surface: 'S5', source: 'dashboard.ts:23' },
  { method: 'POST', template: '/api/owner/locations/:locationId/orders/:orderId/confirm', surface: 'S5', source: 'dashboard.ts:193' },
  { method: 'POST', template: '/api/owner/locations/:locationId/orders/:orderId/reject', surface: 'S5', source: 'dashboard.ts:203' },
  {
    method: 'POST',
    template: '/api/owner/locations/:locationId/orders/:orderId/assign-courier',
    surface: 'S5',
    source: 'dashboard.ts:215',
    flag: 'SURFACE-OWNERSHIP SURPRISE: assigns a courier (S7 dispatch effect) via an S5-owned path (.../orders/:orderId/*) — path-ownership assigns S5, but the write touches courier-binding state S7 also depends on. Flag for both surfaces\' cutover DoD.',
  },
  { method: 'POST', template: '/api/owner/locations/:locationId/orders/:orderId/pickup', surface: 'S5', source: 'dashboard.ts:379' },
  { method: 'POST', template: '/api/owner/locations/:locationId/orders/:orderId/deliver', surface: 'S5', source: 'dashboard.ts:447' },
  { method: 'GET', template: '/api/owner/locations/:locationId/orders/:orderId/verify', surface: 'S5', source: 'dashboard.ts:539' },

  // -- promotions.ts (self-prefixed, flat /api/owner/promotions; 6) — S3 (by analogy)
  { method: 'GET', template: '/api/owner/promotions', surface: 'S3', source: 'promotions.ts:36', flag: 'not explicit in proposal.md §4\'s S1..S10 illustrative rows — assigned S3 (catalog/pricing config) by analogy; not council-confirmed' },
  { method: 'POST', template: '/api/owner/promotions', surface: 'S3', source: 'promotions.ts:91' },
  { method: 'POST', template: '/api/owner/promotions/validate', surface: 'S3', source: 'promotions.ts:143' },
  { method: 'GET', template: '/api/owner/promotions/:id', surface: 'S3', source: 'promotions.ts:224' },
  { method: 'PATCH', template: '/api/owner/promotions/:id', surface: 'S3', source: 'promotions.ts:253' },
  { method: 'DELETE', template: '/api/owner/promotions/:id', surface: 'S3', source: 'promotions.ts:329' },

  // -- signals.ts (prefix /api/owner/locations; 5) — SPLIT S8/S5, see flag
  { method: 'GET', template: '/api/owner/locations/:locationId/signals', surface: 'S8', source: 'signals.ts:20', flag: 'borderline S5(fraud)/S8(monitoring) — assigned S8 by elimination (risk-signal monitoring, not a money transaction itself)' },
  { method: 'GET', template: '/api/owner/locations/:locationId/signals/compute', surface: 'S8', source: 'signals.ts:105' },
  { method: 'POST', template: '/api/owner/locations/:locationId/signals/:signalId/acknowledge', surface: 'S8', source: 'signals.ts:129' },
  { method: 'POST', template: '/api/owner/locations/:locationId/signals/:signalId/dismiss', surface: 'S8', source: 'signals.ts:167' },
  {
    method: 'POST',
    template: '/api/owner/locations/:locationId/orders/:orderId/mark-no-show',
    surface: 'S5',
    source: 'signals.ts:198',
    flag: 'SURFACE-OWNERSHIP SURPRISE: the SAME FILE (signals.ts) produces routes split across TWO surfaces (S8 for the first 4 rows, S5 for this one) — because this one route\'s path falls under .../orders/:orderId/*, not .../signals/*. File-level intuition ("this is the signals file → S8") would have mis-mapped this row; only the per-route template match is safe.',
  },

  // -- product-media.ts (registered in server.ts, prefix /api/owner; 5) — S4 media
  { method: 'POST', template: '/api/owner/menu/products/:productId/media/presign', surface: 'S4', source: 'product-media.ts:82' },
  { method: 'POST', template: '/api/owner/menu/products/:productId/media/confirm', surface: 'S4', source: 'product-media.ts:178' },
  { method: 'POST', template: '/api/owner/menu/products/:productId/media/:mediaId/set-primary', surface: 'S4', source: 'product-media.ts:271' },
  { method: 'POST', template: '/api/owner/menu/products/:productId/media/reorder', surface: 'S4', source: 'product-media.ts:303' },
  { method: 'PATCH', template: '/api/owner/menu/products/:productId/media/:mediaId', surface: 'S4', source: 'product-media.ts:330' },

  // -- onboarding.ts (prefix /api/owner; 5) — S10 (provisioning), borderline S2
  {
    method: 'POST',
    template: '/api/owner/onboarding/start',
    surface: 'S10',
    source: 'onboarding.ts:35',
    flag: 'borderline S2 (bootstrap_owner() SECURITY DEFINER mints the first membership — an auth-shaped operation) vs S10 (REBUILD-MAP §Phase-B lists "provisioning" under S10) — assigned S10; needs explicit council confirmation before either S2 or S10 flips',
  },
  { method: 'GET', template: '/api/owner/onboarding/:locationId/state', surface: 'S10', source: 'onboarding.ts:144' },
  { method: 'POST', template: '/api/owner/onboarding/:locationId/step/complete', surface: 'S10', source: 'onboarding.ts:174' },
  { method: 'POST', template: '/api/owner/onboarding/:locationId/step/:stepNum/skip', surface: 'S10', source: 'onboarding.ts:247' },
  { method: 'GET', template: '/api/owner/onboarding/:locationId/complete', surface: 'S10', source: 'onboarding.ts:315' },

  // -- notifications.ts (self-prefixed; 5) — S8
  { method: 'GET', template: '/api/owner/locations/:locationId/notifications/targets', surface: 'S8', source: 'notifications.ts:16' },
  { method: 'GET', template: '/api/owner/locations/:locationId/notifications/status', surface: 'S8', source: 'notifications.ts:32' },
  { method: 'POST', template: '/api/owner/locations/:locationId/notifications/telegram/connect-init', surface: 'S8', source: 'notifications.ts:54' },
  { method: 'POST', template: '/api/owner/locations/:locationId/notifications/test', surface: 'S8', source: 'notifications.ts:81' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/notifications/targets/:targetId', surface: 'S8', source: 'notifications.ts:118' },

  // -- gdpr.ts (prefix /api/owner/locations; 5) — S9 🔴 NEVER co-flip with S5
  { method: 'POST', template: '/api/owner/locations/:locationId/gdpr-requests', surface: 'S9', source: 'gdpr.ts:33' },
  { method: 'GET', template: '/api/owner/locations/:locationId/gdpr-requests', surface: 'S9', source: 'gdpr.ts:139' },
  { method: 'GET', template: '/api/owner/locations/:locationId/gdpr-requests/:requestId', surface: 'S9', source: 'gdpr.ts:199' },
  { method: 'GET', template: '/api/owner/locations/:locationId/settings/retention', surface: 'S9', source: 'gdpr.ts:257' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/settings/retention', surface: 'S9', source: 'gdpr.ts:272' },

  // -- couriers.ts (owner, self-prefixed; 5) — S7
  { method: 'GET', template: '/api/owner/locations/:locationId/couriers', surface: 'S7', source: 'owner/couriers.ts:22' },
  { method: 'PATCH', template: '/api/owner/locations/:locationId/couriers/:courierId', surface: 'S7', source: 'owner/couriers.ts:79' },
  { method: 'GET', template: '/api/owner/locations/:locationId/couriers/live', surface: 'S7', source: 'owner/couriers.ts:147' },
  {
    method: 'GET',
    template: '/api/owner/locations/:locationId/orders/:orderId/route',
    surface: 'S7',
    source: 'owner/couriers.ts:205',
    flag: 'THE textbook CRIT-1 case: same "/orders/:orderId/" infix as every S5 order-action route, but the trailing literal segment "route" (not "deliver"/"confirm"/etc.) makes this S7, not S5. A longest-prefix router could never separate this from dashboard.ts\'s S5 rows sharing the identical prefix.',
  },
  { method: 'GET', template: '/api/owner/locations/:locationId/couriers/:courierId/details', surface: 'S7', source: 'owner/couriers.ts:251' },

  // -- menu-availability.ts (self-prefixed; 4) — S3
  { method: 'PATCH', template: '/api/owner/locations/:locationId/kitchen-busy', surface: 'S3', source: 'menu-availability.ts:22' },
  { method: 'GET', template: '/api/owner/locations/:locationId/menu-schedules', surface: 'S3', source: 'menu-availability.ts:76' },
  { method: 'POST', template: '/api/owner/locations/:locationId/menu-schedules', surface: 'S3', source: 'menu-availability.ts:95' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/menu-schedules/:id', surface: 'S3', source: 'menu-availability.ts:142' },

  // -- themes.ts (self-prefixed; 3) — S3
  { method: 'GET', template: '/api/owner/locations/:locationId/theme', surface: 'S3', source: 'themes.ts:17' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/theme', surface: 'S3', source: 'themes.ts:46' },
  { method: 'POST', template: '/api/owner/locations/:locationId/theme/logo', surface: 'S3', source: 'themes.ts:119', flag: 'borderline S4 (file upload) — assigned S3 per breaker-findings.md\'s explicit grouping of theme+logo together' },

  // -- push.ts (owner, prefix /api/owner/locations; 3) — S8
  { method: 'POST', template: '/api/owner/locations/:locationId/push/subscribe', surface: 'S8', source: 'owner/push.ts:23' },
  { method: 'POST', template: '/api/owner/locations/:locationId/push/unsubscribe', surface: 'S8', source: 'owner/push.ts:66' },
  { method: 'GET', template: '/api/owner/locations/:locationId/push/state', surface: 'S8', source: 'owner/push.ts:81' },

  // -- menu-import.ts (prefix /api/owner; 3) — S3, stays Node forever (REV-7 two-writer)
  { method: 'POST', template: '/api/owner/menu/import/preview', surface: 'S3', source: 'menu-import.ts:24', flag: 'REV-7 two-writer: menu-import stays on NODE even after S3 flips to Rust — never actually flips despite the S3 assignment' },
  { method: 'POST', template: '/api/owner/menu/import/anonymous', surface: 'S3', source: 'menu-import.ts:173', flag: 'REV-7 two-writer (stays Node); also PUBLIC/unauthenticated — no owner JWT at all' },
  { method: 'POST', template: '/api/owner/menu/import/commit', surface: 'S3', source: 'menu-import.ts:231', flag: 'REV-7 two-writer (stays Node); mode=replace mass-deletes menu rows (🔴 bulk-edit)' },

  // -- fallback.ts (owner, prefix /api/owner/locations; 3) — S3
  { method: 'GET', template: '/api/owner/locations/:locationId/settings/fallback', surface: 'S3', source: 'owner/fallback.ts:21', flag: 'borderline S8 (fallback-channel = notification degradation) — assigned S3 per breaker-findings.md\'s explicit grouping' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/settings/fallback', surface: 'S3', source: 'owner/fallback.ts:43' },
  { method: 'GET', template: '/api/owner/locations/:locationId/degradation', surface: 'S3', source: 'owner/fallback.ts:68', flag: 'borderline S8' },

  // -- courier-invites.ts (owner, self-prefixed; 3) — S7
  { method: 'POST', template: '/api/owner/locations/:locationId/courier-invites', surface: 'S7', source: 'owner/courier-invites.ts:27' },
  { method: 'GET', template: '/api/owner/locations/:locationId/courier-invites', surface: 'S7', source: 'owner/courier-invites.ts:87' },
  { method: 'DELETE', template: '/api/owner/locations/:locationId/courier-invites/:inviteId', surface: 'S7', source: 'owner/courier-invites.ts:104' },

  // -- alerts.ts (prefix /api/owner/locations; 3) — S8
  { method: 'GET', template: '/api/owner/locations/:locationId/alerts', surface: 'S8', source: 'alerts.ts:16' },
  { method: 'POST', template: '/api/owner/locations/:locationId/alerts/:alertId/acknowledge', surface: 'S8', source: 'alerts.ts:106' },
  { method: 'POST', template: '/api/owner/locations/:locationId/alerts/acknowledge-all', surface: 'S8', source: 'alerts.ts:151' },

  // -- activation.ts (prefix /api/owner; 3) — S10, borderline S3
  { method: 'GET', template: '/api/owner/activation/:locationId/status', surface: 'S10', source: 'activation.ts:58', flag: 'borderline S3 (per-location config) — assigned S10 by analogy with onboarding (tenant-lifecycle gate: draft→live)' },
  { method: 'POST', template: '/api/owner/activation/:locationId/pickup', surface: 'S10', source: 'activation.ts:72' },
  { method: 'POST', template: '/api/owner/activation/:locationId/publish', surface: 'S10', source: 'activation.ts:89' },

  // -- refunds.ts (registered in server.ts, prefix /api/owner; 2) — S5, PATH ANOMALY
  {
    method: 'GET',
    template: '/api/owner/:locationId/refunds',
    surface: 'S5',
    source: 'refunds.ts:17',
    flag: 'PATH ANOMALY: missing the "locations/" segment present on every sibling S5/S3/S7/S8/S9 owner route (should be /api/owner/locations/:locationId/refunds). A template built by analogy with the sibling shape would MISS this route entirely — it needed its own literal template, discovered only by reading the file.',
  },
  {
    method: 'POST',
    template: '/api/owner/:locationId/refunds/:paymentId/sent',
    surface: 'S5',
    source: 'refunds.ts:43',
    flag: 'same path anomaly as above',
  },

  // -- dwell-settings.ts (prefix /api/owner/locations; 2) — S3
  { method: 'GET', template: '/api/owner/locations/:locationId/settings/dwell', surface: 'S3', source: 'dwell-settings.ts:22' },
  { method: 'PUT', template: '/api/owner/locations/:locationId/settings/dwell', surface: 'S3', source: 'dwell-settings.ts:41' },

  // -- reveal-contact.ts (prefix /api/owner/locations; 1) — S5 (PII, order-scoped)
  {
    method: 'POST',
    template: '/api/owner/locations/:locationId/orders/:orderId/reveal-customer-contact',
    surface: 'S5',
    source: 'reveal-contact.ts:15',
    flag: 'borderline S9 (PII disclosure) — assigned S5 per breaker-findings.md\'s explicit grouping ("...reveal-customer-contact → S5 ... and reveal-contact (PII)")',
  },

  // -- order-meta.ts (prefix /api/owner/locations; 1) — S5
  { method: 'PATCH', template: '/api/owner/locations/:locationId/orders/:orderId/metadata', surface: 'S5', source: 'order-meta.ts:13' },

  // -- menu-translate.ts (prefix /api/owner; 1) — S3
  { method: 'POST', template: '/api/owner/locations/:id/menu/translate', surface: 'S3', source: 'menu-translate.ts:10', flag: 'likely-dead route — no FE caller found (census grep of apps/web/src)' },

  // -- menu-confirm.ts (self-prefixed; 1) — S3
  { method: 'POST', template: '/api/owner/locations/:locationId/products/:productId/confirm-allergens', surface: 'S3', source: 'menu-confirm.ts:10', flag: 'likely-dead route (no FE caller found) AND a food-safety/liability field — escalate before dropping, per Task-Exit Rule' },

  // -- locations.ts (owner, self-prefixed; 1) — S3
  { method: 'PATCH', template: '/api/owner/locations/:locationId', surface: 'S3', source: 'owner/locations.ts:9', flag: 'money-adjacent (tax_rate/delivery_fee_flat feed the order-total calc) but itself config, not a transaction' },

  // ============================================================================================
  // COURIER + CUSTOMER + CORE — (51) — prefix resolution verified against bootstrap/routes.ts
  // ============================================================================================

  // -- courier/assignments.ts (prefix /api/courier; 9) — S7
  { method: 'GET', template: '/api/courier/me/assignments', surface: 'S7', source: 'courier/assignments.ts:74' },
  { method: 'GET', template: '/api/courier/assignments/:id', surface: 'S7', source: 'courier/assignments.ts:102' },
  { method: 'POST', template: '/api/courier/assignments/:id/accept', surface: 'S7', source: 'courier/assignments.ts:125' },
  { method: 'POST', template: '/api/courier/assignments/:id/reject', surface: 'S7', source: 'courier/assignments.ts:178' },
  { method: 'POST', template: '/api/courier/assignments/:id/picked-up', surface: 'S7', source: 'courier/assignments.ts:239' },
  { method: 'POST', template: '/api/courier/assignments/:id/delivered', surface: 'S7', source: 'courier/assignments.ts:292', flag: 'STATE + MONEY (cash-as-proof) — money-adjacent but path-owned S7' },
  { method: 'POST', template: '/api/courier/assignments/:id/cancel', surface: 'S7', source: 'courier/assignments.ts:413' },
  { method: 'POST', template: '/api/courier/assignments/:id/abort', surface: 'S7', source: 'courier/assignments.ts:482' },
  { method: 'POST', template: '/api/courier/assignments/:id/decline', surface: 'S7', source: 'courier/assignments.ts:535' },

  // -- courier/me.ts (prefix /api/courier; 6) — S7
  { method: 'GET', template: '/api/courier/me', surface: 'S7', source: 'courier/me.ts:36' },
  { method: 'PATCH', template: '/api/courier/me/messenger', surface: 'S7', source: 'courier/me.ts:76' },
  { method: 'GET', template: '/api/courier/me/audit-log', surface: 'S7', source: 'courier/me.ts:94' },
  { method: 'PATCH', template: '/api/courier/me/password', surface: 'S7', source: 'courier/me.ts:110', flag: 'AUTH-class op (password change + full session revoke) but path-owned S7 — see courier/auth.ts note below on the same cross-surface pattern' },
  { method: 'GET', template: '/api/courier/me/earnings', surface: 'S7', source: 'courier/me.ts:177', flag: 'MONEY-adjacent (payout figures) but path-owned S7 — see courier/settlements.ts note (money is not a single atomic surface)' },
  { method: 'GET', template: '/api/courier/me/history', surface: 'S7', source: 'courier/me.ts:249' },

  // -- courier/shifts.ts (prefix /api/courier; 5) — S7
  { method: 'GET', template: '/api/courier/me/shift', surface: 'S7', source: 'courier/shifts.ts:15' },
  { method: 'POST', template: '/api/courier/me/shift/start', surface: 'S7', source: 'courier/shifts.ts:60' },
  { method: 'POST', template: '/api/courier/me/shift/end', surface: 'S7', source: 'courier/shifts.ts:111' },
  { method: 'POST', template: '/api/courier/shifts/transition', surface: 'S7', source: 'courier/shifts.ts:173' },
  { method: 'POST', template: '/api/courier/shifts/ping', surface: 'S7', source: 'courier/shifts.ts:305' },

  // -- courier/auth.ts (prefix /api/courier/auth; 5) — S7 by path, S2-class obligations
  {
    method: 'POST',
    template: '/api/courier/auth/invites/:inviteId/redeem',
    surface: 'S7',
    source: 'courier/auth.ts:23',
    flag: 'SURFACE-OWNERSHIP SURPRISE: mints a courier JWT (an S2-class auth operation) but is path-owned by S7 (falls under /api/courier/*). Carries the SAME cross-stack JWT-verification-parity obligation as S2 (REV-C4 body-kid round-trip) yet is NOT covered by S2\'s cutover DoD gate. Recommend: S7\'s cutover DoD explicitly inherits S2\'s JWT-parity gate before S7 flips, or these 5 routes get pulled into S2\'s gate scope regardless of path.',
  },
  { method: 'GET', template: '/api/courier/auth/invites/:inviteId', surface: 'S7', source: 'courier/auth.ts:159' },
  { method: 'POST', template: '/api/courier/auth/login', surface: 'S7', source: 'courier/auth.ts:219', flag: 'same S2-class-obligation note as invites/redeem' },
  { method: 'POST', template: '/api/courier/auth/refresh', surface: 'S7', source: 'courier/auth.ts:354', flag: 'same S2-class-obligation note' },
  { method: 'POST', template: '/api/courier/auth/logout', surface: 'S7', source: 'courier/auth.ts:479' },

  // -- courier/settlements.ts (prefix /api/courier; 2) — S7, MONEY split across S5/S7
  {
    method: 'GET',
    template: '/api/courier/me/payouts',
    surface: 'S7',
    source: 'courier/settlements.ts:12',
    flag: 'SURFACE-OWNERSHIP SURPRISE: money (payout reads, 🔴 MONEY-tagged in the census) is NOT a single atomic surface — the OWNER side of the identical settlement/payout entity (owner/settlements.ts) is S5, while the COURIER-side read of the same money data is path-owned S7. A single "money flips atomically" mental model does not hold across roles.',
  },
  { method: 'GET', template: '/api/courier/me/payouts/:id', surface: 'S7', source: 'courier/settlements.ts:51', flag: 'same money-split note' },

  // -- customer/orders.ts (prefix /api/customer; 3) — S5
  { method: 'GET', template: '/api/customer/orders/:orderId/status', surface: 'S5', source: 'customer/orders.ts:21' },
  { method: 'POST', template: '/api/customer/orders/:orderId/rating', surface: 'S5', source: 'customer/orders.ts:219' },
  { method: 'POST', template: '/api/customer/orders/:orderId/cancel', surface: 'S5', source: 'customer/orders.ts:259' },

  // -- customer/push.ts (prefix /api/customer; 2) — S8
  { method: 'POST', template: '/api/customer/push/subscribe', surface: 'S8', source: 'customer/push.ts:21' },
  { method: 'POST', template: '/api/customer/push/unsubscribe', surface: 'S8', source: 'customer/push.ts:64' },

  // -- customer/otp.ts (prefix /api/customer; 2) — S2. REAL path (fixes the CRIT-2 phantom "/api/customer/otp/*")
  { method: 'POST', template: '/api/customer/locations/:slug/otp/send', surface: 'S2', source: 'customer/otp.ts:34', flag: 'CRIT-2 correction: proposal.md §4 claimed "POST /api/customer/otp/*"; the real registered path is /api/customer/locations/:slug/otp/send (prefix /api/customer + in-file /locations/:slug/otp/send)' },
  { method: 'POST', template: '/api/customer/locations/:slug/otp/verify', surface: 'S2', source: 'customer/otp.ts:112', flag: 'same CRIT-2 correction' },

  // -- customer/track.ts (prefix /api/customer; 1) — S2
  { method: 'POST', template: '/api/customer/track/exchange', surface: 'S2', source: 'customer/track.ts:28' },

  // -- auth.ts (prefix /api; 8) — S2
  { method: 'GET', template: '/api/auth/google', surface: 'S2', source: 'auth.ts:34' },
  { method: 'GET', template: '/api/auth/google/callback', surface: 'S2', source: 'auth.ts:62' },
  { method: 'POST', template: '/api/auth/exchange', surface: 'S2', source: 'auth.ts:173' },
  { method: 'POST', template: '/api/auth/telegram/start', surface: 'S2', source: 'auth.ts:191' },
  { method: 'GET', template: '/api/auth/telegram/poll', surface: 'S2', source: 'auth.ts:202' },
  { method: 'POST', template: '/api/auth/refresh', surface: 'S2', source: 'auth.ts:235' },
  { method: 'POST', template: '/api/auth/logout', surface: 'S2', source: 'auth.ts:325' },
  { method: 'POST', template: '/api/auth/courier/activate', surface: 'S2', source: 'auth.ts:339' },

  // -- orders.ts (core, prefix /api; 3) — S5. THE CRIT-2 crown-jewel fix.
  { method: 'POST', template: '/api/orders', surface: 'S5', source: 'orders.ts:73', flag: 'CRIT-2 correction: proposal.md §4 claimed "POST /orders" (phantom — verified live-grep, real registration is fastify.post(\'/orders\',...) at orders.ts:73 mounted under prefix "/api")' },
  { method: 'GET', template: '/api/orders/:id', surface: 'S5', source: 'orders.ts:735' },
  { method: 'PATCH', template: '/api/orders/:id/status', surface: 'S5', source: 'orders.ts:864' },

  // -- order-messages.ts (no prefix option, self-embeds "/api/orders/..."; 3) — S5
  { method: 'POST', template: '/api/orders/:orderId/messages', surface: 'S5', source: 'order-messages.ts:32', flag: 'borderline S5/S8 (order-lifecycle messaging) — assigned S5 by path-ownership: proposal.md §4\'s S5 row explicitly claims the whole /api/orders/* namespace' },
  { method: 'GET', template: '/api/orders/:orderId/messages', surface: 'S5', source: 'order-messages.ts:124' },
  { method: 'POST', template: '/api/orders/:orderId/messages/read', surface: 'S5', source: 'order-messages.ts:161' },

  // -- couriers.ts (core, registered BARE, no prefix option; 1) — S7
  {
    method: 'POST',
    template: '/couriers/invites',
    surface: 'S7',
    source: 'couriers.ts:8',
    flag: 'MAJOR SURPRISE: this path does NOT start with /api at all. Any path-ownership map keyed on "/api/..." prefixes (as the original phantom map effectively assumed) would silently MISS this route entirely — same failure class as CRIT-2\'s phantom paths, found independently here. Census flags it as likely-orphaned (no FE caller found); the courier-invite UI exclusively calls the DIFFERENT route /api/owner/locations/:locationId/courier-invites.',
  },

  // -- auth/local.ts (prefix /api; 1) — S2
  { method: 'POST', template: '/api/auth/local/login', surface: 'S2', source: 'auth/local.ts:36' },

  // ============================================================================================
  // PUBLIC + ADMIN + DEV + WEBHOOK + SPA-PROXY + INFRA — (72)
  // ============================================================================================

  // -- spa-proxy.ts (18)
  { method: 'GET', template: '/images/*', surface: 'S1', source: 'spa-proxy.ts:158', flag: 'explicit in proposal.md §4\'s S1 row despite being served by the legacy-named spa-proxy.ts file' },
  { method: 'GET', template: '/media/*', surface: 'S1', source: 'spa-proxy.ts:184' },
  {
    method: 'POST',
    template: '/api/owner/menu/products/:productId/image',
    surface: 'S4',
    source: 'spa-proxy.ts:213',
    flag: 'DUPLICATE-IMPLEMENTATION hazard: a second, independent image-upload path (sharp-resize, single-shot) alongside product-media.ts\'s presign/confirm flow (same surface S4, but two maintained code paths for "set a product image" — a code-duplication risk INSIDE one surface, not just across surfaces).',
  },
  { method: 'POST', template: '/api/public/entry-photo', surface: 'S4', source: 'spa-proxy.ts:268' },
  { method: 'GET', template: '/api/owner/analytics', surface: 'UNMAPPED', source: 'spa-proxy.ts:296', flag: 'breaker-findings.md MEDIUM: reads the S5 orders/order_items money tables but is not itself S5-owned by any proposal.md row — "S5\'s whole family flips atomically" is false while this stays on Node. Needs an explicit S5 sub-scope decision or a permanent-Node carve-out, not a silent default.' },
  { method: 'GET', template: '/api/owner/analytics/product-orders', surface: 'UNMAPPED', source: 'spa-proxy.ts:375', flag: 'same gap as /api/owner/analytics' },
  { method: 'GET', template: '/api/owner/orders', surface: 'S5', source: 'spa-proxy.ts:393', flag: 'the ONE spa-proxy.ts row that DOES match the phantom map\'s literal "GET|PATCH /api/owner/orders/*" claim — but note there is no PATCH at this path; the owner order-ACTION routes (confirm/reject/deliver/etc.) live at a completely different path (dashboard.ts, /api/owner/locations/:locationId/orders/:orderId/*), so the phantom map\'s "/api/owner/orders/*" pattern was still wrong for everything except this one GET.' },
  { method: 'GET', template: '/api/owner/couriers', surface: 'S7', source: 'spa-proxy.ts:452' },
  { method: 'GET', template: '/api/public/theme/:slug', surface: 'S1', source: 'spa-proxy.ts:506' },
  { method: 'GET', template: '/api/owner/brand', surface: 'S3', source: 'spa-proxy.ts:528' },
  { method: 'PUT', template: '/api/owner/brand', surface: 'S3', source: 'spa-proxy.ts:562' },
  { method: 'POST', template: '/api/owner/brand/generate', surface: 'S3', source: 'spa-proxy.ts:616' },
  { method: 'GET', template: '/api/owner/settings', surface: 'S3', source: 'spa-proxy.ts:667' },
  { method: 'PUT', template: '/api/owner/settings', surface: 'S3', source: 'spa-proxy.ts:701' },
  {
    method: 'POST',
    template: '/api/owner/courier-invites',
    surface: 'S7',
    source: 'spa-proxy.ts:742',
    flag: 'NAMING/PATH COLLISION hazard: a second, differently-shaped courier-invite-mint endpoint (flat /api/owner/courier-invites) coexists with owner/courier-invites.ts\'s /api/owner/locations/:locationId/courier-invites — same surface S7, two independent implementations, real maintenance/security-parity hazard (a fix to one can miss the other).',
  },
  {
    method: 'POST',
    template: '/api/owner/onboarding',
    surface: 'S10',
    source: 'spa-proxy.ts:758',
    flag: 'NAMING/PATH COLLISION with onboarding.ts\'s /api/owner/onboarding/start family. ALSO an untracked cross-stack two-writer on products/locations/location_themes per breaker-findings.md MEDIUM finding — writes tables S3-Rust is supposed to own exclusively.',
  },
  { method: 'GET', template: '/api/owner/customers', surface: 'UNMAPPED', source: 'spa-proxy.ts:838', flag: 'CRM — no clean fit in S1..S10; breaker-findings.md MEDIUM names this exact route as an "invisible un-migratable route"' },
  { method: 'GET', template: '/api/owner/customers/:id/analytics', surface: 'UNMAPPED', source: 'spa-proxy.ts:856', flag: 'same gap as /api/owner/customers' },

  // -- public/*.ts (25) — S1 storefront-read, except the anonymous-write outliers (flagged)
  { method: 'GET', template: '/s/:slug/cart', surface: 'S1', source: 'client-flow.ts:15' },
  { method: 'GET', template: '/s/:slug/checkout', surface: 'S1', source: 'client-flow.ts:16' },
  { method: 'GET', template: '/s/:slug/order/:id', surface: 'S1', source: 'client-flow.ts:17' },
  { method: 'GET', template: '/s/:slug/orders/:orderId', surface: 'S1', source: 'client-flow.ts:18', flag: 'legacy alias — keep as an alias/redirect in the Rust port, not a duplicate handler' },
  { method: 'GET', template: '/robots.txt', surface: 'S1', source: 'seo.ts:45' },
  { method: 'GET', template: '/sitemap.xml', surface: 'S1', source: 'seo.ts:90' },
  { method: 'GET', template: '/sitemap-locations-:shard.xml', surface: 'S1', source: 'seo.ts:121' },
  { method: 'GET', template: '/public/locations/:locationIdOrSlug/menu', surface: 'S1', source: 'menu.ts:231' },
  { method: 'GET', template: '/public/locations/:slug/info', surface: 'S1', source: 'menu.ts:312' },
  { method: 'GET', template: '/public/locations/:slug/products/:productId/media', surface: 'S1', source: 'menu.ts:418' },
  { method: 'POST', template: '/api/claim/accept', surface: 'S10', source: 'claim.ts:17', flag: 'borderline S2 (mints reauth via verifyAuth) / S10 (the public-facing half of the acquisition/provisioning pipeline) — assigned S10' },
  { method: 'POST', template: '/api/claim/request', surface: 'S10', source: 'claim.ts:49' },
  { method: 'POST', template: '/api/claim/decline', surface: 'S10', source: 'claim.ts:69' },
  { method: 'POST', template: '/api/telemetry', surface: 'UNMAPPED', source: 'telemetry.ts:37', flag: 'breaker-findings.md MEDIUM-named gap. Also structurally NOT read-only, so force-fitting it into S1 would break the proven "S1 = zero writes" invariant (breaker: "Confirmed sound: S1 read-only holds").' },
  { method: 'POST', template: '/api/telemetry/abuse', surface: 'UNMAPPED', source: 'telemetry.ts:84', flag: 'same as /api/telemetry' },
  { method: 'GET', template: '/api/public/voice-config', surface: 'S1', source: 'voice-config.ts:11' },
  { method: 'GET', template: '/api/push/vapid-public-key', surface: 'S1', source: 'vapid.ts:5' },
  { method: 'GET', template: '/public/locations/:locationId/theme.css', surface: 'S1', source: 'theme.ts:10' },
  { method: 'GET', template: '/s/:slug', surface: 'S1', source: 'ssr.ts:18' },
  { method: 'GET', template: '/v1/rates', surface: 'S1', source: 'rates.ts:14' },
  { method: 'GET', template: '/s/:slug/manifest.webmanifest', surface: 'S1', source: 'pwa.ts:7' },
  { method: 'POST', template: '/api/funnel', surface: 'UNMAPPED', source: 'funnel.ts:34', flag: 'breaker-findings.md MEDIUM-named gap; same not-read-only issue as /api/telemetry' },
  { method: 'GET', template: '/api/public/locations/:slug/fallback-config', surface: 'S1', source: 'fallback-config.ts:9' },
  { method: 'GET', template: '/branding-preview/:slug', surface: 'S1', source: 'branding-preview.ts:6' },
  { method: 'POST', template: '/api/access-requests', surface: 'UNMAPPED', source: 'access-requests.ts:58', flag: 'public PII-capture write, flag-gated at REGISTRATION time (only mounted when ACCESS_GATE_PUBLIC_ENABLED=true) — no clean S1..S10 fit' },

  // -- dev/mock-auth.ts (6) — dev/test-only, 🔴 must never ship enabled
  { method: 'POST', template: '/dev/mock-auth', surface: 'S2', source: 'mock-auth.ts:14', flag: 'test-only token minter, dark behind ALLOW_DEV_LOGIN+DEV_AUTH_SECRET — RECOMMEND EXCLUDING all /dev/* and /api/dev/* routes from the cutover harness\'s scope entirely (never a production surface to flip), rather than folding them into S2' },
  { method: 'POST', template: '/dev/create-assignment', surface: 'UNMAPPED', source: 'mock-auth.ts:122', flag: 'test-only DB-seeding, not an auth operation — recommend excluding from harness scope' },
  { method: 'POST', template: '/dev/seed-telegram-target', surface: 'UNMAPPED', source: 'mock-auth.ts:184', flag: 'test-only — recommend excluding from harness scope' },
  { method: 'POST', template: '/dev/repair-test-owner', surface: 'UNMAPPED', source: 'mock-auth.ts:204', flag: 'test-only — recommend excluding from harness scope' },
  { method: 'POST', template: '/dev/seed-visual-state', surface: 'UNMAPPED', source: 'mock-auth.ts:583', flag: 'test-only — recommend excluding from harness scope' },
  { method: 'POST', template: '/api/dev/seed-visual-state', surface: 'UNMAPPED', source: 'mock-auth.ts:584', flag: 'alias of /dev/seed-visual-state (same handler, two paths) — collapse to one in the Rust port; test-only, exclude from harness scope' },

  // -- admin/*.ts (6) — S10, platform-admin plane (ADR-admin-platform-authz / B4)
  { method: 'GET', template: '/api/admin/backups', surface: 'S10', source: 'admin/backups.ts:13' },
  { method: 'POST', template: '/api/admin/backups/verify', surface: 'S10', source: 'admin/backups.ts:73' },
  { method: 'GET', template: '/api/admin/backups/dr-report', surface: 'S10', source: 'admin/backups.ts:100' },
  { method: 'GET', template: '/api/admin/fallback/health', surface: 'S10', source: 'admin/fallback.ts:13' },
  { method: 'POST', template: '/api/admin/fallback/r2-check', surface: 'S10', source: 'admin/fallback.ts:47' },
  { method: 'GET', template: '/api/admin/notification-audit', surface: 'S10', source: 'admin/notification-audit.ts:17' },

  // -- modules/acquisition/route.ts (prefix /internal; 9) — S10, ANOTHER non-/api path family
  {
    method: 'POST',
    template: '/internal/acquisition',
    surface: 'S10',
    source: 'acquisition/route.ts:62',
    flag: 'SURFACE-OWNERSHIP SURPRISE: S10 (platform-admin) spans BOTH /api/admin/* AND /internal/* — two entirely different top-level path families own the same surface. "Surface = one path prefix" breaks even for S10, not only for the owner/locations family CRIT-1 targets.',
  },
  { method: 'POST', template: '/internal/acquisition/extract', surface: 'S10', source: 'acquisition/route.ts:77' },
  { method: 'POST', template: '/internal/acquisition/provision/mint', surface: 'S10', source: 'acquisition/route.ts:90' },
  { method: 'POST', template: '/internal/acquisition/provision/spine', surface: 'S10', source: 'acquisition/route.ts:107' },
  { method: 'POST', template: '/internal/acquisition/provision/hard-delete', surface: 'S10', source: 'acquisition/route.ts:130' },
  { method: 'POST', template: '/internal/acquisition/claim/verify', surface: 'S10', source: 'acquisition/route.ts:142' },
  { method: 'POST', template: '/internal/acquisition/claim/mint', surface: 'S10', source: 'acquisition/route.ts:159' },
  { method: 'POST', template: '/internal/acquisition/complaint', surface: 'S10', source: 'acquisition/route.ts:186' },
  { method: 'POST', template: '/internal/acquisition/retention/sweep', surface: 'S10', source: 'acquisition/route.ts:199' },

  // -- server.ts inline dev routes (3) — dev/test-only, duplicates of mock-auth.ts
  { method: 'POST', template: '/api/dev/mock-auth', surface: 'S2', source: 'server.ts:549', flag: 'DUPLICATE-IMPLEMENTATION of /dev/mock-auth (mock-auth.ts:14), plus a `fresh:true` mode the twin lacks — same "exclude from harness scope" recommendation' },
  { method: 'POST', template: '/api/dev/create-assignment', surface: 'UNMAPPED', source: 'server.ts:653', flag: 'DUPLICATE-IMPLEMENTATION of /dev/create-assignment (mock-auth.ts:122) — test-only, exclude from harness scope' },
  { method: 'POST', template: '/api/dev/seed-data', surface: 'UNMAPPED', source: 'server.ts:701', flag: 'test-only, no twin — exclude from harness scope' },

  // -- health.ts (2) — deliberately never routed through the flip mechanism
  { method: 'GET', template: '/livez', surface: 'INFRA_NEVER_FLIPS', source: 'health.ts:61', flag: 'liveness probe MUST stay a zero-dependency handler on whichever process is actually live — never a flippable business surface' },
  { method: 'GET', template: '/health', surface: 'INFRA_NEVER_FLIPS', source: 'health.ts:65' },

  // -- telegram-webhook.ts (1) — S8
  { method: 'POST', template: '/webhook/telegram/:secret', surface: 'S8', source: 'telegram-webhook.ts:36' },

  // -- payments-webhook.ts (1) — S5, money source-of-truth
  { method: 'POST', template: '/webhook/payments/plisio', surface: 'S5', source: 'payments-webhook.ts:13' },

  // -- lib/metrics.ts (1) — infra
  { method: 'GET', template: '/metrics', surface: 'INFRA_NEVER_FLIPS', source: 'metrics.ts:134' },
];
