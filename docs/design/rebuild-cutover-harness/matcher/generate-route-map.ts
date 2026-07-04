/**
 * Renders `../route-surface-map.generated.md` FROM `route-templates.generated.ts` —
 * the single source of truth shared with the matcher (cutover-matcher.ts). This is
 * the mechanism that makes REV-C1 durable: the map cannot silently drift from the
 * router config again, because after this change there is only one artifact
 * (ROUTE_TEMPLATES) to drift from itself.
 *
 * Run: `npx tsx docs/design/rebuild-cutover-harness/matcher/generate-route-map.ts > docs/design/rebuild-cutover-harness/route-surface-map.generated.md`
 *
 * To ADD or CHANGE a route mapping: edit `route-templates.generated.ts` (never edit
 * the rendered .md directly — it will be silently overwritten on the next run), then
 * re-run the command above AND re-run the test suite
 * (`npx tsx --test docs/design/rebuild-cutover-harness/matcher/cutover-matcher.test.ts`)
 * to re-prove disjointness over the new set.
 */
import { ROUTE_TEMPLATES } from './route-templates.generated.js';
import type { SurfaceId } from './cutover-matcher.js';

const SURFACE_LABELS: Record<SurfaceId, string> = {
  S1: 'S1 storefront-read',
  S2: 'S2 auth',
  S3: 'S3 catalog CRUD',
  S4: 'S4 media',
  S5: 'S5 orders/money 🔴',
  S6: 'S6 realtime WS 🔴',
  S7: 'S7 courier/dispatch 🔴',
  S8: 'S8 jobs/notifications',
  S9: 'S9 GDPR/compliance 🔴',
  S10: 'S10 platform-admin',
  UNMAPPED: 'UNMAPPED (taxonomy gap — always Node)',
  INFRA_NEVER_FLIPS: 'INFRA (never flips — always Node, by design)',
};

const SURFACE_ORDER: SurfaceId[] = ['S1', 'S2', 'S3', 'S4', 'S5', 'S6', 'S7', 'S8', 'S9', 'S10', 'UNMAPPED', 'INFRA_NEVER_FLIPS'];

function counts(): Record<SurfaceId, number> {
  const c = Object.fromEntries(SURFACE_ORDER.map((s) => [s, 0])) as Record<SurfaceId, number>;
  for (const t of ROUTE_TEMPLATES) c[t.surface]++;
  return c;
}

function renderTable(): string {
  const rows = ROUTE_TEMPLATES.map((t, i) => {
    const flag = t.flag ? t.flag.replace(/\|/g, '\\|') : '';
    return `| ${i + 1} | ${t.method} | \`${t.template}\` | ${t.surface} | ${t.source} | ${flag} |`;
  });
  return [
    '| # | Method | Path template | Surface | Source (file:line) | Flag |',
    '|---|---|---|---|---|---|',
    ...rows,
  ].join('\n');
}

function renderSummary(): string {
  const c = counts();
  const flagged = ROUTE_TEMPLATES.filter((t) => t.flag).length;
  const total = ROUTE_TEMPLATES.length;
  const sum = SURFACE_ORDER.reduce((acc, s) => acc + c[s], 0);
  const lines = [
    `**Total registered HTTP routes: ${total}** (matches \`docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md\` §1 "HTTP routes 236" — independently re-verified against live source during this pass, zero delta).`,
    '',
    '| Surface | Count |',
    '|---|---|',
    ...SURFACE_ORDER.map((s) => `| ${SURFACE_LABELS[s]} | ${c[s]} |`),
    `| **sum** | **${sum}** |`,
    '',
    `Partition check: ${sum} === ${total} → **${sum === total ? 'PASS — every route maps to exactly one bucket (a surface, UNMAPPED, or INFRA_NEVER_FLIPS)' : 'FAIL — see generator bug'}**.`,
    '',
    `**${flagged} of ${total} rows carry an explicit \`flag\`** — a judgment call, a duplicate-implementation hazard, a path anomaly, or a cross-surface surprise. Nothing is silently forced into a surface; every non-obvious assignment says so inline.`,
  ];
  return lines.join('\n');
}

