// The tip record beside the Z report, called for real. `node --test workers/api/public/room/tips.test.mjs`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as Money from '../lib/money.js';
import { tipsQuery, renderTips } from './till-view.js';

const T = k => `[${k}]`;
const OPEN = { kind: 'till.opened', open: true, opened_at: 1000, float: { ALL: 5000 } };
const CLOSE = { kind: 'till.closed', open: false, opened_at: 1000, closed_at: 9000, float: {}, counted: {}, expected: {}, over_short: {} };

test('tips: the period is the drawer\'s -- opening to close, or to now while open', () => {
  assert.equal(tipsQuery(CLOSE, 'v 1'), '/staff/till/tips?location_id=v%201&from_ms=1000&to_ms=9000');
  assert.equal(tipsQuery(OPEN, 'v1'), '/staff/till/tips?location_id=v1&from_ms=1000', 'open: the server reads to now');
});

test('tips: with no till known, the phone still asks -- for the venue\'s day', () => {
  assert.equal(tipsQuery(null, 'v1'), '/staff/till/tips?location_id=v1', 'a card-only day: no drawer, still tips');
  assert.equal(tipsQuery({ kind: 'till.counted', open: true }, 'v1'), '/staff/till/tips?location_id=v1', 'no start: the server\'s day');
  assert.equal(tipsQuery(null, ''), null, 'no venue, nothing to ask');
});

test('tips: the day\'s answer says it is the day, a drawer\'s says the drawer', () => {
  assert.ok(renderTips({ day: true, tips: [] }, T, 'en').includes('[tipsHintDay]'));
  assert.ok(renderTips({ tips: [] }, T, 'en').includes('[tipsHint]'));
});

test('tips: each person\'s amount in its own currency, by name when the server knows it', () => {
  const res = { tips: [
    { by: 'u1', currency: 'ALL', amount: 1700 },
    { by: 'u2', currency: 'ALL', amount: 250 },
    { by: 'u1', currency: 'EUR', amount: 150 },
  ], names: { u1: 'Ana' } };
  const html = renderTips(res, T, 'en');
  assert.ok(html.includes('[tipsTitle]'));
  assert.ok(html.includes('Ana') && html.includes('u2'), 'a name, else the id: ' + html);
  for (const [n, c] of [[1700, 'ALL'], [250, 'ALL'], [150, 'EUR']]) assert.ok(html.includes(Money.format(n, c, 'en')), `missing ${n} ${c}`);
  assert.ok(!html.includes('[tipsNone]'));
});

test('tips: an empty period says so, and a name is escaped', () => {
  assert.ok(renderTips({ tips: [] }, T, 'en').includes('[tipsNone]'));
  assert.ok(renderTips(null, T, 'en').includes('[tipsNone]'));
  const html = renderTips({ tips: [{ by: 'x', currency: 'ALL', amount: 1 }], names: { x: '<b>' } }, T, 'en');
  assert.ok(html.includes('&lt;b&gt;') && !html.includes('<b>'));
});
