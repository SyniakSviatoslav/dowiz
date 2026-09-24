// `node public/lib/booking-guest.test.mjs`. A guest's booking rules, no browser.

import assert from 'node:assert/strict';
import { guestErrors, bookingBody, remember, forget, ofVenue, linkHash, parseHash, canCancel, refusalText, STATUS_KEY, KEEP } from './booking-guest.js';

let run = 0;
const test = (name, fn) => { fn(); run++; console.log(`  ok  ${name}`); };

test('a booking needs a name and a callable phone', () => {
  assert.deepEqual(guestErrors({ name: '', phone: '' }), ['bkNeedName', 'bkNeedPhone']);
  assert.deepEqual(guestErrors({ name: 'Ana', phone: '12345' }), ['bkNeedPhone']);
  assert.deepEqual(guestErrors({ name: 'a'.repeat(81), phone: '+355 69 123 4567' }), ['bkNameLong']);
  // The twin.
  assert.deepEqual(guestErrors({ name: ' Ana ', phone: '069 123 4567' }), []);
});

test('the body carries a table only when both halves are there', () => {
  const b = bookingBody({ party: 2, slotMin: 100, pick: { zone: 'terasa', n: 4 }, name: ' Ana ', phone: ' 069 ', rid: 'r1' });
  assert.deepEqual(b, { party: 2, slotMin: 100, contactName: 'Ana', contactPhone: '069', requestId: 'r1', zoneId: 'terasa', tableN: 4 });
  const none = bookingBody({ party: 2, slotMin: 100, pick: null, name: 'A', phone: '1', rid: 'r' });
  assert.ok(!('zoneId' in none) && !('tableN' in none), 'no table is no table, not half of one');
  assert.ok(!('zoneId' in bookingBody({ party: 1, slotMin: 1, pick: { zone: 'z' }, name: 'A', phone: '1', rid: 'r' })));
});

test('the remembered list is newest first, one per id, bounded', () => {
  let l = [];
  l = remember(l, { id: 'rsv_1', t: 'a', slug: 's' });
  l = remember(l, { id: 'rsv_2', t: 'b', slug: 's' });
  l = remember(l, { id: 'rsv_1', t: 'c', slug: 's' });
  assert.deepEqual(l.map(x => [x.id, x.t]), [['rsv_1', 'c'], ['rsv_2', 'b']]);
  assert.deepEqual(forget(l, 'rsv_1').map(x => x.id), ['rsv_2']);
  assert.equal(remember(l, { id: 'x' }), l, 'an entry without a token is not kept');
  for (let i = 0; i < KEEP + 5; i++) l = remember(l, { id: `rsv_${i + 10}`, t: 't', slug: i % 2 ? 's' : 'o' });
  assert.equal(l.length, KEEP);
  assert.ok(ofVenue(l, 's').every(x => x.slug === 's'));
});

test('a link round-trips, and a mangled one is nothing', () => {
  const id = 'rsv_0123456789abcdef', tok = 'eyJ.eyJ.sig-_';
  assert.deepEqual(parseHash(linkHash(id, tok)), { id, t: tok });
  assert.equal(parseHash('#rsv=rsv_zz&t=x'), null, 'not a reservation id');
  assert.equal(parseHash('#rsv=' + id), null, 'no token');
  assert.equal(parseHash(''), null);
});

test('cancel is offered exactly when the hub lists it', () => {
  assert.equal(canCancel({ next: ['CANCELLED_BY_GUEST'] }), true);
  assert.equal(canCancel({ next: [] }), false);
  assert.equal(canCancel({}), false);
});

test('a refusal reads the same plain or replayed', () => {
  assert.equal(refusalText('table 1 is taken'), 'table 1 is taken');
  assert.equal(refusalText('{"error":"table 1 is taken"}'), 'table 1 is taken');
});

test('every status the kernel knows has words', () => {
  for (const s of ['REQUESTED', 'CONFIRMED', 'SEATED', 'COMPLETED', 'DECLINED', 'CANCELLED_BY_GUEST', 'CANCELLED_BY_VENUE', 'NO_SHOW'])
    assert.ok(STATUS_KEY[s], s);
});

console.log(`booking-guest: ${run} tests passed`);
