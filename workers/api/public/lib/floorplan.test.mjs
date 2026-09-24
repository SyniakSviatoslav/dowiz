// `node public/lib/floorplan.test.mjs`. The floor editor's rules, no browser.

import assert from 'node:assert/strict';
import { GRID, zoneId, nextNumber, snap, clampTable, nudge, placeAt, newTable, withShape, check, toWire } from './floorplan.js';

let run = 0;
const test = (name, fn) => { fn(); run++; console.log(`  ok  ${name}`); };
const LIM = { planW: 390, planH: 446, maxSeats: 20 };
const T = (n, over = {}) => ({ n, x: 100, y: 100, w: 40, h: 40, seats: 4, shape: 'rect', ...over });

test('a zone id is a storage-key word, unique', () => {
  assert.equal(zoneId('Terasa e Madhe'), 'terasa-e-madhe');
  assert.equal(zoneId('Salla', ['salla']), 'salla-2');
  assert.equal(zoneId('Salla', ['salla', 'salla-2']), 'salla-3');
  assert.equal(zoneId('Тераса'), 'zone', 'a name with no latin letters still gets an id');
  assert.equal(zoneId('Çajtore'), 'cajtore', 'accents are folded, not dropped');
  assert.match(zoneId('A!!b__c'), /^[a-z0-9-]+$/);
});

test('the next number fills the first gap', () => {
  assert.equal(nextNumber({ tables: [] }), 1);
  assert.equal(nextNumber({ tables: [T(1), T(2), T(4)] }), 3);
  assert.equal(nextNumber(null), 1);
});

test('a nudge moves one grid step and snaps', () => {
  assert.equal(snap(97), 100);
  const t = nudge(T(1, { x: 97, y: 100 }), 1, 0, LIM);
  assert.equal(t.x, 110, 'snapped to the grid after the step');
  assert.equal(nudge(T(1), 0, -1, LIM).y, 100 - GRID);
});

test('a table cannot be pushed off the drawing', () => {
  // Left edge: the centre stops at half the width.
  assert.equal(nudge(T(1, { x: 20 }), -5, 0, LIM).x, 20);
  // Bottom-right corner.
  const t = placeAt(T(1), 999, 999, LIM);
  assert.equal(t.x, 390 - 20); assert.equal(t.y, 446 - 20);
  assert.deepEqual(check([{ id: 'a', name: 'A', tables: [t] }], LIM), [], 'a clamped table passes the hub\'s rule');
  // The twin: an unclamped one does not.
  assert.equal(check([{ id: 'a', name: 'A', tables: [T(1, { x: 385 })] }], LIM)[0].key, 'fpOffPlan');
});

test('a new table takes the next number and a free spot', () => {
  const z = { tables: [] };
  const a = newTable(z, LIM); z.tables.push(a);
  const b = newTable(z, LIM);
  assert.equal(a.n, 1); assert.equal(b.n, 2);
  assert.notDeepEqual([a.x, a.y], [b.x, b.y], 'two new tables do not stack');
  assert.deepEqual(check([{ id: 'z', name: 'Z', tables: [a, b] }], LIM), []);
});

test('a round table is as wide as it is tall', () => {
  const c = withShape(T(1, { w: 60, h: 40 }), 'circle');
  assert.equal(c.w, 60); assert.equal(c.h, 60); assert.equal(c.shape, 'circle');
  assert.equal(withShape(c, 'rect').shape, 'rect');
});

test('check names what the hub would refuse, and passes what it accepts', () => {
  const ok = [{ id: 'a', name: 'A', tables: [T(1), T(2, { x: 200 })] }, { id: 'b', name: 'B', tables: [T(1)] }];
  assert.deepEqual(check(ok, LIM), [], 'one number in two zones is two tables');
  const keys = zs => check(zs, LIM).map(e => e.key);
  assert.deepEqual(keys([{ id: 'a', name: 'A', tables: [T(1), T(1, { x: 200 })] }]), ['fpNumberTwice']);
  assert.deepEqual(keys([{ id: 'a', name: 'A', tables: [T(1, { seats: 0 })] }]), ['fpSeats']);
  assert.deepEqual(keys([{ id: 'a', name: 'A', tables: [T(1, { seats: 21 })] }]), ['fpSeats']);
  assert.deepEqual(keys([{ id: 'a', name: ' ', tables: [] }]), ['fpNoName']);
  assert.deepEqual(keys([{ id: 'a', name: 'A', tables: [] }, { id: 'a', name: 'B', tables: [] }]), ['fpZoneTwice']);
});

test('the wire carries integers and only the hub\'s fields', () => {
  const w = toWire([{ id: 'a', name: ' A ', extra: 1, tables: [T(1, { x: 100.4, sel: true })] }]);
  assert.deepEqual(w, [{ id: 'a', name: 'A', tables: [{ n: 1, x: 100, y: 100, w: 40, h: 40, seats: 4, shape: 'rect' }] }]);
});

test('clampTable leaves a table that fits exactly where it is', () => {
  assert.deepEqual(clampTable(T(3), LIM), T(3));
});

console.log(`floorplan: ${run} tests passed`);
