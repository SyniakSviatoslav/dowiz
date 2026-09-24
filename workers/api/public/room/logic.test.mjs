// The room app's pure rules, called for real. `node --test workers/api/public/room/*.test.mjs`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as Money from '../lib/money.js';
import {
  parseCaps, actionsFor, canTill, reasonWord, parseMinor, minorToInput, ratePpm, convertPpm,
  maxAmountFor, owed, sittingDue, money, slugOfHost, ageOf, METHODS, REASONS,
  canTransfer, transferTargets, transferBody, canMoveSitting, refusalKey,
  settles, keepTheChange, tipMinor, walletOk, walletTipOk,
} from './logic.js';
import { renderTill, visible } from './till-view.js';

// The canonical spellings `open_session` signs (caps.rs Display, Cap::ALL order).
const WAITER = parseCaps('take_orders,take_payment');
const COUNTER = parseCaps('take_orders,take_payment,void,open_till');
const KITCHEN = parseCaps('advance');
const round = (status, extra = {}) => ({ id: 'o1', seq: 7, status, total: 3000, items: [], ...extra });

test('caps: a waiter before the kitchen may add, re-count, remove, move and take money -- not comp', () => {
  const a = actionsFor(WAITER, round('CONFIRMED'));
  assert.deepEqual(a, { add: true, qty: true, remove: true, comp: false, table: true, pay: true });
});

test('caps: once the kitchen has it, a waiter may only move it and take money', () => {
  const a = actionsFor(WAITER, round('PREPARING'));
  assert.deepEqual(a, { add: false, qty: false, remove: false, comp: false, table: true, pay: true });
});

test('caps: the Counter-Manager (void) may remove a cooked line and comp at any stage', () => {
  assert.deepEqual(actionsFor(COUNTER, round('READY')), { add: false, qty: false, remove: true, comp: true, table: true, pay: true });
  assert.equal(actionsFor(COUNTER, round('PENDING')).comp, true);
});

test('caps: the kitchen is offered nothing here, and the till only to open_till', () => {
  assert.deepEqual(actionsFor(KITCHEN, round('CONFIRMED')), { add: false, qty: false, remove: false, comp: false, table: false, pay: false });
  assert.equal(canTill(COUNTER), true);
  assert.equal(canTill(WAITER), false);
  assert.equal(canTill(KITCHEN), false);
});

test('caps: a paid or finished round offers no change and no payment; a cancelled one no payment', () => {
  const paid = actionsFor(COUNTER, round('CONFIRMED', { payment_status: 'paid' }));
  assert.ok(Object.values(paid).every(v => v === false), JSON.stringify(paid));
  assert.ok(Object.values(actionsFor(COUNTER, round('PICKED_UP', { total: 0 }))).every(v => v === false));
  assert.equal(actionsFor(COUNTER, round('CANCELLED')).pay, false);
  // Positive twin: the same round unpaid still takes money after the pass.
  assert.equal(actionsFor(COUNTER, round('PICKED_UP')).pay, true);
});

test('reasons: the closed set passes, other carries bounded text, anything else is refused', () => {
  assert.deepEqual(REASONS.slice(0, 4).map(r => reasonWord(r)), ['mistake', 'guest_changed', 'unavailable', 'dropped']);
  assert.equal(reasonWord('other', '  corked wine '), 'other:corked wine');
  assert.equal(reasonWord('other', ''), null);
  assert.equal(reasonWord('other', 'x'.repeat(141)), null);
  assert.equal(reasonWord('other', 'x'.repeat(140)), 'other:' + 'x'.repeat(140));
  assert.equal(reasonWord('burnt'), null);
  assert.deepEqual(METHODS, ['cash', 'card', 'cheque', 'transfer', 'gift_card', 'wallet', 'other']);
});

// fx.rs's own worked examples are the fixtures: 1 EUR = 97.50 ALL.
test('rate: the board rate becomes rate_ppm exactly, both directions (fx.rs examples)', () => {
  assert.equal(ratePpm('97.50', 'ALL', 'EUR'), 975000);
  assert.equal(ratePpm('97,5', 'ALL', 'EUR'), 975000);
  assert.equal(ratePpm(' 97.50 ', 'ALL', 'EUR'), 975000);
  assert.equal(ratePpm('97.50', 'EUR', 'ALL'), 1025641);
  assert.equal(convertPpm(2000, 975000), 1950);
});

