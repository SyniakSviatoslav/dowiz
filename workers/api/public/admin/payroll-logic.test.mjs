// `node --test workers/api/public/admin/payroll-logic.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseHours, hoursLabel, entryPay, shiftMonth, monthTitle, cellOf, monthBounds, dateIn } from './payroll-logic.js';

test('hours typed by a person become integer minutes in quarters', () => {
  for (const [typed, minutes] of [['8', 480], ['7.5', 450], ['7,25', 435], ['7:30', 450], ['0.25', 15], ['24', 1440], [' 6,75 ', 405]]) assert.equal(parseHours(typed), minutes, typed);
});

test('anything that is not a quarter of an hour inside a day is refused, not guessed', () => {
  for (const typed of ['', 'abc', '0', '0.1', '7.3', '7:20', '24.25', '25', '-1', '7.5.5']) assert.equal(parseHours(typed), null, typed);
});

test('minutes read back as hours', () => {
  assert.equal(hoursLabel(480), '8');
  assert.equal(hoursLabel(450), '7.5');
  assert.equal(hoursLabel(435), '7.25');
  assert.equal(hoursLabel(405), '6.75');
});

test('one entry previews to the same half-up integer as the hub', () => {
  assert.equal(entryPay(450, 250), 1875);
  assert.equal(entryPay(15, 250), 63);
  assert.equal(entryPay(60, 300), 300);
});

test('a month steps across years without a Date', () => {
  assert.equal(shiftMonth('2026-11', -1), '2026-10');
  assert.equal(shiftMonth('2026-01', -1), '2025-12');
  assert.equal(shiftMonth('2026-12', 1), '2027-01');
  assert.equal(shiftMonth('2026-05', 0), '2026-05');
});

test('a month title is the month the string names, in any zone', () => {
  assert.match(monthTitle('2026-01', 'en'), /January 2026/);
  assert.match(monthTitle('2026-12', 'en'), /December 2026/);
});

test('a cell is found by person and date', () => {
  const data = { rows: [{ staffId: 'a', days: { '2026-11-08': { minutes: 60 } } }] };
  assert.deepEqual(cellOf(data, 'a', '2026-11-08'), { minutes: 60 });
  assert.equal(cellOf(data, 'a', '2026-11-09'), null);
  assert.equal(cellOf(data, 'b', '2026-11-08'), null);
});

test('the form is bounded by the month on screen and by today', () => {
  assert.deepEqual(monthBounds('2026-11', '2026-11-08'), { min: '2026-11-01', max: '2026-11-08' });
  assert.deepEqual(monthBounds('2026-10', '2026-11-08'), { min: '2026-10-01', max: '2026-10-31' });
  assert.deepEqual(monthBounds('2026-02', '2026-11-08'), { min: '2026-02-01', max: '2026-02-28' });
  assert.deepEqual(monthBounds('2024-02', '2026-11-08'), { min: '2024-02-01', max: '2024-02-29' });
  assert.deepEqual(monthBounds('2100-02', '2101-01-01'), { min: '2100-02-01', max: '2100-02-28' });
  const b = monthBounds('2026-11', '2026-11-08');
  for (const ok of ['2026-11-01', '2026-11-08']) assert.equal(dateIn(ok, b), true, ok);
  for (const bad of ['2026-10-31', '2026-11-09', '2026-12-01', '', 'nope', '2026-11-8']) assert.equal(dateIn(bad, b), false, bad);
});
