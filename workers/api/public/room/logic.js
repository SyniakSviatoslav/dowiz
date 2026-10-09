// THE ROOM APP'S PURE RULES (no DOM, fetch or storage; money only through lib/money.js). Why and how: crates/dowiz-canvas/HOST-LIBS.md, logic.js.
import * as Money from '../lib/money.js';
import { DECIMALS, REFUSED } from '../lib/vocab.js';

export const money = (amount, code, locale) => (code ? Money.format(amount, code, locale) : '—');

export const parseCaps = s => new Set(String(s || '').split(',').map(x => x.trim()).filter(Boolean));

export function stageOf(status) {
  if (status === 'PENDING' || status === 'CONFIRMED') return 'before';
  if (status === 'PREPARING' || status === 'READY') return 'kitchen';
  return 'over';
}

const NO_PAYMENT = new Set([...REFUSED, 'REFUNDING']);

export const GUEST = 'guest';

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
    remove: editable && orders && (stage === 'before' || voids),
    comp: editable && orders && voids,
    table: editable && orders,
    pay: caps.has('take_payment') && !NO_PAYMENT.has(round?.status) && !guestWaiting(round) && !paid && owed(round) > 0,
  };
}

export const canTill = caps => caps.has('open_till');

export const REASONS = ['mistake', 'guest_changed', 'unavailable', 'dropped', 'other'];
export const OTHER_MAX_CHARS = 140;

export function reasonWord(kind, text) {
  if (!REASONS.includes(kind)) return null;
  if (kind !== 'other') return kind;
  const t = String(text || '').trim();
  return t && [...t].length <= OTHER_MAX_CHARS ? 'other:' + t : null;
}

export const METHODS = ['cash', 'card', 'cheque', 'transfer', 'gift_card', 'wallet', 'other'];

export const TILL_CURRENCIES = ['ALL', 'EUR'];

const decimalsOf = code => DECIMALS[code] ?? 2;

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

export function minorToInput(minor, code) {
  if (!(code in DECIMALS)) return '';
  const d = decimalsOf(code);
  const s = String(Math.max(0, Math.trunc(minor || 0))).padStart(d + 1, '0');
  return d === 0 ? s : s.slice(0, -d) + '.' + s.slice(-d);
}

export function quoteOf(orderCur, paidCur) {
  if (orderCur === 'ALL') return { base: paidCur, quote: orderCur };
  if (paidCur === 'ALL') return { base: orderCur, quote: paidCur };
  return { base: paidCur, quote: orderCur };
}

export function parseRate(text) {
  const s = String(text ?? '').replace(/[\s ]/g, '').replace(',', '.');
  const m = /^(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) return null;
  const frac = m[2] || '';
  const n = BigInt(m[1] + frac), d = 10n ** BigInt(frac.length);
  return n > 0n ? { n, d } : null;
}

const halfUp = (num, den) => (2n * num + den) / (2n * den);

export function ratePpm(text, orderCur, paidCur) {
  if (orderCur === paidCur) return null;
  const r = parseRate(text);
  if (!r) return null;
  const { base } = quoteOf(orderCur, paidCur);
  const dO = 10n ** BigInt(decimalsOf(orderCur)), dP = 10n ** BigInt(decimalsOf(paidCur));
  const M = 1000000n;
  const ppm = paidCur === base
    ? halfUp(r.n * dO * M, r.d * dP)
    : halfUp(r.d * dO * M, r.n * dP);
  return ppm >= 1n && ppm <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(ppm) : null;
}

export function convertPpm(amount, ppm) {
  const v = (BigInt(amount) * BigInt(ppm) + 500000n) / 1000000n;
  return Number(v);
}

export function maxAmountFor(owedMinor, ppm) {
  if (!(owedMinor > 0) || !(ppm > 0)) return 0;
  const cap = (BigInt(owedMinor) + 1n) * 1000000n - 500001n;
  return Number(cap / BigInt(ppm));
}

export const settles = p => (Number(p?.amount_in_order_currency ?? p?.amount ?? 0) || 0) + (Number(p?.tip ?? 0) || 0);

export function keepTheChange(handed, owedMinor) {
  if (!Number.isSafeInteger(handed) || !(owedMinor > 0) || handed < owedMinor) return null;
  return { amount: owedMinor, tip: handed - owedMinor };
}

export function tipMinor(text, code) {
  if (String(text ?? '').trim() === '') return 0;
  return parseMinor(text, code);
}

export const walletOk = (method, wallet) => method !== 'wallet' || String(wallet || '').trim().length > 0;

export const walletField = v => {
  const s = String(v || '').trim();
  return s.split('.').length === 3 ? { wallet_token: s } : { wallet: s };
};

export const walletTipOk = (method, tip) => method !== 'wallet' || !(tip > 0);

export function owed(round) {
  if (!round || round.payment_status === 'paid') return 0;
  const paid = Array.isArray(round.payments) ? round.payments.reduce((a, p) => a + settles(p), 0) : Number(round.paid || 0);
  return Math.max(0, Number(round.total || 0) - paid);
}

export const sittingDue = s => (s?.rounds || []).filter(r => !REFUSED.has(r.status) && !guestWaiting(r)).reduce((a, r) => a + owed(r), 0);

export function slugOfHost(hostname, search) {
  const explicit = new URLSearchParams(search || '').get('s');
  if (explicit) return explicit;
  const host = String(hostname || '').toLowerCase();
  if (host.endsWith('.workers.dev') || host === 'localhost') return null;
  const labels = host.split('.');
  return labels.length > 2 && labels[0] !== 'www' ? labels[0] : null;
}

export function ageOf(ms) {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return { n: s, unit: 's' };
  if (s < 3600) return { n: Math.floor(s / 60), unit: 'm' };
  return { n: Math.floor(s / 3600), unit: 'h' };
}

export function canTransfer(caps, round) {
  return caps.has('take_orders') && stageOf(round?.status) === 'before' && round?.payment_status !== 'paid';
}

export function transferTargets(caps, sittings, fromId) {
  const out = [];
  for (const s of sittings || []) for (const r of s.rounds || []) {
    if (r.id !== fromId && canTransfer(caps, r)) out.push({ sitting: s, round: r });
  }
  return out;
}

export function transferBody(loc, from, to, lines) {
  const n = Array.isArray(from?.items) ? from.items.length : 0;
  const picked = [...new Set((lines || []).map(Number))].filter(i => Number.isInteger(i) && i >= 0 && i < n).sort((a, b) => a - b);
  if (!to || to.id === from?.id) return { error: 'pickRound' };
  if (!picked.length) return { error: 'pickLines' };
  if (picked.length === n) return { error: 'notAllLines' };
  return { location_id: loc, to_order_id: to.id, from_base_seq: from.seq, to_base_seq: to.seq, lines: picked };
}

export function canMoveSitting(caps, sitting) {
  if (!caps.has('take_orders')) return false;
  const live = (sitting?.rounds || []).filter(r => stageOf(r.status) !== 'over');
  return live.length > 0 && !live.some(r => r.payment_status === 'paid');
}

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

export const FLOOR_STATES = ['free', 'booked', 'ordering', 'waiting', 'paying', 'dirty'];

export const canClear = (caps, state) => caps.has('take_orders') && state === 'dirty';
