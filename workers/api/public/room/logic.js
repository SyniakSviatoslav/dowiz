// The room app's PURE rules: no DOM, no fetch, no storage. Everything here is
// imported by `room/*.test.mjs` and called for real, so the imports are
// RELATIVE ('../lib/...'), which resolves to the same `/lib/...` URL in the
// browser and to the same file under `node --test`.
//
// MONEY IS INTEGER MINOR UNITS and is DRAWN ONLY BY `lib/money.js`. Parsing a
// typed amount is here, and is string arithmetic on the currency's decimals
// (`DECIMALS`, generated from the kernel) -- never a float, never a `/ 100`.
import * as Money from '../lib/money.js';
import { DECIMALS } from '../lib/vocab.js';

export const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

/// Draw an amount. The one door to `lib/money.js` for this app. A currency
/// not yet known draws a dash, NOT a guess: `money.js` would fall back to two
/// decimals, and 1500 lek drawn as "15.00" is the fourth-money-copy defect.
export const money = (amount, code, locale) => (code ? Money.format(amount, code, locale) : '—');

// ── who may do what ─────────────────────────────────────────────────────────

/// The token's `caps`: the canonical comma-joined spelling (`caps.rs` Display).
export const parseCaps = s => new Set(String(s || '').split(',').map(x => x.trim()).filter(Boolean));

/// `command::room_rules::stage`, the same three answers.
export function stageOf(status) {
  if (status === 'PENDING' || status === 'CONFIRMED') return 'before';
  if (status === 'PREPARING' || status === 'READY') return 'kitchen';
  return 'over';
}

/// `command::pay::refuses_payment`.
const NO_PAYMENT = new Set(['CANCELLED', 'REJECTED', 'REFUNDING', 'COMPENSATED_REFUND']);

/// WHAT THIS PERSON IS SHOWN for one round. The server refuses the same things
/// (`command::amend::apply`, `pay::decide`); hiding them is so a waiter is not
/// offered a button whose only answer is "no".
export function actionsFor(caps, round) {
  const stage = stageOf(round?.status);
  const paid = round?.payment_status === 'paid';
  const editable = stage !== 'over' && !paid;
  const orders = caps.has('take_orders');
  const voids = caps.has('void');
  return {
    add: editable && orders && stage === 'before',
    qty: editable && orders && stage === 'before',
    // After the pass a line leaves only with `void` (it was cooked).
    remove: editable && orders && (stage === 'before' || voids),
    // A comp is the void-holder's word at any stage.
    comp: editable && orders && voids,
    table: editable && orders,
    pay: caps.has('take_payment') && !NO_PAYMENT.has(round?.status) && !paid && owed(round) > 0,
  };
}

/// The till is the Counter-Manager's (`Cap::OpenTill`).
export const canTill = caps => caps.has('open_till');

/// `room_rules::VoidReason`, in order. `other` carries text.
export const REASONS = ['mistake', 'guest_changed', 'unavailable', 'dropped', 'other'];
export const OTHER_MAX_CHARS = 140;

/// The word the server parses, or null when it would refuse it.
export function reasonWord(kind, text) {
  if (!REASONS.includes(kind)) return null;
  if (kind !== 'other') return kind;
  const t = String(text || '').trim();
  return t && [...t].length <= OTHER_MAX_CHARS ? 'other:' + t : null;
}

/// `command::pay::validate_method`, exact.
export const METHODS = ['cash', 'card', 'cheque', 'transfer', 'gift_card', 'other'];

/// The two piles a Durrës drawer holds.
export const TILL_CURRENCIES = ['ALL', 'EUR'];

// ── amounts typed by a person ───────────────────────────────────────────────

const decimalsOf = code => DECIMALS[code] ?? 2;

/// "20", "20.5", "20,50", "1 950" -> minor units of `code`, or null. More
/// fractional digits than the currency has is refused, not rounded: 97.505
/// lek is a typing mistake, not an amount.
export function parseMinor(text, code) {
  if (!(code in DECIMALS)) return null;
  const s = String(text ?? '').replace(/[\s ]/g, '').replace(',', '.');
  const m = /^(\d+)(?:\.(\d*))?$/.exec(s);
  if (!m) return null;
  const d = decimalsOf(code);
  const frac = m[2] || '';
  if (frac.length > d) return null;
  const n = Number(m[1] + frac.padEnd(d, '0'));
  return Number.isSafeInteger(n) ? n : null;
}

/// Minor units back to what an input field holds ("20.00", "1950"). An INPUT
/// value, not a display: display is `money()`.
export function minorToInput(minor, code) {
  if (!(code in DECIMALS)) return '';
  const d = decimalsOf(code);
  const s = String(Math.max(0, Math.trunc(minor || 0))).padStart(d + 1, '0');
  return d === 0 ? s : s.slice(0, -d) + '.' + s.slice(-d);
}