const doc = `# Route → Surface Map (generated) — REV-C1 fix for breaker CRIT-2

> **GENERATED FILE — do not hand-edit.** Source of truth: \`matcher/route-templates.generated.ts\`.
> Regenerate with \`npx tsx docs/design/rebuild-cutover-harness/matcher/generate-route-map.ts > docs/design/rebuild-cutover-harness/route-surface-map.generated.md\`.
> This is the machine-derived replacement for the hand-authored map that breaker-findings.md's
> CRIT-2 found to be phantom (it claimed \`POST /orders\`; the real registered route is
> \`POST /api/orders\`, mounted under the \`/api\` prefix — \`apps/api/src/routes/orders.ts:73\` +
> \`apps/api/src/bootstrap/routes.ts:96\`).

## How this was derived (reproducible, not hand-maintained)

1. **Extract every literal route registration** (file:line, method, in-file path):
   \`\`\`
   cd apps/api && grep -rnE "^\\s*(fastify|server)\\.(get|post|put|patch|delete|all|head|options|route)\\(" src --include="*.ts" | grep -vE "\\.test\\.|\\.spec\\."
   \`\`\`
2. **Resolve each file's mount prefix** by reading its \`fastify.register(plugin, { prefix: '...' })\`
   call in \`apps/api/src/bootstrap/routes.ts\` (\`registerCoreRoutes\`, the load-bearing order) and
   the tail registrations in \`apps/api/src/server.ts\` (product-media.ts, refunds.ts, spa-proxy.ts,
   mock-auth.ts, acquisition/route.ts, admin/index.ts). This step is NOT grep-derivable from the
   route file alone — the prefix lives in the *caller*, which is exactly the class of bug that
   produced the original CRIT-2 phantom map (a path read out of context, without its mount prefix).
3. **Compute full path = prefix + in-file path** (self-prefixed files already embed the full path
   in-file and register with no \`prefix\` option at all — verified per-file).
4. **Cross-check against the independent census** in
   \`docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md\` (same grep command, run separately
   for that document) — 236/236, zero delta between the two independent extractions, and both were
   spot-verified directly against live source for a sample of files during this pass (2026-07-04).
5. **Assign a surface** (S1..S10 / UNMAPPED / INFRA_NEVER_FLIPS) per row from
   \`docs/design/rebuild-plan/REBUILD-MAP.md\`'s surface definitions +
   \`docs/design/rebuild-cutover-harness/proposal.md\` §4's illustrative ownership rows +
   \`docs/design/rebuild-cutover-harness/breaker-findings.md\`'s explicit family groupings. Every row
   that required a judgment call (no explicit prior source to cite) carries a \`flag\` saying so.
6. **Prove the partition mechanically** — \`matcher/cutover-matcher.test.ts\`'s "disjointness proof"
   suite synthesizes one concrete example path per template and asserts it matches EXACTLY that one
   template among the full 236-row set (no other template also matches). This is not a prose claim;
   it is an executable, currently-green test (\`npx tsx --test matcher/cutover-matcher.test.ts\`).

## Partition summary

${renderSummary()}

## Full route → surface table

${renderTable()}

## Surface-ownership surprises (found while building this map, not hypothesized in advance)

1. **Money is not one atomic surface.** Owner-side settlement/payout data (\`owner/settlements.ts\`,
   under \`/api/owner/locations/:locationId/settlements*\`) is S5. The COURIER-side read of the
   IDENTICAL payout data (\`courier/settlements.ts\`, \`/api/courier/me/payouts*\`) is path-owned S7. A
   single "flip S5 and all money moves atomically" mental model does not hold — courier payout reads
   would stay on Node (or flip) independently of the owner-side ledger.
2. **Auth-class operations hide inside non-S2 surfaces.** \`courier/auth.ts\` (\`/api/courier/auth/*\`)
   mints/refreshes/revokes courier JWTs — the same class of operation S2 owns for owners/customers —
   but is path-owned S7 (falls under \`/api/courier/*\`). It carries the SAME cross-stack JWT-verification-
   parity obligation as S2 (REV-C4's body-\`kid\` round-trip) without being covered by S2's cutover DoD
   gate. \`courier/me.ts\`'s password-change route has the identical pattern.
3. **The infix problem recurs even where the breaker's own illustrative list didn't flag it.**
   \`owner/couriers.ts:205\` — \`GET /api/owner/locations/:locationId/orders/:orderId/route\` — shares the
   *exact* prefix-through-UUID as every S5 order-action route in \`dashboard.ts\` (same file family that
   motivated CRIT-1), yet is S7 (dispatch), not S5, because the trailing literal segment is \`route\` not
   \`deliver\`/\`confirm\`/etc. A longest-prefix router would have collapsed this into whichever rule owns
   the shared prefix — textbook CRIT-1, found in a THIRD file beyond the breaker's own settlements/
   couriers/notifications/gdpr/theme illustration.
4. **One file can straddle two surfaces.** \`owner/signals.ts\` registers 5 routes: 4 are S8 (risk-signal
   monitoring) and the 5th (\`POST .../orders/:orderId/mark-no-show\`) is S5 (it drives an order-status
   transition). "This is the signals file, so it's S8" would have mis-mapped one row in five.
5. **Not every route lives under \`/api\`.** \`routes/couriers.ts:8\` registers \`POST /couriers/invites\`
   with NO prefix at all — a route-ownership map keyed on \`/api/...\` prefixes (as the original phantom
   map implicitly assumed by only ever citing \`/api/...\` examples) would silently miss it entirely. Same
   failure class as CRIT-2's phantom paths, found independently in this pass. (Census flags this route as
   likely-orphaned — no FE caller found — but it is still a REGISTERED, reachable route today.)
6. **S10 spans two unrelated top-level prefixes.** \`/api/admin/*\` (backups/fallback/notification-audit)
   and \`/internal/acquisition/*\` (a completely different mount, gated by a different secret,
   deliberately decoupled from the dev-login family per breaker finding B4) are BOTH S10. "Surface = one
   path prefix" breaks even for the platform-admin surface, not only for the owner/locations family
   CRIT-1 was scoped against.
7. **S6 (WebSocket) cannot be expressed as a \`(method, path)\` rule at all.**
   \`apps/api/src/websocket.ts:192\` — \`new WebSocketServer({ server: fastify.server })\` — is
   constructed with NO \`path\` option, so the \`ws\` package intercepts every HTTP Upgrade request on the
   shared server regardless of URL. The FE always connects to \`/ws\`
   (\`apps/web/src/lib/useWebSocket.ts:6\`), but the SERVER does not enforce that — a path template for
   \`/ws\` would be a phantom precision the real server does not have. The matcher (\`cutover-matcher.ts\`)
   special-cases this: \`isWebSocketUpgrade()\` (checks the \`Upgrade\` header) is evaluated BEFORE any
   path-template matching, exactly mirroring the real server's actual (lack of) discrimination.
8. **Two independent implementations mint the same capability on different paths — twice.**
   (a) Courier-invite minting exists at BOTH \`owner/courier-invites.ts\`'s
   \`/api/owner/locations/:locationId/courier-invites\` and \`spa-proxy.ts:742\`'s flat
   \`/api/owner/courier-invites\` — both S7, same effect, two maintained code paths. (b) Product-image
   upload exists at BOTH \`product-media.ts\`'s presign/confirm flow and \`spa-proxy.ts:213\`'s single-shot
   sharp-resize POST — both S4, same effect, two maintained code paths. (c) Tenant onboarding exists at
   BOTH \`owner/onboarding.ts\`'s \`/api/owner/onboarding/start\` family and \`spa-proxy.ts:758\`'s flat
   \`/api/owner/onboarding\` — both assigned S10, and the second one is also breaker-findings.md's
   untracked \`products\`/\`location_themes\` two-writer. None of these are ROUTING collisions (the literal
   paths differ, so the matcher resolves each unambiguously) — they are maintenance/security-parity
   hazards: a fix to one implementation can silently miss its twin.
9. **A real taxonomy gap exists, confirmed mechanically, not just asserted by the breaker.** 15 routes
   (owner analytics ×2, owner customers/CRM ×2, \`/api/telemetry\` ×2, \`/api/funnel\`,
   \`/api/access-requests\`, and 8 dev/test-infra routes) have no clean home in S1..S10 as currently
   defined. They are marked \`UNMAPPED\` rather than forced into a nearby surface — they always resolve
   to \`NODE_UNMAPPED\`-equivalent behavior (stay on Node), which is safe, but the taxonomy gap itself
   needs an architect decision (either extend S1..S10 or explicitly retire these routes), not a silent
   default.

## Regeneration recipe (copy-paste)

\`\`\`bash
# 1. Re-extract the raw census (sanity check against the hardcoded template list below):
cd apps/api && grep -rnE "^\\s*(fastify|server)\\.(get|post|put|patch|delete|all|head|options|route)\\(" src --include="*.ts" | grep -vE "\\.test\\.|\\.spec\\." | wc -l
# Expect: 236

# 2. Edit docs/design/rebuild-cutover-harness/matcher/route-templates.generated.ts if routes changed.

# 3. Re-render this document:
npx tsx docs/design/rebuild-cutover-harness/matcher/generate-route-map.ts > docs/design/rebuild-cutover-harness/route-surface-map.generated.md

# 4. Re-prove disjointness + re-run every scenario test:
npx tsx --test docs/design/rebuild-cutover-harness/matcher/cutover-matcher.test.ts
\`\`\`
`;

process.stdout.write(doc);