test('rate: no float anywhere -- digits a float would lose are kept', () => {
  // 0.1 + 0.2-class inputs: exact as integers.
  assert.equal(ratePpm('1.1', 'ALL', 'EUR'), 11000);
  assert.equal(ratePpm('97.505', 'ALL', 'EUR'), 975050);
  assert.equal(ratePpm('100.3', 'ALL', 'EUR'), 1003000);
  // Half-up on the reverse: 1e8 / 97.505 = 1025588.43... -> 1025588; 1e8/3 = 33333333.33 -> 33333333
  assert.equal(ratePpm('97.505', 'EUR', 'ALL'), 1025588);
  assert.equal(ratePpm('3', 'EUR', 'ALL'), 33333333);
  // 1e8 / 64 = 1562500 exactly; 1e8 / 96 = 1041666.67 -> 1041667 (up)
  assert.equal(ratePpm('64', 'EUR', 'ALL'), 1562500);
  assert.equal(ratePpm('96', 'EUR', 'ALL'), 1041667);
});

test('rate: same currency, nonsense and zero are refused (the server refuses a rate on a same-currency payment)', () => {
  assert.equal(ratePpm('97.50', 'ALL', 'ALL'), null);
  assert.equal(ratePpm('abc', 'ALL', 'EUR'), null);
  assert.equal(ratePpm('0', 'ALL', 'EUR'), null);
  assert.equal(ratePpm('-97', 'ALL', 'EUR'), null);
  assert.equal(ratePpm('', 'ALL', 'EUR'), null);
});

test('owed: partial payments, one of them foreign, leave the right remainder', () => {
  const r = round('CONFIRMED', { total: 3000, payments: [
    { amount: 1000, method: 'cash', currency: 'ALL' },
    { amount: 1000, method: 'cash', currency: 'EUR', rate_ppm: 975000, amount_in_order_currency: 975 },
  ] });
  assert.equal(owed(r), 1025);
  assert.equal(owed({ ...r, payment_status: 'paid' }), 0);
  assert.equal(owed(round('CONFIRMED', { total: 3000, paid: 1200 })), 1800);
  assert.equal(owed(round('CONFIRMED', { total: 3000 })), 3000);
});

test('owed: the most euro that does not overpay, and one cent more that would', () => {
  const a = maxAmountFor(1950, 975000);
  assert.equal(a, 2000);
  assert.ok(convertPpm(a, 975000) <= 1950);
  assert.ok(convertPpm(a + 1, 975000) > 1950);
  assert.equal(maxAmountFor(0, 975000), 0);
});

test('owed: a sitting sums its billed rounds only', () => {
  const s = { rounds: [round('CONFIRMED', { total: 1000 }), round('CANCELLED', { total: 5000 }), round('READY', { total: 700, payments: [{ amount: 200 }] })] };
  assert.equal(sittingDue(s), 1500);
});

test('money: ALL has 0 decimals, EUR 2, and both come from lib/money.js', () => {
  assert.equal(money(1500, 'ALL', 'en'), Money.format(1500, 'ALL', 'en'));
  assert.equal(money(1500, 'EUR', 'en'), Money.format(1500, 'EUR', 'en'));
  assert.match(money(1500, 'ALL', 'en'), /1,500/);
  assert.doesNotMatch(money(1500, 'ALL', 'en'), /15\.00|\.00/);
  assert.match(money(1500, 'EUR', 'en'), /15\.00/);
  // An unknown currency is a dash, never a guessed two decimals.
  assert.equal(money(1500, null, 'en'), '—');
});

test('amounts typed: parsed on the currency\'s decimals, never rounded', () => {
  assert.equal(parseMinor('15', 'EUR'), 1500);
  assert.equal(parseMinor('15,5', 'EUR'), 1550);
  assert.equal(parseMinor('0.05', 'EUR'), 5);
  assert.equal(parseMinor('1 950', 'ALL'), 1950);
  assert.equal(parseMinor('15.5', 'ALL'), null);
  assert.equal(parseMinor('1.234', 'EUR'), null);
  assert.equal(parseMinor('-5', 'EUR'), null);
  assert.equal(minorToInput(1500, 'EUR'), '15.00');
  assert.equal(minorToInput(5, 'EUR'), '0.05');
  assert.equal(minorToInput(1950, 'ALL'), '1950');
  assert.equal(parseMinor(minorToInput(123456, 'EUR'), 'EUR'), 123456);
});

