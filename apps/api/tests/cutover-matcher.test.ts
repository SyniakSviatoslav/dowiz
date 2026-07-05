/**
 * Unit tests for the REV-C2 cutover matcher (breaker CRIT-1).
 *
 * Run: `npx tsx docs/design/rebuild-cutover-harness/matcher/cutover-matcher.test.ts`
 * (uses Node's built-in test runner + assert — zero extra dependencies, consistent
 * with the matcher itself being dependency-free).
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { matchSurface, matchSurfaceForRequest, isWebSocketUpgrade } from '../src/lib/cutover/matcher.js';
import { ROUTE_TEMPLATES } from '../src/lib/cutover/route-templates.generated.js';

const UUID_A = '11111111-1111-1111-1111-111111111111';
const UUID_B = '22222222-2222-2222-2222-222222222222';

describe('census completeness (the machine-derived partition, not a hand-authored one)', () => {
  test('total route-template count matches the REBUILD-MAP §1 census (236 registered HTTP routes)', () => {
    assert.equal(ROUTE_TEMPLATES.length, 236);
  });

  test('every template has a real file:line source (never a bare guess)', () => {
    for (const t of ROUTE_TEMPLATES) {
      assert.match(t.source, /^[\w./-]+\.ts:\d+$/, `template ${t.method} ${t.template} is missing a file:line source`);
    }
  });

  test('no two templates are byte-identical (method+template) — the census has no accidental duplicate rows', () => {
    const seen = new Map<string, string>();
    for (const t of ROUTE_TEMPLATES) {
      const key = `${t.method} ${t.template}`;
      const prior = seen.get(key);
      assert.equal(prior, undefined, `duplicate template row: "${key}" appears at both ${prior} and ${t.source}`);
      seen.set(key, t.source);
    }
  });
});

describe('disjointness proof (the actual CRIT-1 claim, checked mechanically, not asserted in prose)', () => {
  test('every template is pairwise non-colliding against every other template', () => {
    // For each template, synthesize ONE concrete example path (substitute a fixed UUID
    // for every :param, drop the trailing wildcard to one literal segment) and assert
    // that path matches EXACTLY that one template among the whole set — i.e. no other
    // template in the census can also match it. This is the disjointness proof made
    // executable instead of merely claimed.
    for (const t of ROUTE_TEMPLATES) {
      const examplePath = t.template
        .split('/')
        .map((seg) => {
          if (seg === '*') return 'example-file.webp';
          if (seg.startsWith(':')) return UUID_A;
          return seg;
        })
        .join('/');

      const result = matchSurface(t.method, examplePath, ROUTE_TEMPLATES);
      assert.equal(
        result.matched,
        true,
        `template ${t.method} ${t.template} (${t.source}) did not match its own synthesized example "${examplePath}": ${result.reason}`,
      );
      assert.equal(
        result.template?.source,
        t.source,
        `example path for ${t.method} ${t.template} (${t.source}) ambiguously matched a DIFFERENT template (${result.template?.source}) — collision detected`,
      );
    }
  });
});

describe('CRIT-1: the five owner-location sub-surfaces resolve distinctly (never co-flip)', () => {
  test('S3 catalog (theme) resolves to S3', () => {
    const r = matchSurface('PUT', `/api/owner/locations/${UUID_A}/theme`, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S3');
  });

  test('S5 money (deliver) resolves to S5', () => {
    const r = matchSurface('POST', `/api/owner/locations/${UUID_A}/orders/${UUID_B}/deliver`, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S5');
  });

  test('S7 courier/dispatch (courier-invites) resolves to S7', () => {
    const r = matchSurface('POST', `/api/owner/locations/${UUID_A}/courier-invites`, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S7');
  });

  test('S8 jobs/notifications (alerts) resolves to S8', () => {
    const r = matchSurface('GET', `/api/owner/locations/${UUID_A}/alerts`, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S8');
  });

  test('S9 GDPR/erase (gdpr-requests) resolves to S9', () => {
    const r = matchSurface('POST', `/api/owner/locations/${UUID_A}/gdpr-requests`, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S9');
  });

  test('THE headline invariant: money (S5) and GDPR-erase (S9) resolve to DIFFERENT surfaces despite sharing the exact same prefix up to the UUID', () => {
    const money = matchSurface('POST', `/api/owner/locations/${UUID_A}/orders/${UUID_B}/deliver`, ROUTE_TEMPLATES);
    const erase = matchSurface('POST', `/api/owner/locations/${UUID_A}/gdpr-requests`, ROUTE_TEMPLATES);
    assert.equal(money.surface, 'S5');
    assert.equal(erase.surface, 'S9');
    assert.notEqual(money.surface, erase.surface, 'flipping S5 must never co-flip S9 — the exact breaker CRIT-1 scenario');
  });

  test('the textbook infix case: .../orders/:orderId/route (S7 dispatch) vs .../orders/:orderId/deliver (S5 money) — same prefix, same segment count, different trailing literal', () => {
    const route = matchSurface('GET', `/api/owner/locations/${UUID_A}/orders/${UUID_B}/route`, ROUTE_TEMPLATES);
    const deliver = matchSurface('POST', `/api/owner/locations/${UUID_A}/orders/${UUID_B}/deliver`, ROUTE_TEMPLATES);
    assert.equal(route.surface, 'S7');
    assert.equal(deliver.surface, 'S5');
  });
});

describe('fail-closed behavior (a request matching no template never guesses)', () => {
  test('an unmapped/unknown path under the shared owner-location prefix fails closed to Node', () => {
    const r = matchSurface('GET', `/api/owner/locations/${UUID_A}/totally-unknown-thing`, ROUTE_TEMPLATES);
    assert.equal(r.matched, false);
    assert.equal(r.surface, 'NODE_UNMAPPED');
  });

  test('a registered-but-taxonomy-gap route (owner analytics) is explicitly UNMAPPED, not silently forced into a surface', () => {
    const r = matchSurface('GET', '/api/owner/analytics', ROUTE_TEMPLATES);
    assert.equal(r.matched, true);
    assert.equal(r.surface, 'UNMAPPED');
  });

  test('an infra route (health) is INFRA_NEVER_FLIPS, distinct from a genuine taxonomy gap', () => {
    const r = matchSurface('GET', '/health', ROUTE_TEMPLATES);
    assert.equal(r.surface, 'INFRA_NEVER_FLIPS');
  });

  test('a path that resembles a real template but with an extra segment does not match', () => {
    const r = matchSurface('POST', `/api/owner/locations/${UUID_A}/gdpr-requests/extra/segment`, ROUTE_TEMPLATES);
    assert.equal(r.matched, false);
  });
});

describe('method-sensitivity (the same path under a different verb is a different — or no — route)', () => {
  test('POST /api/orders matches S5; GET /api/orders (no such registration) does not match at all', () => {
    const post = matchSurface('POST', '/api/orders', ROUTE_TEMPLATES);
    const get = matchSurface('GET', '/api/orders', ROUTE_TEMPLATES);
    assert.equal(post.surface, 'S5');
    assert.equal(get.matched, false, 'there is no GET /api/orders registration — the matcher must not match on path alone');
  });

  test('DELETE on the GET-only .../orders/:orderId/route template does not match', () => {
    const r = matchSurface('DELETE', `/api/owner/locations/${UUID_A}/orders/${UUID_B}/route`, ROUTE_TEMPLATES);
    assert.equal(r.matched, false);
  });
});

describe('param segments never leak across surfaces (a literal segment must actually be literal)', () => {
  test('a :requestId value that reads like "orders" does not get mistaken for the literal /orders/ segment', () => {
    // gdpr.ts's template is /api/owner/locations/:locationId/gdpr-requests/:requestId (S9).
    // Even if :requestId's VALUE were the string "orders", the route stays S9 because segment
    // position 4 is the literal "gdpr-requests", not "orders" — position, not substring content, decides.
    const r = matchSurface('GET', `/api/owner/locations/${UUID_A}/gdpr-requests/orders`, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S9');
  });

  test('a :locationId value that reads like a literal route segment ("gdpr-requests") does not redirect the match', () => {
    // /api/owner/locations/:locationId/theme (S3) — :locationId is positional segment 3;
    // giving it the literal value "gdpr-requests" must NOT make this resolve as S9.
    const r = matchSurface('GET', '/api/owner/locations/gdpr-requests/theme', ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S3');
  });

  test('the refunds.ts path anomaly (missing "locations/" segment) resolves distinctly from the sibling shape', () => {
    const anomaly = matchSurface('GET', `/api/owner/${UUID_A}/refunds`, ROUTE_TEMPLATES);
    const wouldBeSibling = matchSurface('GET', `/api/owner/locations/${UUID_A}/refunds`, ROUTE_TEMPLATES);
    assert.equal(anomaly.surface, 'S5');
    assert.equal(wouldBeSibling.matched, false, 'the (nonexistent) "correct-shaped" refunds path must not accidentally match anything either');
  });
});

describe('non-/api-prefixed routes are still handled (no implicit "/api" assumption)', () => {
  test('/couriers/invites (no /api prefix at all) still resolves', () => {
    const r = matchSurface('POST', '/couriers/invites', ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S7');
  });

  test('/internal/acquisition/* (S10 under a completely different top-level prefix than /api/admin/*) resolves', () => {
    const r = matchSurface('POST', '/internal/acquisition/provision/mint', ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S10');
  });
});

describe('S6 (WebSocket) — protocol-level match, not a path-template match', () => {
  test('an Upgrade: websocket request on the expected /ws path resolves to S6', () => {
    const r = matchSurfaceForRequest('GET', '/ws', { upgrade: 'websocket' }, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S6');
  });

  test('an Upgrade: websocket request on a COMPLETELY UNRELATED path ALSO resolves to S6 — proving the real ws.WebSocketServer (no `path` filter) accepts upgrades on any URL, so a "/ws"-only template would be a phantom precision', () => {
    const r = matchSurfaceForRequest('GET', '/this/is/not/a/websocket/path/at/all', { upgrade: 'websocket' }, ROUTE_TEMPLATES);
    assert.equal(r.surface, 'S6');
  });

  test('the SAME /ws path WITHOUT an Upgrade header falls through to ordinary path matching and fails closed (no such HTTP-GET template exists)', () => {
    const r = matchSurfaceForRequest('GET', '/ws', {}, ROUTE_TEMPLATES);
    assert.equal(r.matched, false);
    assert.equal(r.surface, 'NODE_UNMAPPED');
  });

  test('isWebSocketUpgrade is case-insensitive on the header value and tolerates an array header', () => {
    assert.equal(isWebSocketUpgrade({ upgrade: 'WebSocket' }), true);
    assert.equal(isWebSocketUpgrade({ upgrade: ['websocket'] }), true);
    assert.equal(isWebSocketUpgrade({ upgrade: 'h2c' }), false);
    assert.equal(isWebSocketUpgrade({}), false);
  });
});

describe('ambiguous-collision runtime backstop (defense in depth beyond the static disjointness proof)', () => {
  test('if two templates were ever made to collide, matchSurface fails closed rather than picking either', () => {
    const colliding = [
      { method: 'POST', template: '/api/owner/locations/:locationId/orders/:orderId/deliver', surface: 'S5' as const, source: 'test-fixture-a' },
      { method: 'POST', template: '/api/owner/locations/:x/orders/:y/deliver', surface: 'S9' as const, source: 'test-fixture-b' },
    ];
    const r = matchSurface('POST', `/api/owner/locations/${UUID_A}/orders/${UUID_B}/deliver`, colliding);
    assert.equal(r.matched, false);
    assert.equal(r.surface, 'NODE_UNMAPPED');
    assert.equal(r.collisions?.length, 2);
  });
});
