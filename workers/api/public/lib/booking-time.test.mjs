// `node public/lib/booking-time.test.mjs`. No browser: that is the point of
// keeping the slot arithmetic out of the screen.

import assert from 'node:assert/strict';
import {
  midnightMs, slotOf, weekdayOf, minuteNow, timesOn, DAY_MIN,
} from './booking-time.js';

let run = 0;
const test = (name, fn) => { fn(); run++; console.log(`  ok  ${name}`); };

// Durrës is UTC+2 in winter. 2026-09-22T18:30:00Z is 20:30 there, a Tuesday.
const NOW = Date.parse('2026-09-22T18:30:00Z');
const OFF = 120;

test('the venue\'s day is the venue\'s, not the phone\'s', () => {
  assert.equal(minuteNow(NOW, OFF), 20 * 60 + 30, '20:30 in Durrës');
  assert.equal(minuteNow(NOW, 0), 18 * 60 + 30, 'and 18:30 in UTC');
  // 23:30 local is still the same local day; 00:30 the next one is not.
  const lateUtc = Date.parse('2026-09-22T22:30:00Z'); // 00:30 local, Wednesday
  assert.equal(weekdayOf(NOW, OFF), 1, 'Tuesday, with Monday as 0');
  assert.equal(weekdayOf(lateUtc, OFF), 2, 'half past midnight is Wednesday there');
  assert.equal(weekdayOf(lateUtc, 0), 1, 'and still Tuesday in UTC');
});

test('midnight is local midnight, expressed as a real instant', () => {
  const m = midnightMs(NOW, OFF, 0);
  assert.equal(new Date(m).toISOString(), '2026-09-21T22:00:00.000Z',
    'local midnight on the 22nd is 22:00 UTC on the 21st at +2');
  assert.equal(midnightMs(NOW, OFF, 1) - m, 86_400_000, 'one day on');
  assert.equal(midnightMs(NOW, 0, 0), Date.parse('2026-09-22T00:00:00Z'));
});

test('a slot is minutes since the epoch, and 19:00 local means 17:00 UTC', () => {
  const s = slotOf(NOW, OFF, 0, 19 * 60);
  assert.equal(new Date(s * 60_000).toISOString(), '2026-09-22T17:00:00.000Z');
  // The same wall time tomorrow is exactly a day later.
  assert.equal(slotOf(NOW, OFF, 1, 19 * 60) - s, DAY_MIN);
  // A diner whose phone is in Kyiv (+3) still books the venue's 19:00: the
  // offset that decides is the venue's, and nothing here reads the browser's.
  assert.equal(slotOf(NOW, OFF, 0, 19 * 60), s);
});

test('a day offers the half hours inside its windows, up to the last sitting', () => {
  assert.deepEqual(timesOn([{ open: 11 * 60, close: 13 * 60 }]),
    [660, 690, 720], '11:00, 11:30, 12:00 — 12:30 is inside the last hour');
  assert.deepEqual(timesOn([]), []);
  assert.deepEqual(timesOn(null), []);
});

test('a window that closes before it opens is a late kitchen, not an error', () => {
  const t = timesOn([{ open: 22 * 60, close: 2 * 60 }]);
  // The close is carried past midnight, so the last sitting is measured
  // against 02:00: 23:30 is offered. Clamping to midnight instead cut an hour
  // off every late kitchen's evening, which is what this pins.
  assert.deepEqual(t, [1320, 1350, 1380, 1410], '22:00 through 23:30');
  assert.ok(t.every(m => m < DAY_MIN),
    'and nothing after midnight: that belongs to the next calendar day');
  // Against a kitchen that really does shut at midnight, the last sitting is
  // half an hour earlier -- the two cases must not give the same answer.
  assert.deepEqual(timesOn([{ open: 22 * 60, close: 24 * 60 }]), [1320, 1350, 1380]);
});

test('today\'s times that have gone are gone', () => {
  const win = [{ open: 11 * 60, close: 23 * 60 }];
  const whole = timesOn(win, -1);
  const rest = timesOn(win, 20 * 60 + 30);
  assert.equal(whole[0], 660, 'a future day starts at opening');
  assert.equal(rest[0], 21 * 60, 'at 20:30 the next offer is 21:00');
  assert.ok(rest.length < whole.length);
  assert.deepEqual(timesOn(win, 23 * 60), [], 'after the last sitting there is nothing left');
});

test('two services on one day come back as one ordered strip, without repeats', () => {
  const t = timesOn([{ open: 12 * 60, close: 15 * 60 }, { open: 18 * 60, close: 23 * 60 }]);
  assert.deepEqual(t, [...t].sort((a, b) => a - b));
  assert.equal(new Set(t).size, t.length);
  assert.ok(t.includes(720) && t.includes(1080) && !t.includes(900),
    'noon and six are offered, three o\'clock is inside the lunch last sitting');
});

console.log(`booking-time: ${run} tests, all green`);
