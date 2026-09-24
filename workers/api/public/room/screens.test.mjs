// The room's screens on /lib/ui, rendered in node against the real components.
// `node --test workers/api/public/room/*.test.mjs`
//
// What these hold: every hook app.js binds and the e2e walks drive
// (`data-act`, `data-form`, the control `name`s) is on the screen that needs
// it; every server string (a table, a dish, a status) is escaped; money wears
// `.money` and goes through logic.money; one main action per screen; the
// learning anchors (`data-tour`) are on the controls lessons W1-W6 name.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderLogin, renderRoom, renderSitting } from './screens.js';
import { renderRound } from './sheet.js';
import { renderGuestBar } from './guest.js';
import { actionRow, pickChips, backBar, loading } from './parts.js';
import { parseCaps } from './logic.js';
import { useTranslator } from '../lib/ui/core.js';
import { XSS, injected, render } from '../lib/ui/dom-shim.mjs';

const t = k => `«${k}»`;
useTranslator(t);
const ctx = (S = {}, extra = {}) => ({
  S: { caps: parseCaps(''), sittings: [], currency: 'ALL', at: 1, ...S }, t, statusWord: s => `~${s}`, locale: () => 'en',
  tableOf: r => r?.fulfilment?.table || '', sitting: () => null, ...extra,
});
const $ = (h, s) => render(h).root.querySelector(s);
const $$ = (h, s) => render(h).root.querySelectorAll(s);
const primaries = h => $$(h, '.ui-btn--primary,.ui-btn--success').length;
const tours = h => $$(h, '[data-tour]').map(e => e.getAttribute('data-tour'));

test('login: the form the walks drive, fields labelled, one primary, anchors', () => {
  const h = renderLogin(ctx({ claiming: false }));
  assert.ok($(h, 'form[data-form="login"]'));
  for (const n of ['email', 'password']) assert.ok($(h, `input[name="${n}"]`), n);
  assert.equal($(h, 'input[name="code"]'), null, 'no invite code unless claiming');
  assert.equal($(h, 'input[name="password"]').getAttribute('autocomplete'), 'current-password');
  assert.equal($(h, 'label[for="roomEmail"]').textContent, '«email»');
  assert.equal($(h, 'form[data-form="login"] button[type="submit"]').textContent, '«signIn»');
  assert.ok($(h, '[data-act="claimToggle"]'));
  assert.equal(primaries(h), 1);
  assert.deepEqual(tours(h), ['login.email', 'login.password', 'login.submit', 'login.claimToggle']);
});

test('login, claiming: the invite code, a new password of 8+, the claim words', () => {
  const h = renderLogin(ctx({ claiming: true }));
  assert.ok($(h, 'input[name="code"][autocomplete="one-time-code"]'));
  const pw = $(h, 'input[name="password"]');
  assert.equal(pw.getAttribute('autocomplete'), 'new-password');
  assert.equal(pw.getAttribute('minlength'), '8');
  assert.equal($(h, 'button[type="submit"]').textContent, '«claim»');
  assert.ok(tours(h).includes('login.code'));
});

const sit = (o = {}) => ({ sitting_id: 's1', table: '7', rounds: [{ id: 'r1', status: 'CONFIRMED', total: 1500, subtotal: 1500, payment_status: 'unpaid', seq: 3 }], ...o });

test('room: one card per table with its rounds, statuses and what is due', () => {
  const S = { caps: parseCaps('take_orders'), sittings: [sit(), sit({ sitting_id: 's2', table: '9', rounds: [{ id: 'g', status: 'PENDING', placed_by: 'guest', total: 700 }] })] };
  const h = renderRoom(ctx(S));
  const cards = $$(h, '.tables [data-act="sit"]');
  assert.deepEqual(cards.map(c => c.getAttribute('data-id')), ['s1', 's2']);
  assert.ok(cards[0].classList.contains('card') && cards[0].classList.contains('ui-row'));
  assert.equal($(h, '[data-id="s1"] .ui-status').getAttribute('data-status'), 'CONFIRMED');
  assert.match($(h, '[data-id="s1"] .money').textContent, /1[,.\s ]?500/);
  assert.ok($(h, '[data-id="s2"] .ui-badge--warning'), 'a guest round waiting is flagged');
  assert.equal($(h, '[data-id="s1"] .ui-badge--warning'), null);
  assert.ok($(h, '.bar [data-act="open"]') && $(h, '.bar [data-act="floor"]') && $(h, '[data-act="refresh"]') && $(h, '[data-act="signout"]'));
  assert.equal($(h, '[data-act="till"]'), null, 'no open_till, no till');
  assert.equal(primaries(h), 1);
  assert.ok(tours(h).includes('room.table') && tours(h).includes('room.open'));
});

