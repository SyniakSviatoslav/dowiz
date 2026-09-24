// The room app's PURE rules: no DOM, no fetch, no storage. Everything here is
// imported by `room/*.test.mjs` and called for real, so the imports are
// RELATIVE ('../lib/...'), which resolves to the same `/lib/...` URL in the
// browser and to the same file under `node --test`.
//
// MONEY IS INTEGER MINOR UNITS and is DRAWN ONLY BY `lib/money.js`. Parsing a
// typed amount is here, and is string arithmetic on the currency's decimals
// (`DECIMALS`, generated from the kernel) -- never a float, never a `/ 100`.
import * as Money from '../lib/money.js';
import { DECIMALS, REFUSED } from '../lib/vocab.js';

/// THE one escaper is /lib/ui's (`core.js`); this name is kept for importers.
export { esc } from '../lib/ui/core.js';

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

/// `command::pay::refuses_payment`: the kernel's REFUSED set (money not kept)
/// plus REFUNDING (money on its way back). Built from `/lib/vocab.js` so the
/// `vocab` gate's rule holds: no hand copy of the status set.
const NO_PAYMENT = new Set([...REFUSED, 'REFUNDING']);

/// WHAT THIS PERSON IS SHOWN for one round. The server refuses the same things
/// (`command::amend::apply`, `pay::decide`); hiding them is so a waiter is not
/// offered a button whose only answer is "no".
/// The signer word a guest's round carries (`placer::GUEST`, `pay::GUEST`).
export const GUEST = 'guest';

/// A guest's round still waiting for the room (D9). Not on the bill and not
/// payable until a waiter confirms it: the server refuses the payment too
/// (`dowiz_hub::room::pay::decide`) and leaves it out of `sitting::bill`.
export const guestWaiting = r => r?.placed_by === GUEST && r?.status === 'PENDING';

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
    pay: caps.has('take_payment') && !NO_PAYMENT.has(round?.status) && !guestWaiting(round) && !paid && owed(round) > 0,
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
export const METHODS = ['cash', 'card', 'cheque', 'transfer', 'gift_card', 'wallet', 'other'];

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

/// `command::pay::settles`: what one recorded payment took off the bill --
/// its bill share plus the TIP it carried, which raised the total by itself.
export const settles = p => (Number(p?.amount_in_order_currency ?? p?.amount ?? 0) || 0) + (Number(p?.tip ?? 0) || 0);

// ── a tip at payment (OPERATIONAL-BLIND-SPOTS §2.3) ─────────────────────────

/// "KEEP THE CHANGE": the guest hands `handed` for `owedMinor` (both in the
/// bill's currency). The payment is the owed share, the rest is the tip --
/// the server raises the round's tip and total by it in the same `Paid`.
/// Null when the hand-over does not cover what is owed.
export function keepTheChange(handed, owedMinor) {
  if (!Number.isSafeInteger(handed) || !(owedMinor > 0) || handed < owedMinor) return null;
  return { amount: owedMinor, tip: handed - owedMinor };
}

/// The tip field, parsed: '' is no tip (0), anything else must read as
/// minor units of the bill's currency. Null = refuse before sending.
export function tipMinor(text, code) {
  if (String(text ?? '').trim() === '') return 0;
  return parseMinor(text, code);
}

/// A wallet payment names its wallet (`PayIn.wallet`), and only it does.
export const walletOk = (method, wallet) => method !== 'wallet' || String(wallet || '').trim().length > 0;

/// D13 (G6): what the wallet field sends. The customer's own code -- their
/// order token, three dot-joined parts -- goes as `wallet_token`, and staff
/// spend only the wallet it names; anything else is a wallet id, which only
/// the owner may name (`room/pay/whose.rs`).
export const walletField = v => {
  const s = String(v || '').trim();
  return s.split('.').length === 3 ? { wallet_token: s } : { wallet: s };
};

/// A wallet pays the bill's share only (`command::pay::decide`): its leg
/// debits `amount`, so a tip on it would settle money nobody paid.
export const walletTipOk = (method, tip) => method !== 'wallet' || !(tip > 0);