const T = k => `[${k}]`;
const COUNT_LEAKY = { kind: 'till.counted', till_id: 'main', open: true, opened_at: 1, float: { ALL: 5000 }, counted: { ALL: 12345 }, expected: { ALL: 98765 }, over_short: { ALL: -86420 } };
const CLOSE = { kind: 'till.closed', till_id: 'main', open: false, opened_at: 1, closed_at: 2, float: { ALL: 5000 }, counted: { ALL: 12345, EUR: 2000 }, expected: { ALL: 98765, EUR: 2000 }, over_short: { ALL: -86420, EUR: 0 } };

test('till: a COUNT is drawn blind even if the answer carried the expected figure', () => {
  const html = renderTill(COUNT_LEAKY, T, 'en', () => 't');
  assert.ok(html.includes(Money.format(12345, 'ALL', 'en')), 'the count itself is shown');
  assert.ok(!html.includes(Money.format(98765, 'ALL', 'en')), 'expected must not be drawn: ' + html);
  assert.ok(!html.includes(Money.format(-86420, 'ALL', 'en')), 'over/short must not be drawn');
  assert.ok(!html.includes('[expected]') && !html.includes('[overShort]'));
  assert.equal('expected' in visible(COUNT_LEAKY), false);
});

test('till: the CLOSE shows expected, counted and over/short per currency', () => {
  const html = renderTill(CLOSE, T, 'en', () => 't');
  for (const n of [98765, 12345, -86420]) assert.ok(html.includes(Money.format(n, 'ALL', 'en')), 'missing ' + n);
  assert.ok(html.includes(Money.format(2000, 'EUR', 'en')));
  assert.ok(html.includes('[expected]') && html.includes('[overShort]'));
  assert.match(html, /class="os short"/);
});

test('where: the venue is the host\'s first label, ?s= wins, workers.dev names none', () => {
  assert.equal(slugOfHost('sushi-durres.dowiz.org', ''), 'sushi-durres');
  assert.equal(slugOfHost('dowiz.org', ''), null);
  assert.equal(slugOfHost('x.workers.dev', '?s=demo'), 'demo');
  assert.equal(slugOfHost('dowiz-api.x.workers.dev', ''), null);
  assert.deepEqual(ageOf(59_000), { n: 59, unit: 's' });
  assert.deepEqual(ageOf(125_000), { n: 2, unit: 'm' });
});

// ── moving lines and tables ─────────────────────────────────────────────────
const lined = (id, status, n, extra = {}) => round(status, { id, seq: 10, items: Array.from({ length: n }, (_, i) => ({ product_id: 'p' + i, quantity: 1, unit_price: 100 })), ...extra });

test('transfer: only before the kitchen, unpaid, and only for take_orders', () => {
  assert.equal(canTransfer(WAITER, lined('a', 'CONFIRMED', 2)), true, 'the positive twin');
  assert.equal(canTransfer(WAITER, lined('a', 'PREPARING', 2)), false, 'the kitchen has it');
  assert.equal(canTransfer(COUNTER, lined('a', 'READY', 2)), false, 'void does not make it legal');
  assert.equal(canTransfer(WAITER, lined('a', 'PENDING', 2, { payment_status: 'paid' })), false, 'paid');
  assert.equal(canTransfer(KITCHEN, lined('a', 'PENDING', 2)), false, 'no take_orders');
});

test('transfer: targets are every other movable round, across sittings', () => {
  const sittings = [
    { sitting_id: 's1', rounds: [lined('a', 'PENDING', 2), lined('b', 'CONFIRMED', 1), lined('c', 'PREPARING', 1)] },
    { sitting_id: 's2', rounds: [lined('d', 'PENDING', 1), lined('e', 'PENDING', 1, { payment_status: 'paid' })] },
  ];
  assert.deepEqual(transferTargets(WAITER, sittings, 'a').map(x => x.round.id), ['b', 'd']);
  assert.deepEqual(transferTargets(KITCHEN, sittings, 'a'), []);
});

test('transfer: the body carries both versions and the sorted, unique lines', () => {
  const from = lined('a', 'PENDING', 3, { seq: 11 }), to = lined('b', 'PENDING', 1, { seq: 22 });
  assert.deepEqual(transferBody('v1', from, to, [2, '0', 2]),
    { location_id: 'v1', to_order_id: 'b', from_base_seq: 11, to_base_seq: 22, lines: [0, 2] });
});

