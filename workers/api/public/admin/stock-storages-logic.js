// W-STORE's pure rules for the Storages and Raw fish & HACCP sheets: what a
// transfer, a storage count and a freezing record SEND, which rule a freezing
// meets (the hub's `stock::haccp::rule`, number for number), the export's
// path, and the per-device "deliveries go to" choice. No DOM, no fetch.

import * as C from './ingredients-calc.js';

/// The storages a venue starts with; `kitchen` is where every old record is.
export const BUILT_IN = ['kitchen', 'bar', 'freezer'];
export const DEFAULT = 'kitchen';

/// A storage's name: its own, else the built-in's word.
export const nameOf = (s, t) => (s?.name || (BUILT_IN.includes(s?.id) ? t('sto_' + s.id) : s?.id || ''));

/// The storages a transfer or a delivery may go to (not archived).
export const open = storages => (storages || []).filter(s => !s.archived);

/// The rows of one storage (`all`: every storage), from `GET /owner/stock`:
/// `{id, name, unit, qty, by: [[storage, qty]...], home}`, non-zero only.
export function rowsFor(supplies, store){
  const out = [];
  for (const s of supplies || []) {
    const by = Object.entries(s.byStore?.stores || {}).filter(([, q]) => q !== 0);
    const qty = store === 'all' ? by.reduce((a, [, q]) => a + q, 0) : (s.byStore?.stores || {})[store] || 0;
    if (qty !== 0) out.push({ id: s.id, name: s.name || s.id, unit: s.unit || 'g', qty, by, home: s.byStore?.home || DEFAULT });
  }
  return out.sort((a, b) => a.name.localeCompare(b.name));
}

/// A transfer's body, or `{error}` (a word key).
export function moveBody(item, raw, unit, from, to){
  if (!item) return { error: 'sto_item' };
  const qty = C.amount(raw, unit);
  if (qty == null || qty <= 0) return { error: 'required' };
  if (!from || !to || from === to) return { error: 'sto_to' };
  return { body: { item, qty, from, to } };
}

/// A count of ONE storage: `{lines, store}`; empty fields are not counted.
export function countBody(entries, store, unitOf){
  const lines = [];
  for (const [item, raw] of entries) {
    if (String(raw ?? '').trim() === '') continue;
    const v = C.amount(raw, unitOf(item));
    if (v == null || v < 0) return { error: item };
    lines.push({ item, observed: v });
  }
  return lines.length ? { body: { lines, store } } : { error: 'required' };
}

/// EU 853/2004 Annex III VIII, as the hub reads it: the word key of the rule
/// `temp` °C for `hours` meets. Integers only.
export function ruleKey(hours, temp){
  const h = Number(hours), c = Number(temp);
  if (!Number.isInteger(h) || !Number.isInteger(c)) return 'hc_ruleNone';
  if (c <= -20 && h >= 24) return 'hc_rule20';
  if (c <= -35 && h >= 15) return 'hc_rule35';
  return 'hc_ruleNone';
}

/// A freezing record's body, or `{error}`. `started` (W-STORE2): the venue's
/// local `yyyy-mm-ddThh:mm` the freezing began, optional; with it the hours
/// may be left empty -- the hub decides the rule by start..now.
export function freezeBody(item, lot, hours, temp, store, started = '', ended = ''){
  const st = String(started || '').trim(), en = String(ended || '').trim();
  const hs = String(hours ?? '').trim();
  const h = Number(hs), c = Number(String(temp).trim().replace(',', '.'));
  if (!item || !String(lot || '').trim()) return { error: 'hc_lotPick' };
  if (st && !LOCAL.test(st)) return { error: 'hc_started' };
  if (en && (!st || !LOCAL.test(en) || en <= st)) return { error: 'hc_ended' };
  const noHours = st && hs === '';
  if ((!noHours && (!Number.isInteger(h) || h < 1 || h > 2000)) || !Number.isInteger(c) || c < -80 || c > 0) return { error: 'required' };
  return { body: { item, lot: String(lot).trim(), ...(noHours ? {} : { hours: h }), tempC: c, ...(store ? { store } : {}), ...(st ? { started: st } : {}), ...(en ? { ended: en } : {}) } };
}

const LOCAL = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/;
/// Whole hours from a local start to `nowMs`, or to a local `ended` when
/// one is given (the preview; the hub decides).
export function hoursSince(started, nowMs, ended = ''){
  if (!LOCAL.test(String(started || ''))) return null;
  const t = new Date(started).getTime();
  const e = LOCAL.test(String(ended || '')) ? new Date(ended).getTime() : nowMs;
  return Number.isFinite(t) && Number.isFinite(e) ? Math.floor((e - t) / 3600000) : null;
}

/// W-STORE2: the kitchen stations a storage can serve (a dish's `station`).
export const STATIONS = ['kitchen', 'sushi', 'bar'];
/// The storage `station` is bound to, if any (`storages[].stations`).
export const boundTo = (storages, station) => (storages || []).find(s => (s.stations || []).includes(station));
/// Tapping a station on storage `store`: bind it here, or unbind it if it is.
export const bindBody = (station, store, here) => ({ station, store: here ? '' : store });

/// The export's API path for `kind` over local days `from`..`to` (yyyy-mm-dd).
export const exportPath = (kind, from, to) => `/owner/stock/haccp?kind=${encodeURIComponent(kind)}&from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`;

/// `yyyy-mm-dd` of `today` minus `days` (calendar arithmetic on the date, no zone).
export function daysBefore(today, days){
  const d = new Date(`${today}T12:00:00Z`);
  d.setUTCDate(d.getUTCDate() - days);
  return d.toISOString().slice(0, 10);
}

const KEY = 'dowiz.recvStore';
/// "Deliveries go to": a per-device convenience. Unset (or unreadable
/// storage) means the hub's rule: where the ingredient was last received.
export function recvStore(){
  try { return globalThis.localStorage?.getItem(KEY) || ''; } catch { return ''; }
}
export function setRecvStore(id){
  try { if (id) globalThis.localStorage?.setItem(KEY, id); else globalThis.localStorage?.removeItem(KEY); } catch { /* private window */ }
}
/// A receipt body with the chosen storage, unless it names one already.
export const withStore = (body, store = recvStore()) => (store && !body.store ? { ...body, store } : body);
