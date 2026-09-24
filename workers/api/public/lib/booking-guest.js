// A guest's booking, the rules that are not visual. PURE: no DOM, no fetch,
// no storage, no clock -- `node public/lib/booking-guest.test.mjs` runs them.
//
// NO ACCOUNT, the same as an order: the guest gives a name and a phone, and
// the booking comes back with a token scoped to that ONE booking. The browser
// keeps the list (as it keeps the order history), and a link carries one
// booking to another phone. The hub decides everything else: whether the
// booking may be made, and -- through `next` on the detail -- whether it may
// still be cancelled. Nothing here restates the reservation FSM.

/// The hub's own floors (`booking/guest.rs`): a name, and a phone the venue
/// can call. Checked here only so the form can say so before sending.
export const PHONE_MIN_DIGITS = 8;
export const NAME_MAX = 80;
/// How many bookings this browser remembers.
export const KEEP = 20;

/// The form's early word, as i18n keys. The hub's refusal is still the truth.
export function guestErrors({ name, phone }) {
  const errs = [];
  const n = String(name || '').trim();
  if (!n) errs.push('bkNeedName');
  else if ([...n].length > NAME_MAX) errs.push('bkNameLong');
  if (String(phone || '').replace(/\D/g, '').length < PHONE_MIN_DIGITS) errs.push('bkNeedPhone');
  return errs;
}

/// The body `POST .../reservations` takes. A table is both halves or neither.
export function bookingBody({ party, slotMin, pick, name, phone, rid }) {
  const b = { party, slotMin, contactName: String(name || '').trim(), contactPhone: String(phone || '').trim(), requestId: rid };
  if (pick && pick.zone && Number.isInteger(pick.n)) { b.zoneId = pick.zone; b.tableN = pick.n; }
  return b;
}

/// The request key for a booking attempt: `prev` again while it names the
/// SAME request (slot, table, party) and has not succeeded, else a fresh one.
///
/// ONE KEY PER BOOKING, NOT PER TAP (audit D28). A fresh key on every tap
/// turned "the answer was lost, tap again" into a second table held under a
/// token nobody kept; the hub replays its first answer to a key it has seen.
/// The caller forgets `prev` (passes null) once a booking succeeded.
export function requestKey(prev, { slotMin, pick, party }, rand = Math.random) {
  const what = `bk_${slotMin}_${pick ? `${pick.zone}_${pick.n}` : 'any'}_${party}`;
  if (prev && String(prev).startsWith(`${what}_`)) return prev;
  return `${what}_${rand().toString(36).slice(2, 10)}`;
}

/// Remember a booking, newest first, one entry per id.
export function remember(list, entry, keep = KEEP) {
  if (!entry?.id || !entry?.t) return list || [];
  const rest = (list || []).filter(x => x && x.id !== entry.id);
  return [{ id: entry.id, t: entry.t, slug: entry.slug, slotMin: entry.slotMin ?? null, party: entry.party ?? null }, ...rest].slice(0, keep);
}
export const forget = (list, id) => (list || []).filter(x => x && x.id !== id);
export const ofVenue = (list, slug) => (list || []).filter(x => x && x.slug === slug);

/// The fragment that carries one booking: `#rsv=<id>&t=<token>`. A FRAGMENT,
/// never a query: it is not sent to the server, so it is not in any log.
export const linkHash = (id, token) => `#rsv=${encodeURIComponent(id)}&t=${encodeURIComponent(token)}`;
export function parseHash(hash) {
  const p = new URLSearchParams(String(hash || '').replace(/^#/, ''));
  const id = p.get('rsv'), t = p.get('t');
  return id && t && /^rsv_[0-9a-f]{16}$/.test(id) ? { id, t } : null;
}

/// May the guest still cancel? The hub says, in the detail's `next`.
export const canCancel = detail => Array.isArray(detail?.next) && detail.next.includes('CANCELLED_BY_GUEST');

/// A refusal's sentence: the hub answers plain text, or `{"error": ...}` when
/// the idempotency layer replays a recorded refusal.
export function refusalText(text) {
  const s = String(text || '').trim();
  try { const j = JSON.parse(s); if (j && typeof j.error === 'string') return j.error; } catch {}
  return s;
}

/// The i18n key for a booking's status as the guest reads it.
export const STATUS_KEY = {
  REQUESTED: 'bkStRequested', CONFIRMED: 'bkStConfirmed', SEATED: 'bkStSeated', COMPLETED: 'bkStCompleted',
  DECLINED: 'bkStDeclined', CANCELLED_BY_GUEST: 'bkStCancelled', CANCELLED_BY_VENUE: 'bkStCancelledVenue', NO_SHOW: 'bkStNoShow',
};
