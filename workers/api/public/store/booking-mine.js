// A guest's own bookings: see them, share one, cancel one.
//
// NO ACCOUNT. Each booking came back with a token scoped to that ONE booking;
// this browser keeps the list (`dw_bookings`), the way it keeps the order
// history. A link (`#rsv=<id>&t=<token>`, a fragment, so it never reaches a
// server log) carries one booking to another phone.
//
// THE HUB DECIDES what may happen next: the detail answers `next`, and the
// cancel button is drawn exactly when it lists CANCELLED_BY_GUEST. Nothing
// here restates the reservation FSM.

import { API, SLUG, state, hhmm } from '/store/state.js';
import { t } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, toast } from '/store/ui.js';
import { safeGet, safeSet } from '/store/storage.js';
import { offsetMinutes } from '/lib/booking-time.js';
import { remember, forget, ofVenue, linkHash, parseHash, canCancel, refusalText, STATUS_KEY } from '/lib/booking-guest.js';
import '/store/booking-words.js';

const KEY = 'dw_bookings';
const read = () => { try { const l = JSON.parse(safeGet(KEY) || '[]'); return Array.isArray(l) ? l : []; } catch { return []; } };
const write = list => safeSet(KEY, JSON.stringify(list));
const base = `${API}/public/locations/${encodeURIComponent(SLUG)}/reservations`;

/// Keep a booking this browser just made (or was sent a link to).
export function keepBooking(entry) { write(remember(read(), { ...entry, slug: SLUG })); }
export const myBookings = () => ofVenue(read(), SLUG);

async function detail(b) {
  const r = await fetch(`${base}/${encodeURIComponent(b.id)}`, { headers: { authorization: 'Bearer ' + b.t } });
  // A booking whose token expired, or that another venue answers for, is
  // dropped from the list rather than shown as an error forever.
  if (r.status === 401 || r.status === 404) { write(forget(read(), b.id)); return { gone: true }; }
  if (!r.ok) throw new Error(refusalText(await r.text()));
  return r.json();
}

/// The slot as the venue's own clock reads it.
function when(slotMin) {
  const off = offsetMinutes(state.loc?.tz || 'Europe/Tirane', slotMin * 60_000);
  const local = slotMin + off;
  const d = new Date(local * 60_000);
  return `${d.getUTCDate()}.${String(d.getUTCMonth() + 1).padStart(2, '0')} ${hhmm(((local % 1440) + 1440) % 1440)}`;
}

function card(b, d) {
  if (!d) return `<div class="empty">${icon('alert-triangle')}<span data-t="bkLoadFail"></span></div>`;
  const table = d.tableN ? `${esc(t('bkTable'))} ${esc(d.tableN)}` : esc(t('bkAnyTable'));
  return `<article class="bk-mine" data-id="${esc(b.id)}">
    <p class="eyebrow">${esc(t(STATUS_KEY[d.status] || 'bkStRequested'))}</p>
    <h3>${esc(when(d.slotMin))} · ${esc(d.party)} ${esc(t('bkParty'))}</h3>
    <p class="muted small">${table} · ${esc(t('bkWho'))} ${esc(d.contactName || '')} · ${esc(t('bkRef'))} ${esc(b.id.slice(-6))}</p>
    <div class="bk-row">
      <button type="button" class="btn ghost" data-share="${esc(b.id)}">${icon('share')}<span data-t="bkShare"></span></button>
      ${canCancel(d) ? `<button type="button" class="btn ghost" data-cancel="${esc(b.id)}">${icon('x')}<span data-t="bkCancel"></span></button>` : ''}
    </div></article>`;
}

/// The sheet. `first` is a booking to put on top (the one just made).
export async function openMine(first = null) {
  sheet(`<div class="bk"><p class="eyebrow" data-t="bkRoom"></p><h2 data-t="bkMine"></h2><div class="skel skel-row"></div></div>`,
    { name: 'bookMine', full: true });
  const list = myBookings().sort((a, b) => (a.id === first ? -1 : b.id === first ? 1 : 0));
  const rows = await Promise.all(list.map(b => detail(b).then(d => [b, d], () => [b, null])));
  const shown = rows.filter(([, d]) => !d?.gone);
  sheet(`<div class="bk"><p class="eyebrow" data-t="bkRoom"></p><h2 data-t="bkMine"></h2>
    ${shown.length ? shown.map(([b, d]) => card(b, d)).join('') : `<div class="empty">${icon('clock')}<b data-t="bkMineNone"></b></div>`}</div>`,
    { name: 'bookMine', full: true, keepScroll: true });
  const root = $('#sheetIn');
  for (const btn of $$('[data-share]', root)) btn.onclick = () => share(btn.dataset.share);
  for (const btn of $$('[data-cancel]', root)) btn.onclick = () => cancel(btn.dataset.cancel, btn);
}

async function share(id) {
  const b = read().find(x => x.id === id); if (!b) return;
  const url = `${location.origin}${location.pathname}${location.search}${linkHash(b.id, b.t)}`;
  try { await navigator.clipboard.writeText(url); toast(t('bkCopied')); }
  catch { prompt(t('bkShare'), url); }
}

/// Two taps, not a dialog: the first arms the button, the second sends.
async function cancel(id, btn) {
  if (btn.dataset.armed !== '1') {
    btn.dataset.armed = '1';
    btn.querySelector('span').textContent = t('bkCancelSure');
    return;
  }
  const b = read().find(x => x.id === id); if (!b) return;
  btn.disabled = true;
  try {
    const r = await fetch(`${base}/${encodeURIComponent(id)}/action`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', authorization: 'Bearer ' + b.t },
      body: JSON.stringify({ to: 'CANCELLED_BY_GUEST' }),
    });
    if (!r.ok) throw new Error(refusalText(await r.text()));
    toast(t('bkCancelled'));
  } catch (e) {
    toast(`${t('bkFail')}: ${String(e.message || e).slice(0, 160)}`);
  }
  await openMine(id);
}

/// A booking link opened on this phone: keep it, clear the fragment (the
/// token must not linger in the address bar), and show it.
export function takeLinkFromHash() {
  const got = parseHash(location.hash);
  if (!got) return false;
  keepBooking({ id: got.id, t: got.t });
  history.replaceState(null, '', location.pathname + location.search);
  openMine(got.id);
  return true;
}
