// THE DRAFT RULE (W-SESSION, operator TZ 7, 2026-10-12): what a person typed
// into a form and did not save is kept on THEIR device, under one registered
// key family, for seven days, so a session that ends mid-edit (a 401, a reload,
// a closed tab) costs nothing they typed. Pure: no DOM, no clock, no storage --
// `draft.js` is the browser half, and this file is what the tests prove.
//
// WHAT IS NEVER KEPT. A password, a one-time code, a card field, a hidden or
// file input, and the person's OWN identity fields (autocomplete name/tel/
// email/address: the storefront keeps those under their own registry rows).
// A field or a whole form opts out with `data-nodraft`.
//
// THE KEY NAMES THE FORM, NOT THE RECORD. The app, the venue, the screen and
// the SHAPE (the sorted field names) make the key. Which record is told apart
// by the BASE: a hash of the values the form opened with. A draft is restored
// only into a form that opens with the same values it was typed over, so a
// dish's draft never lands in another dish, and a "new" form (blank base)
// gets its own draft back. Privacy registry row: `prefix("dw_draft_", ...)`.

export const PREFIX = 'dw_draft_';
/// A week: longer than any session, shorter than a person forgets what they meant.
export const KEEP_MS = 7 * 24 * 60 * 60 * 1000;
/// Drafts kept per form shape (one per record edited); the oldest goes first.
export const MAX_PER_FORM = 12;

const SKIP_TYPES = new Set(['password', 'hidden', 'file', 'submit', 'button', 'reset', 'image', 'tel', 'email']);
const SKIP_AUTOCOMPLETE = /^(cc-|current-password|new-password|one-time-code|username|name$|given-name|family-name|email|tel|street-address|address-line|postal-code)/;

/// May this field's value be kept? `f` is { tag, type, autocomplete, nodraft }.
export function collectable(f = {}) {
  if (f.nodraft) return false;
  const tag = String(f.tag || 'input').toLowerCase();
  if (tag === 'textarea' || tag === 'select') return true;
  if (SKIP_TYPES.has(String(f.type || 'text').toLowerCase())) return false;
  if (f.autocomplete && SKIP_AUTOCOMPLETE.test(String(f.autocomplete).toLowerCase())) return false;
  return true;
}

// Only the separator and the escape itself are escaped: a slug cannot forge another key.
const seg = s => String(s ?? '').replace(/%/g, '%25').replace(/\//g, '%2F');
export const keyOf = (app, venue, scope, shape) => `${PREFIX}${seg(app)}/${seg(venue)}/${seg(scope)}/${seg(shape)}`;

/// FNV-1a over a string, as eight hex digits: stable, tiny, not a secret.
export function fnv(s) {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0; }
  return h.toString(16).padStart(8, '0');
}
export const shapeOf = names => fnv([...names].sort().join('\u0000'));
export const hashOf = obj => fnv(JSON.stringify(Object.keys(obj).sort().map(k => [k, obj[k]])));

/// The fields whose current value differs from the base, or null when none does.
export function diff(base, current) {
  const out = {};
  let any = false;
  for (const k of Object.keys(current)) if (current[k] !== base[k]) { out[k] = current[k]; any = true; }
  return any ? out : null;
}

/// A book is { [baseHash]: { at, fields } }. `remember` writes one entry and prunes.
export function remember(book, baseHash, fields, now) {
  const next = {};
  for (const [h, e] of Object.entries(book || {})) if (h !== baseHash && e && now - (e.at || 0) < KEEP_MS) next[h] = e;
  next[baseHash] = { at: now, fields };
  const keys = Object.keys(next).sort((a, b) => next[a].at - next[b].at);
  while (keys.length > MAX_PER_FORM) delete next[keys.shift()];
  return next;
}
export function forget(book, baseHash) {
  const next = { ...(book || {}) };
  delete next[baseHash];
  return next;
}
export function recall(book, baseHash, now) {
  const e = book && book[baseHash];
  if (!e || !e.fields || typeof e.fields !== 'object') return null;
  return now - (e.at || 0) < KEEP_MS ? e.fields : null;
}
/// What storage gave back, as a book; anything unreadable is an empty one.
export function parseBook(raw) {
  try {
    const v = JSON.parse(raw);
    return v && typeof v === 'object' && !Array.isArray(v) ? v : {};
  } catch { return {}; }
}
