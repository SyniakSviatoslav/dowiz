// `node public/lib/booking-time.test.mjs`. No browser: that is the point of
// keeping the slot arithmetic out of the screen.

import assert from 'node:assert/strict';
import {
  midnightMs, slotOf, weekdayOf, minuteNow, timesOn, DAY_MIN, offsetMinutes,
  venueClock, venueDate, venueMidnightMs, venueSlot, venueDayRange, slotMinuteOfDay,
  venueWallMs, venueWallValue, laterPrefill,
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

test('the offset is the zone\'s at that instant, not a summer constant', () => {
  // Europe/Tirane: CEST (+120) until the last Sunday of October 2026, CET (+60) after.
  assert.equal(offsetMinutes('Europe/Tirane', Date.parse('2026-09-24T12:00:00Z')), 120);
  assert.equal(offsetMinutes('Europe/Tirane', Date.parse('2026-10-26T12:00:00Z')), 60,
    'the defect: +120 all year books every winter slot an hour late');
  assert.equal(offsetMinutes('UTC', Date.parse('2026-01-01T00:00:00Z')), 0);
  const err = console.error; console.error = () => {};
  try { assert.equal(offsetMinutes('Not/AZone', 0, 60), 60, 'an unknown name falls back, loudly'); }
  finally { console.error = err; }
});

// ── G3: the venue's day across the DST change (audit D3, D10, D11) ─────────
const TZ = 'Europe/Tirane';
const tirane = ms => new Intl.DateTimeFormat('en-GB', { timeZone: TZ, dateStyle: 'short', timeStyle: 'short' })
  .format(new Date(ms));

test('D3: booked on 24 Oct for Mon 26 Oct 19:00, the slot IS 19:00 in Tirane', () => {
  const now = Date.UTC(2026, 9, 24, 10, 0); // Sat 12:00 CEST (+120)
  const slot = venueSlot(now, TZ, 2, 19 * 60);
  assert.equal(tirane(slot * 60_000), '26/10/2026, 19:00', 'not 18:00: the offset is the 26th\'s (+60)');
  assert.equal(new Date(slot * 60_000).toISOString(), '2026-10-26T18:00:00.000Z');
  assert.equal(slotMinuteOfDay(TZ, slot), 19 * 60, 'and the console renders it at 19:00');
  // its positive twin: a day before the change keeps +120
  assert.equal(tirane(venueSlot(now, TZ, 0, 19 * 60) * 60_000), '24/10/2026, 19:00');
});

test('D3: the day of 25 Oct is 25 hours, from two real midnights', () => {
  const now = Date.UTC(2026, 9, 24, 10, 0);
  const [from, to] = venueDayRange(now, TZ, 1);
  assert.equal(to - from, 25 * 60, 'the hour 02:00-03:00 happens twice');
  assert.equal(new Date(from * 60_000).toISOString(), '2026-10-24T22:00:00.000Z');
  assert.equal(new Date(to * 60_000).toISOString(), '2026-10-25T23:00:00.000Z');
  const [f0, t0] = venueDayRange(now, TZ, 0);
  assert.equal(t0 - f0, DAY_MIN, 'an ordinary day is 1440');
  const spring = venueDayRange(Date.UTC(2027, 2, 27, 12), TZ, 1); // Sun 28 Mar 2027
  assert.equal(spring[1] - spring[0], 23 * 60, 'the spring day is 23 hours');
  assert.equal(venueDate(now, TZ, 2).weekday, 0, 'the 26th is a Monday');
  assert.equal(venueDate(now, TZ, 2).d, 26);
});

test('P6: every 5 hours of 2026, every day 0..60, the slot renders at its minute', () => {
  const MINUTES = [0, 90, 19 * 60, 23 * 60 + 30];
  let cases = 0;
  for (let now = Date.UTC(2026, 0, 1); now < Date.UTC(2027, 0, 1); now += 5 * 3_600_000) {
    const today = venueClock(TZ, now);
    for (let n = 0; n <= 60; n++) {
      const want = venueDate(now, TZ, n);
      for (const m of MINUTES) {
        const at = venueClock(TZ, venueSlot(now, TZ, n, m) * 60_000);
        if (at.minute !== m || at.d !== want.d || at.mo !== want.mo) {
          assert.fail(`now ${new Date(now).toISOString()} day ${n} minute ${m} -> ${JSON.stringify(at)}`);
        }
        cases++;
      }
    }
    assert.equal(venueMidnightMs(now, TZ, 0) <= now && now < venueMidnightMs(now, TZ, 1), true);
    assert.equal(venueClock(TZ, venueMidnightMs(now, TZ, 0)).minute, 0, `midnight at ${today.d}/${today.mo + 1}`);
  }
  assert.equal(cases, 1752 * 61 * 4, 'every case executed, none skipped');
});

test('D10: checkout "later" is the venue\'s 19:00 whatever the phone\'s zone', () => {
  const ms = venueWallMs(TZ, '2026-09-24T19:00');
  assert.equal(tirane(ms), '24/09/2026, 19:00');
  assert.equal(new Date(ms).toISOString(), '2026-09-24T17:00:00.000Z');
  assert.equal(tirane(venueWallMs(TZ, '2026-10-26T19:00')), '26/10/2026, 19:00', 'and after the change');
  assert.equal(venueWallValue(TZ, ms), '2026-09-24T19:00', 'the prefill round-trips on the venue wall');
  assert.equal(venueWallMs(TZ, ''), null);
  assert.equal(venueWallMs(TZ, '2026-13-01T19:00'), null);
});

test('D11: the venue\'s weekday and minute come from the zone, not +120', () => {
  const winter = venueClock(TZ, Date.UTC(2026, 9, 26, 21, 30)); // Mon 22:30 CET
  assert.equal(winter.minute, 22 * 60 + 30, 'not 23:30');
  assert.equal(winter.weekday, 0);
  const summer = venueClock(TZ, Date.UTC(2026, 8, 21, 20, 30)); // Mon 22:30 CEST
  assert.equal(summer.minute, 22 * 60 + 30);
});

test('D10: the "later" prefill is the venue\'s wall, an hour on, rounded up to :00/:30', () => {
  const H = 3_600_000;
  assert.equal(laterPrefill(TZ, Date.UTC(2026, 8, 24, 16, 40), H, 30), '2026-09-24T20:00', '18:40 CEST + 1h -> 20:00');
  assert.equal(laterPrefill(TZ, Date.UTC(2026, 9, 26, 16, 10), H, 30), '2026-10-26T18:30', '17:10 CET + 1h -> 18:30');
  assert.equal(laterPrefill(TZ, Date.UTC(2026, 9, 26, 17, 0), H, 30), '2026-10-26T19:00', 'on the half hour stays');
  // the prefill read back is the instant it names, on the venue's wall
  assert.equal(venueWallMs(TZ, laterPrefill(TZ, Date.UTC(2026, 9, 26, 17, 0), H, 30)), Date.UTC(2026, 9, 26, 18, 0));
});

console.log(`booking-time: ${run} tests, all green`);