// ── a payment in another currency (command/pay/fx.rs) ───────────────────────

/// THE HUMAN UNIT is the board by the till: "1 EUR = 97.50 ALL". The side
/// that is not lek is the one quoted per unit; between two non-lek
/// currencies, the one handed over is.
export function quoteOf(orderCur, paidCur) {
  if (orderCur === 'ALL') return { base: paidCur, quote: orderCur };
  if (paidCur === 'ALL') return { base: orderCur, quote: paidCur };
  return { base: paidCur, quote: orderCur };
}

/// "97.50" -> exact fraction {n, d} of BigInts, or null.
export function parseRate(text) {
  const s = String(text ?? '').replace(/[\s ]/g, '').replace(',', '.');
  const m = /^(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) return null;
  const frac = m[2] || '';
  const n = BigInt(m[1] + frac), d = 10n ** BigInt(frac.length);
  return n > 0n ? { n, d } : null;
}

const halfUp = (num, den) => (2n * num + den) / (2n * den);

/// `rate_ppm`: ORDER minor units per ONE PAYMENT minor unit, x 1 000 000,
/// rounded half-up, from the human quote. Exact integer arithmetic. Null when
/// the two currencies are the same (the server refuses a rate then) or the
/// text is not a rate.
export function ratePpm(text, orderCur, paidCur) {
  if (orderCur === paidCur) return null;
  const r = parseRate(text);
  if (!r) return null;
  const { base } = quoteOf(orderCur, paidCur);
  const dO = 10n ** BigInt(decimalsOf(orderCur)), dP = 10n ** BigInt(decimalsOf(paidCur));
  const M = 1000000n;
  // Paid IS the base: one paid major = R order majors.
  const ppm = paidCur === base
    ? halfUp(r.n * dO * M, r.d * dP)
    // Order is the base: one paid major = 1/R order majors.
    : halfUp(r.d * dO * M, r.n * dP);
  return ppm >= 1n && ppm <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(ppm) : null;
}

/// `fx::convert`, for the PREVIEW beside the field. The figure that counts is
/// the server's `amount_in_order_currency`, shown once it answers.
export function convertPpm(amount, ppm) {
  const v = (BigInt(amount) * BigInt(ppm) + 500000n) / 1000000n;
  return Number(v);
}

/// The most of `paidCur` that does not pay past `owedMinor` of the order's
/// currency at `ppm` (the server refuses Σ > total).
export function maxAmountFor(owedMinor, ppm) {
  if (!(owedMinor > 0) || !(ppm > 0)) return 0;
  const cap = (BigInt(owedMinor) + 1n) * 1000000n - 500001n;
  return Number(cap / BigInt(ppm));
}

// ── what is still owed ──────────────────────────────────────────────────────

/// `command::pay::settles`: what one recorded payment took off the bill.
export const settles = p => Number(p?.amount_in_order_currency ?? p?.amount ?? 0) || 0;

/// What a round still owes, in its order's currency. The payments list when
/// the round carries one (a pay answer's `order` does); the card's `paid`
/// otherwise.
export function owed(round) {
  if (!round || round.payment_status === 'paid') return 0;
  const paid = Array.isArray(round.payments) ? round.payments.reduce((a, p) => a + settles(p), 0) : Number(round.paid || 0);
  return Math.max(0, Number(round.total || 0) - paid);
}

/// `took_money`'s complement, for the bill: a refused round is not owed.
const UNBILLED = new Set(['REJECTED', 'CANCELLED', 'COMPENSATED_REFUND']);
export const sittingDue = s => (s?.rounds || []).filter(r => !UNBILLED.has(r.status)).reduce((a, r) => a + owed(r), 0);

// ── where, and how old ──────────────────────────────────────────────────────

/// The venue the address names -- the storefront's rule (`store/state.js`).
export function slugOfHost(hostname, search) {
  const explicit = new URLSearchParams(search || '').get('s');
  if (explicit) return explicit;
  const host = String(hostname || '').toLowerCase();
  if (host.endsWith('.workers.dev') || host === 'localhost') return null;
  const labels = host.split('.');
  return labels.length > 2 && labels[0] !== 'www' ? labels[0] : null;
}

/// An age in the largest whole unit: {n, unit: 's'|'m'|'h'}.
export function ageOf(ms) {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return { n: s, unit: 's' };
  if (s < 3600) return { n: Math.floor(s / 60), unit: 'm' };
  return { n: Math.floor(s / 3600), unit: 'h' };
}
