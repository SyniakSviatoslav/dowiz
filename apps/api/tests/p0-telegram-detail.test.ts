import test from 'node:test';
import assert from 'node:assert/strict';
import { coarsenAddress, renderTelegramMessage } from '../src/notifications/render.js';
import type { NotificationData } from '../src/notifications/provider.js';

// P0-4 proof (ADR-p0-privacy-hardening): Telegram owner-alert body minimization.

test('coarsenAddress NEVER leaks a digit and fails closed on number-first streets', () => {
  // The load-bearing invariant: a coarsened address can never contain a house/building
  // number (any digit). Exact wording is best-effort.
  for (const a of [
    'Rruga Myslym Shyri, Pall. 5, Ap. 12',
    'Rruga e Durrësit 145',
    'Blloku, Rr. Ismail Qemali 34',
    'Njësia 7, Rruga A 12/3',
  ]) {
    const c = coarsenAddress(a);
    if (c !== undefined) assert.ok(!/\d/.test(c), `coarsened "${a}" must contain no digit, got "${c}"`);
  }
  assert.equal(coarsenAddress('Rr. 5 Maji'), undefined, 'street starting with a number → fail closed (no leak)');
  assert.equal(coarsenAddress('Rruga e Durrësit 145'), 'Rruga e Durrësit');
  assert.equal(coarsenAddress(undefined), undefined);
  assert.equal(coarsenAddress(''), undefined);
});

const base: NotificationData = {
  locationId: 'loc1', orderId: 'o1', shortOrderId: 'A1', total: 1500, currency: 'ALL',
  orderType: 'delivery', customerName: 'Test User', customerPhone: '+355691234567',
  deliveryAddress: 'Rruga Myslym Shyri, Pall. 5',
};

test("level 'full' → body carries the customer phone", () => {
  const { text } = renderTelegramMessage({ type: 'order.created' }, { ...base, alertDetail: 'full' }, 'sq');
  assert.ok(text.includes('+355691234567'), 'phone present at full detail');
});

test("level 'area' (default) → NO phone, NO house number, coarse street only", () => {
  const { text } = renderTelegramMessage({ type: 'order.created' }, { ...base, alertDetail: 'area' }, 'sq');
  assert.ok(!text.includes('+355691234567'), 'phone withheld at area detail');
  assert.ok(!text.includes('Pall. 5'), 'house/building number withheld at area detail');
});

test("level 'minimal' → no phone, no address at all", () => {
  const { text } = renderTelegramMessage({ type: 'order.created' }, { ...base, alertDetail: 'minimal' }, 'sq');
  assert.ok(!text.includes('+355691234567'), 'phone withheld');
  assert.ok(!text.includes('Myslym Shyri'), 'address fully withheld at minimal detail');
});

test('unset alertDetail defaults to area (privacy-preserving) — no phone leaks', () => {
  const { text } = renderTelegramMessage({ type: 'order.created' }, base, 'sq');
  assert.ok(!text.includes('+355691234567'), 'default omits phone (fail-safe to area)');
});

test('owner keeps an authenticated action path (in-app buttons) at area detail', () => {
  // For order.created the owner acts via authenticated Confirm/Reject buttons (bot-auth
  // callbacks scoped to the order) — so withholding the address from the BODY does not
  // strand the owner: full detail + actions remain in the authenticated app.
  const { reply_markup } = renderTelegramMessage({ type: 'order.created' }, { ...base, alertDetail: 'area' }, 'sq');
  const blob = JSON.stringify(reply_markup);
  assert.ok(/order\.confirm:o1/.test(blob) && /order\.reject/.test(blob), 'authenticated order-action buttons present');
});

test('a non-actionable event carries the authenticated owner-app deep-link (button url)', () => {
  const { text, reply_markup } = renderTelegramMessage({ type: 'order.timeout_cancelled' }, { ...base, alertDetail: 'area' }, 'sq');
  const blob = JSON.stringify({ text, reply_markup });
  assert.ok(/app\.dowiz\.org\/admin\/locations\/loc1\/orders\/o1/.test(blob), 'owner-app deep-link present (full detail behind auth link)');
});
