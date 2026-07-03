import './_env-stub.js';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { assertOwnerTargetAllowed } from '../src/lib/orderAuthz.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

// F2 coupling-fix (offer-sweep-cancel addendum): the machine now PERMITS CONFIRMED/PREPARING/READY→CANCELLED
// (SYSTEM-only). assertOwnerTargetAllowed closes owner exposure at the route layer → 403 CANCEL_NOT_PERMITTED.

test('assertOwnerTargetAllowed — owner CANCELLED from CONFIRMED/PREPARING/READY → 403 CANCEL_NOT_PERMITTED', () => {
  for (const from of ['CONFIRMED', 'PREPARING', 'READY']) {
    try {
      assertOwnerTargetAllowed(from, 'CANCELLED');
      assert.fail(`${from}→CANCELLED must be forbidden for owner`);
    } catch (e: any) {
      assert.equal(e.statusCode, 403, `${from}→CANCELLED → 403`);
      assert.equal(e.code, 'CANCEL_NOT_PERMITTED');
    }
  }
});

test('assertOwnerTargetAllowed — pre-existing owner cancels preserved (no regression)', () => {
  assert.doesNotThrow(() => assertOwnerTargetAllowed('PENDING', 'CANCELLED'), 'PENDING→CANCELLED still allowed');
  assert.doesNotThrow(() => assertOwnerTargetAllowed('IN_DELIVERY', 'CANCELLED'), 'IN_DELIVERY→CANCELLED (no-show) still allowed');
});

test('assertOwnerTargetAllowed — non-cancel owner transitions untouched', () => {
  assert.doesNotThrow(() => assertOwnerTargetAllowed('CONFIRMED', 'PREPARING'));
  assert.doesNotThrow(() => assertOwnerTargetAllowed('PREPARING', 'READY'));
  assert.doesNotThrow(() => assertOwnerTargetAllowed('READY', 'IN_DELIVERY'));
  assert.doesNotThrow(() => assertOwnerTargetAllowed('PENDING', 'CONFIRMED'));
  assert.doesNotThrow(() => assertOwnerTargetAllowed('PENDING', 'REJECTED'));
});

// Wiring proof: the owner PATCH /orders/:id/status route calls the guard on the freshly-read current status,
// before handing the transition to updateOrderStatus. (Full HTTP 403 needs a running server; this pins that
// the deterministic guard is actually invoked at the owner surface — a drift that removes it fails RED.)
test('wiring — orders.ts owner PATCH invokes assertOwnerTargetAllowed before updateOrderStatus', () => {
  const src = readFileSync(join(__dirname, '../src/routes/orders.ts'), 'utf8');
  assert.match(src, /import \{ assertOwnerTargetAllowed \} from '\.\.\/lib\/orderAuthz\.js'/, 'guard imported');
  assert.match(src, /assertOwnerTargetAllowed\(cur\.rows\[0\]\.status, newStatus\)/, 'guard called on current status');
  const callIdx = src.indexOf('assertOwnerTargetAllowed(cur.rows[0].status, newStatus)');
  const mutIdx = src.indexOf('await updateOrderStatus(client, id, locationId, newStatus');
  assert.ok(callIdx > 0 && mutIdx > callIdx, 'guard precedes the updateOrderStatus call');
});
