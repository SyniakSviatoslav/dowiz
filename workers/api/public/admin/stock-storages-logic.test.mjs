// node --test workers/api/public/admin/stock-storages-logic.test.mjs
// W-STORE: what the Storages and HACCP sheets send, and the freezing rule
// number for number with the hub (`crates/dowiz-hub/src/stock/haccp.rs`).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as L from './stock-storages-logic.js';

test('the freezing rule at its edges, as the hub reads it', () => {
  assert.equal(L.ruleKey(23, -20), 'hc_ruleNone', '-20 C for 23 h is one hour short');
  assert.equal(L.ruleKey(24, -20), 'hc_rule20');
  assert.equal(L.ruleKey(200, -19), 'hc_ruleNone');
  assert.equal(L.ruleKey(14, -35), 'hc_ruleNone');
  assert.equal(L.ruleKey(15, -35), 'hc_rule35');
  assert.equal(L.ruleKey('', ''), 'hc_ruleNone');
});

test('a transfer, a storage count and a freezing record send integers', () => {
  assert.deepEqual(L.moveBody('salmon', '0,5 kg', 'g', 'freezer', 'kitchen'), { body: { item: 'salmon', qty: 500, from: 'freezer', to: 'kitchen' } });
  assert.equal(L.moveBody('salmon', '500', 'g', 'bar', 'bar').error, 'sto_to');
  assert.equal(L.moveBody('salmon', '0', 'g', 'bar', 'kitchen').error, 'required');
  assert.equal(L.moveBody('', '5', 'g', 'bar', 'kitchen').error, 'sto_item');
  assert.deepEqual(L.countBody([['salmon', '450'], ['rice', ''], ['nori', '2']], 'kitchen', i => (i === 'nori' ? 'unit' : 'g')),
    { body: { lines: [{ item: 'salmon', observed: 450 }, { item: 'nori', observed: 2 }], store: 'kitchen' } });
  assert.equal(L.countBody([['salmon', 'abc']], 'bar', () => 'g').error, 'salmon');
  assert.deepEqual(L.freezeBody('salmon', 'L1', '24', '-20', 'freezer'), { body: { item: 'salmon', lot: 'L1', hours: 24, tempC: -20, store: 'freezer' } });
  assert.equal(L.freezeBody('salmon', '', '24', '-20').error, 'hc_lotPick');
  assert.equal(L.freezeBody('salmon', 'L1', '24', '-20.5').error, 'required', 'a temperature is whole degrees');
});

test('rows per storage, the export path, the date range', () => {
  const sup = [
    { id: 'salmon', name: 'Salmon', unit: 'g', byStore: { stores: { freezer: 1500, kitchen: 450 }, home: 'kitchen' } },
    { id: 'rice', name: 'Rice', unit: 'g', byStore: { stores: {}, home: 'kitchen' } },
  ];
  assert.deepEqual(L.rowsFor(sup, 'freezer').map(r => [r.id, r.qty]), [['salmon', 1500]]);
  assert.deepEqual(L.rowsFor(sup, 'all').map(r => [r.id, r.qty, r.home]), [['salmon', 1950, 'kitchen']]);
  assert.equal(L.exportPath('lots', '2026-10-01', '2026-10-05'), '/owner/stock/haccp?kind=lots&from=2026-10-01&to=2026-10-05');
  assert.equal(L.daysBefore('2026-03-01', 1), '2026-02-28');
  assert.equal(L.nameOf({ id: 'bar', name: '' }, k => `<${k}>`), '<sto_bar>');
  assert.equal(L.nameOf({ id: 'cellar', name: 'Cellar' }, k => k), 'Cellar');
  assert.deepEqual(L.withStore({ item: 'x', qty: 1 }, 'freezer'), { item: 'x', qty: 1, store: 'freezer' });
  assert.deepEqual(L.withStore({ item: 'x', qty: 1, store: 'bar' }, 'freezer'), { item: 'x', qty: 1, store: 'bar' });
  assert.deepEqual(L.withStore({ item: 'x' }, ''), { item: 'x' });
});

test('W-STORE2: a freezing start in local time, hours then optional', () => {
  assert.deepEqual(L.freezeBody('salmon', 'L1', '', '-20', 'freezer', '2026-10-05T14:30'),
    { body: { item: 'salmon', lot: 'L1', tempC: -20, store: 'freezer', started: '2026-10-05T14:30' } });
  assert.deepEqual(L.freezeBody('salmon', 'L1', '24', '-20', '', '2026-10-05T14:30'),
    { body: { item: 'salmon', lot: 'L1', hours: 24, tempC: -20, started: '2026-10-05T14:30' } });
  assert.equal(L.freezeBody('salmon', 'L1', '24', '-20', '', '5 oct').error, 'hc_started');
  assert.equal(L.freezeBody('salmon', 'L1', '', '-20').error, 'required', 'no start: the hours are required');
  const start = '2026-10-05T10:00', at = new Date(start).getTime();
  assert.equal(L.hoursSince(start, at + 20 * 3600000 + 59 * 60000), 20);
  assert.equal(L.hoursSince('nonsense', at), null);
  assert.equal(L.ruleKey(L.hoursSince(start, at + 20 * 3600000), -20), 'hc_ruleNone', 'a 20 h freeze typed as 24 h meets nothing');
});

test('W-STORE2: a station is bound here, elsewhere, or not at all', () => {
  const all = [{ id: 'kitchen', stations: [] }, { id: 'freezer', stations: ['sushi'] }, { id: 'bar' }];
  assert.equal(L.boundTo(all, 'sushi')?.id, 'freezer');
  assert.equal(L.boundTo(all, 'bar'), undefined);
  assert.deepEqual(L.bindBody('sushi', 'freezer', true), { station: 'sushi', store: '' }, 'tapping a bound station unbinds it');
  assert.deepEqual(L.bindBody('bar', 'freezer', false), { station: 'bar', store: 'freezer' });
  assert.deepEqual(L.STATIONS, ['kitchen', 'sushi', 'bar']);
});

test('W-STORE2: a recorded end needs the start and decides with it', () => {
  assert.deepEqual(L.freezeBody('salmon', 'L1', '', '-20', '', '2026-10-04T08:00', '2026-10-05T04:00'),
    { body: { item: 'salmon', lot: 'L1', tempC: -20, started: '2026-10-04T08:00', ended: '2026-10-05T04:00' } });
  assert.equal(L.freezeBody('salmon', 'L1', '24', '-20', '', '', '2026-10-05T04:00').error, 'hc_ended', 'an end without a start');
  assert.equal(L.freezeBody('salmon', 'L1', '', '-20', '', '2026-10-05T04:00', '2026-10-05T04:00').error, 'hc_ended', 'an end at the start');
  const later = new Date('2026-10-05T14:00').getTime();
  assert.equal(L.hoursSince('2026-10-04T08:00', later, '2026-10-05T04:00'), 20, 'a late record adds no hours');
});