test('transfer: refused before sending -- no target, no lines, itself, every line, out of range', () => {
  const from = lined('a', 'PENDING', 2), to = lined('b', 'PENDING', 1);
  assert.deepEqual(transferBody('v1', from, null, [0]), { error: 'pickRound' });
  assert.deepEqual(transferBody('v1', from, from, [0]), { error: 'pickRound' });
  assert.deepEqual(transferBody('v1', from, to, []), { error: 'pickLines' });
  assert.deepEqual(transferBody('v1', from, to, [5, -1]), { error: 'pickLines' });
  assert.deepEqual(transferBody('v1', from, to, [0, 1]), { error: 'notAllLines' });
  assert.equal(transferBody('v1', from, to, [1]).lines[0], 1, 'the positive twin');
});

test('move sitting: a live round and nothing paid-but-unserved', () => {
  assert.equal(canMoveSitting(WAITER, { rounds: [lined('a', 'PREPARING', 1)] }), true, 'the positive twin');
  assert.equal(canMoveSitting(WAITER, { rounds: [lined('a', 'READY', 1, { payment_status: 'paid' })] }), false);
  assert.equal(canMoveSitting(WAITER, { rounds: [lined('a', 'DELIVERED', 1)] }), false, 'nothing left in the room');
  assert.equal(canMoveSitting(KITCHEN, { rounds: [lined('a', 'PENDING', 1)] }), false);
});

test('refusals: the server\'s fixed phrases map to the waiter\'s words; anything else is shown as said', () => {
  assert.equal(refusalKey(409, 'this order changed while you were editing it'), 'changedReload');
  assert.equal(refusalKey(409, 'the kitchen has the destination round: nothing moves between rounds after that'), 'kitchenHasIt');
  assert.equal(refusalKey(409, 'this round is paid; taking money off it now is a refund'), 'roundPaid');
  assert.equal(refusalKey(409, 'round o1 is paid and not yet served; move the table once it is'), 'roundPaid');
  assert.equal(refusalKey(400, 'no open round of this sitting is anywhere but table 7'), 'alreadyThere');
  assert.equal(refusalKey(404, 'order not found'), 'notHere');
  assert.equal(refusalKey(500, 'hub append failed'), null);
});

// ── a tip at payment (§2.3) ─────────────────────────────────────────────────
test('tip: a payment settles its bill share plus its tip, and owed follows', () => {
  assert.equal(settles({ amount: 1500, tip: 200 }), 1700);
  assert.equal(settles({ amount: 1500 }), 1500, 'the twin: no tip');
  assert.equal(settles({ amount: 20, amount_in_order_currency: 1950, tip: 50 }), 2000);
  // The server raised the total by the tip, so the round is square.
  assert.equal(owed({ total: 1700, payments: [{ amount: 1500, tip: 200 }] }), 0);
});

test('tip: keep the change is amount = owed, tip = the rest; short is refused', () => {
  assert.deepEqual(keepTheChange(2000, 1800), { amount: 1800, tip: 200 });
  assert.deepEqual(keepTheChange(1800, 1800), { amount: 1800, tip: 0 });
  assert.equal(keepTheChange(1799, 1800), null, 'short of the bill is not a tip');
  assert.equal(keepTheChange(2000, 0), null, 'nothing owed');
});

test('tip: the field reads as minor units; empty is none; junk is refused', () => {
  assert.equal(tipMinor('', 'ALL'), 0);
  assert.equal(tipMinor('200', 'ALL'), 200);
  assert.equal(tipMinor('2.50', 'EUR'), 250);
  assert.equal(tipMinor('-5', 'ALL'), null);
  assert.equal(tipMinor('abc', 'ALL'), null);
});

test('wallet: a wallet payment names its wallet and nothing else needs one', () => {
  assert.equal(walletOk('wallet', ''), false);
  assert.equal(walletOk('wallet', ' u1 '), true);
  assert.equal(walletOk('card', ''), true);
});

test('wallet: a wallet pays the bill only; a tip rides on any other method', () => {
  assert.equal(walletTipOk('wallet', 100), false);
  assert.equal(walletTipOk('wallet', 0), true, 'the twin: no tip');
  assert.equal(walletTipOk('card', 100), true);
});