test('room: caps decide the bar, the kitchen gets no room, first paint is a skeleton', () => {
  const till = renderRoom(ctx({ caps: parseCaps('open_till') }));
  assert.ok($(till, '[data-act="till"]'));
  assert.equal($(till, '[data-act="open"]'), null);
  assert.ok($(renderRoom(ctx({ role: 'kitchen', caps: parseCaps('advance') })), '.ui-empty [data-t="kitchenNoRoom"]'));
  assert.ok($(renderRoom(ctx({ at: 0 })), '.ui-skel-wrap[aria-busy="true"]'), 'before the first answer: the shape, not "no tables"');
  assert.ok($(renderRoom(ctx({ at: 5 })), '.ui-empty [data-t="noOrders"]'));
});

test('room and sitting: a table name from the server is escaped', () => {
  const evil = sit({ table: XSS, rounds: [{ id: XSS, status: XSS, total: 1 }] });
  assert.deepEqual(injected(renderRoom(ctx({ sittings: [evil] }))), []);
  assert.deepEqual(injected(renderSitting(ctx(), evil)), []);
});

test('sitting: its rounds, and move for the caps that may', () => {
  const s = sit({ rounds: [{ id: 'a', status: 'PENDING', total: 100 }, { id: 'b', status: 'READY', total: 200 }] });
  const h = renderSitting(ctx(), s);
  assert.deepEqual($$(h, '[data-act="round"]').map(e => e.getAttribute('data-id')), ['a', 'b']);
  assert.ok($(h, '[data-act="back"]'));
  assert.equal($(h, '[data-act="moveSit"]'), null, 'no take_orders: no move');
  const moved = renderSitting(ctx({ caps: parseCaps('take_orders') }), s);
  assert.equal($(moved, '[data-act="moveSit"]').getAttribute('data-tour'), 'sitting.move');
  const paid = sit({ rounds: [{ id: 'a', status: 'READY', payment_status: 'paid' }] });
  assert.equal($(renderSitting(ctx({ caps: parseCaps('take_orders') }), paid), '[data-act="moveSit"]'), null, 'a paid sitting stays put');
});

const round = (o = {}) => ({ id: 'r1', status: 'CONFIRMED', seq: 4, subtotal: 2500, total: 2500, payment_status: 'unpaid',
  items: [{ name: 'Maki', product_id: 'p1', quantity: 2, unit_price: 500 }, { name: 'Tea', product_id: 'p2', quantity: 1, unit_price: 1500 }], ...o });

test('round: lines with quantity, remove and comp by caps; sums; add and pay', () => {
  const c = ctx({ caps: parseCaps('take_orders,take_payment,void') });
  const h = renderRound(c, round());
  assert.equal($$(h, 'li.line').length, 2);
  const less = $$(h, '[data-act="qty"]');
  assert.deepEqual(less.map(b => [b.getAttribute('data-line'), b.getAttribute('data-q')]), [['0', '1'], ['0', '3'], ['1', '0'], ['1', '2']]);
  assert.ok(less[2].disabled, 'a line of one cannot go to zero with "less"');
  assert.equal($$(h, '[data-act="ask"][data-op="remove"]').length, 2);
  assert.equal($$(h, '[data-act="ask"][data-op="comp"]').length, 2);
  assert.ok($(h, '[data-act="add"]') && $(h, '[data-act="pay"]'));
  assert.equal(primaries(h), 1, 'pay is the one main action; add is secondary beside it');
  assert.ok($(h, 'form[data-form="table"] input[name="table"]'));
  assert.ok($(h, '.sums .owed .money'));
  for (const a of ['round.line', 'round.less', 'round.more', 'round.remove', 'round.comp', 'round.add', 'round.pay', 'round.sums', 'round.tableField', 'round.tableMove'])
    assert.ok(tours(h).includes(a), a);
});

