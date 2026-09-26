// node --test workers/api/public/admin/wire-logic.test.mjs
// Every refusal has a positive twin; money never passes through a float.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as W from './wire-logic.js';

test('parseMinor reads lek and euro as integer minor units', () => {
  assert.equal(W.parseMinor('1500', 0), 1500);
  assert.equal(W.parseMinor('1 500', 0), 1500);
  assert.equal(W.parseMinor('1.500', 0), 1500, 'a till prints thousands like this');
  assert.equal(W.parseMinor('12,50', 2), 1250);
  assert.equal(W.parseMinor('12.5', 2), 1250);
  assert.equal(W.parseMinor('12', 2), 1200);
});

test('parseMinor refuses what it would have to guess', () => {
  for (const [s, d] of [['', 0], ['-5', 0], ['12.5', 0], ['1.50', 0], ['12.500', 2], ['1.2.3', 2], ['abc', 0], ['12.555', 2], [null, 0]]) {
    assert.equal(W.parseMinor(s, d), null, `${s} with ${d} decimals`);
  }
  assert.equal(W.parseMinor('99999999999999999999', 0), null, 'past a safe integer');
  assert.equal(W.parseMinor('99999999999999999', 2), null);
});

test('a percentage is parts per million, and back', () => {
  assert.equal(W.pctToPpm('20'), 200000);
  assert.equal(W.pctToPpm('6,5'), 65000);
  assert.equal(W.pctToPpm('0.0001'), 1);
  assert.equal(W.pctToPpm('100'), 1000000);
  for (const bad of ['101', '0.00001', '-1', 'x', '', '20%']) assert.equal(W.pctToPpm(bad), null, bad);
  assert.equal(W.ppmToPct(200000), '20');
  assert.equal(W.ppmToPct(65000), '6.5');
  assert.equal(W.ppmToPct(1), '0.0001');
  assert.equal(W.ppmToPct(''), '0');
  assert.equal(W.ppmToPct(-1), '');
  assert.equal(W.ppmToPct(1.5), '');
});

const TZ = 'Europe/Tirane';
test('a day is the VENUE\'s midnight, and back, whatever the phone\'s zone', () => {
  const ms = W.dayToMs('2026-09-26', TZ);
  assert.equal(ms, Date.UTC(2026, 8, 25, 22), 'Tirana summer is UTC+2');
  assert.equal(W.dayToMs('2026-12-01', TZ), Date.UTC(2026, 10, 30, 23), 'and winter UTC+1');
  assert.equal(W.msToDay(ms, TZ), '2026-09-26');
  assert.equal(W.msToDay(ms - 1, TZ), '2026-09-25');
  assert.equal(W.dayToMs('2026-02-30', TZ), null, 'no such day');
  assert.equal(W.dayToMs('26-09-2026', TZ), null);
  assert.equal(W.msToDay(0, TZ), '');
  assert.equal(W.msToDay('x', TZ), '');
});

test('orders in a date range, newest first, the end day whole even across a clock change', () => {
  const at = d => W.dayToMs(d, TZ) + 3_600_000;
  const orders = [{ id: 'a', created_at_ms: at('2026-09-01'), total: 500 }, { id: 'b', created_at_ms: at('2026-09-10'), total: 700 },
    { id: 'c', created_at_ms: at('2026-09-20'), total: 1.5 }, { id: 'd', created_at_ms: W.dayToMs('2026-10-26', TZ) - 60_000, total: 1 }];
  assert.deepEqual(W.inRange(orders, '2026-09-05', '2026-09-20', TZ).map(o => o.id), ['c', 'b']);
  assert.deepEqual(W.inRange(orders, '', '2026-09-01', TZ).map(o => o.id), ['a'], 'the end day is included');
  assert.deepEqual(W.inRange(orders, '2026-10-25', '2026-10-25', TZ).map(o => o.id), ['d'], 'a 25-hour day keeps its last minute');
  assert.deepEqual(W.inRange(orders, '', '', TZ).map(o => o.id), ['d', 'c', 'b', 'a']);
  assert.deepEqual(W.inRange(null, '', '', TZ), []);
  assert.equal(W.total(orders), 1201, 'a non-integer amount is not money');
  assert.equal(W.total(null), 0);
});

test('archives are newest first', () => {
  assert.deepEqual(W.archivesNewestFirst(['log@3', 'log@12', 'log@7']), ['log@12', 'log@7', 'log@3']);
  assert.deepEqual(W.archivesNewestFirst(null), []);
});

test('a read mark covers the last customer message only', () => {
  const m = [{ from: 'CUSTOMER', kind: 'TEXT', seq: 1 }, { from: 'VENUE', kind: 'TEXT', seq: 5 }, { from: 'CUSTOMER', kind: 'TEXT', seq: 3 }, { from: 'CUSTOMER', kind: 'READ', seq: 9 }];
  assert.equal(W.readThrough(m), 3);
  assert.equal(W.readThrough([]), 0);
  assert.equal(W.readThrough(null), 0);
});

test('pixels travel as hex rgb, transparent ones skipped', () => {
  assert.equal(W.pixelsHex([255, 0, 16, 255, 1, 2, 3, 0, 10, 20, 30, 200]), 'ff00100a141e');
  assert.equal(W.pixelsHex([]), '');
});

test('the VAT schedule round-trips and names a bad row', () => {
  const since = W.dayToMs('2027-01-01', TZ);
  const rows = W.scheduleRows(JSON.stringify([{ since_ms: since, ppm: 70000 }]), TZ);
  assert.deepEqual(rows, [{ day: '2027-01-01', pct: '7' }]);
  assert.deepEqual(W.scheduleJson(rows, TZ), { json: JSON.stringify([{ since_ms: since, ppm: 70000 }]), bad: [] });
  assert.deepEqual(W.scheduleJson([{ day: '', pct: '' }], TZ), { json: '', bad: [] }, 'an empty row is no row');
  assert.deepEqual(W.scheduleJson([{ day: '2027-13-01', pct: '7' }, { day: '2027-01-01', pct: 'x' }], TZ).bad, [1, 2]);
  assert.deepEqual(W.scheduleRows('not json'), []);
  assert.deepEqual(W.scheduleRows('{}'), []);
  assert.deepEqual(W.scheduleRows(''), []);
});

test('a request id is stable for one press', () => {
  assert.equal(W.requestId('tu', 36, 'a-b c'), 'tu-10-abc');
});