/// What a round still owes, in its order's currency. The payments list when
/// the round carries one (a pay answer's `order` does); the card's `paid`
/// otherwise.
export function owed(round) {
  if (!round || round.payment_status === 'paid') return 0;
  const paid = Array.isArray(round.payments) ? round.payments.reduce((a, p) => a + settles(p), 0) : Number(round.paid || 0);
  return Math.max(0, Number(round.total || 0) - paid);
}

/// `took_money`'s complement, for the bill: a refused round is not owed, and
/// neither is a guest's round nobody has confirmed (D9, `sitting::billed`).
export const sittingDue = s => (s?.rounds || []).filter(r => !REFUSED.has(r.status) && !guestWaiting(r)).reduce((a, r) => a + owed(r), 0);

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

// ── moving lines and tables (BLUEPRINT-POS-THE-ROOM §2.9) ───────────────────

/// `command::transfer::apply`: a round gives or takes lines only BEFORE the
/// kitchen, unpaid, and by someone who takes orders. There is no capability
/// that makes it legal after the pass (the server says why in its header).
export function canTransfer(caps, round) {
  return caps.has('take_orders') && stageOf(round?.status) === 'before' && round?.payment_status !== 'paid';
}

/// Every OTHER round in the room that may receive lines: any sitting, so a
/// guest who joins another table takes their dishes with them.
export function transferTargets(caps, sittings, fromId) {
  const out = [];
  for (const s of sittings || []) for (const r of s.rounds || []) {
    if (r.id !== fromId && canTransfer(caps, r)) out.push({ sitting: s, round: r });
  }
  return out;
}

/// The body `POST /staff/orders/:id/transfer` takes, or `{error}` naming the
/// i18n key of why it would be refused. Lines are indices into the source's
/// items AS THIS SCREEN SHOWED THEM; both versions travel so a stale screen
/// is refused rather than moving the wrong dish.
export function transferBody(loc, from, to, lines) {
  const n = Array.isArray(from?.items) ? from.items.length : 0;
  const picked = [...new Set((lines || []).map(Number))].filter(i => Number.isInteger(i) && i >= 0 && i < n).sort((a, b) => a - b);
  if (!to || to.id === from?.id) return { error: 'pickRound' };
  if (!picked.length) return { error: 'pickLines' };
  // A round with no lines left is a cancellation, not a move (room_rules).
  if (picked.length === n) return { error: 'notAllLines' };
  return { location_id: loc, to_order_id: to.id, from_base_seq: from.seq, to_base_seq: to.seq, lines: picked };
}

/// May this sitting be moved to another table? Every round still in the room
/// goes; a PAID one not yet served blocks it (`transfer::sitting`).
export function canMoveSitting(caps, sitting) {
  if (!caps.has('take_orders')) return false;
  const live = (sitting?.rounds || []).filter(r => stageOf(r.status) !== 'over');
  return live.length > 0 && !live.some(r => r.payment_status === 'paid');
}

/// The server's refusal, as the i18n key that says it in the waiter's
/// language, or null to show its own words. Matched on the server's fixed
/// phrases (`transfer.rs`, `room_rules.rs`, `transfer/sitting.rs`).
export function refusalKey(status, message) {
  const m = String(message || '');
  if (status === 409 && m.includes('changed while you were editing')) return 'changedReload';
  if (status === 409 && m.includes('the kitchen has the')) return 'kitchenHasIt';
  if (status === 409 && (m.includes('is paid') || m.includes('has been paid'))) return 'roundPaid';
  if (status === 400 && m.includes('is anywhere but table')) return 'alreadyThere';
  if (status === 409 && m.includes('no lines left')) return 'notAllLines';
  if (status === 404) return 'notHere';
  return null;
}

// ── floor states ────────────────────────────────────────────────────────────

/// The six states `GET /api/staff/floor` answers (`command::floor::FloorState`),
/// in the legend's order. Anything else the server sends is drawn as free.
export const FLOOR_STATES = ['free', 'booked', 'ordering', 'waiting', 'paying', 'dirty'];

/// May this signer tap a table to clear it? The cap is TakeOrders, the one the
/// route checks; only a dirty table is cleared. The server decides again.
export const canClear = (caps, state) => caps.has('take_orders') && state === 'dirty';