test('round: the reason panel is chips with the chosen one pressed; "other" opens a field', () => {
  const S = { caps: parseCaps('take_orders,void'), ask: { op: 'comp', line: 1, kind: 'other' } };
  const h = renderRound(ctx(S), round());
  const chips = $$(h, '.ask [data-act="reason"]');
  assert.equal(chips.length, 5);
  assert.equal($(h, '[data-act="reason"][data-r="other"]').getAttribute('aria-pressed'), 'true');
  assert.equal($(h, '[data-act="reason"][data-r="mistake"]').getAttribute('aria-pressed'), 'false');
  assert.ok($(h, 'form[data-form="other"] input[name="text"][maxlength="140"]'));
  assert.ok($(h, '[data-act="unask"]'));
  assert.ok($(h, '.ask [data-t="whyComp"]') || /«whyComp»/.test(h));
});

test('round: a comped line, a paid round, a discount; names escaped', () => {
  const r = round({ discount: 300, payment_status: 'paid', items: [{ name: XSS, quantity: 1, unit_price: 5, comped: true }] });
  const h = renderRound(ctx({ caps: parseCaps('take_orders,take_payment') }), r);
  assert.ok($(h, 'li.is-comped .ui-badge'));
  assert.ok($(h, '.owed [data-t="paidInFull"]'));
  assert.ok(/−/.test($(h, '.sums').textContent), 'the discount is a minus');
  assert.equal($(h, '[data-act="pay"]'), null, 'nothing to take on a paid round');
  assert.deepEqual(injected(h), []);
  assert.ok($(renderRound(ctx(), round({ items: [] })), '.ui-empty [data-t="noLines"]'));
});

test('guest bar: confirm is the success action, reject the danger one', () => {
  const g = { id: 'g', placed_by: 'guest', status: 'PENDING' };
  const h = renderGuestBar(ctx({ caps: parseCaps('take_orders') }), g);
  // (the shim reads a dot inside an attribute value as a class: anchors are compared by getAttribute)
  assert.equal($(h, '.guest-round .ui-btn--success[data-act="guestConfirm"]').getAttribute('data-tour'), 'guest.confirm');
  assert.equal($(h, '.guest-round .ui-btn--danger[data-act="guestReject"]').getAttribute('data-tour'), 'guest.reject');
  assert.equal(renderGuestBar(ctx({ caps: parseCaps('advance') }), g), '', 'the kitchen does not answer a guest');
  assert.ok($(h, '.guest-round [role="status"]'), 'the waiting round is announced as a status');
});

test('parts: a card is a button with the row classes; a pick group presses one; loading is labelled', () => {
  const b = actionRow({ title: XSS, cls: 'card x', disabled: true, attrs: { data: { act: 'sit', id: '1' } } });
  const el = render(b).root.children[0];
  assert.equal(el.tagName, 'BUTTON');
  assert.equal(el.getAttribute('type'), 'button');
  assert.ok(el.disabled && el.classList.contains('ui-row--action') && el.classList.contains('card'));
  assert.deepEqual(injected(b), []);
  const g = pickChips('method', 'M', ['cash', 'card'], 'card', v => v, 'pay.method');
  assert.equal($(g, '[data-v="card"]').getAttribute('aria-pressed'), 'true');
  assert.equal($(g, '[role="group"]').getAttribute('aria-label'), 'M');
  assert.equal($(backBar(), '[data-act="back"]').getAttribute('data-tour'), 'nav.back');
  assert.equal($(loading(t), '[aria-busy="true"]').getAttribute('aria-label'), '«loading»');
});
