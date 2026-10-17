// W-STORE's pure rules for the Storages and Raw fish & HACCP sheets: what a
// transfer, a storage count and a freezing record SEND, which rule a freezing
// meets (the hub's `stock::haccp::rule`, number for number), the export's
// path, and the per-device "deliveries go to" choice. No DOM, no fetch.

import * as C from './ingredients-calc.js';
import { wireQty } from './qty-adapter.js';

/// The storages a venue starts with; `kitchen` is where every old record is.
export const BUILT_IN = ['kitchen', 'bar', 'freezer'];
export const DEFAULT = 'kitchen';

/// A storage's name: its own, else the built-in's word.
export const nameOf = (s, t) => (s?.name || (BUILT_IN.includes(s?.id) ? t('sto_' + s.id) : s?.id || ''));

/// The storages a transfer or a delivery may go to (not archived).
export const open = storages => (storages || []).filter(s => !s.archived);

const MICRO = 1000000;
/// The hub's exact decimal text ("0.75", "-0.05", "7") -> integer millionths. Never a float.
export function uqOf(text){
  const m = /^(-?)(\d+)(?:\.(\d{1,6}))?$/.exec(String(text ?? '').trim());
  if (!m) return 0;
  const n = Number(m[2]) * MICRO + Number(((m[3] || '') + '000000').slice(0, 6));
  return m[1] ? -n : n;
}
/// Integer millionths -> the short exact decimal the owner reads: 750000 -> "0.75".
export function showUq(uq){
  const neg = uq < 0 ? '-' : '', a = Math.abs(uq);
  const f = String(a % MICRO).padStart(6, '0').replace(/0+$/, '');
  return `${neg}${Math.floor(a / MICRO)}${f ? '.' + f : ''}`;
}
/// A supply's EXACT level per storage, `{storage: uq}`: the answer's `exact`
/// map (2026-10-17: fractions move between storages) when it has one, else the whole `stores`.
export function levelsUq(s){
  const ex = s?.byStore?.exact;
  if (ex) return Object.fromEntries(Object.entries(ex).map(([k, v]) => [k, uqOf(v)]));
  return Object.fromEntries(Object.entries(s?.byStore?.stores || {}).map(([k, v]) => [k, Math.round(Number(v) || 0) * MICRO]));
}
/// "0.75" -- `supply`'s exact level in `store`, as text.
export const levelText = (s, store) => showUq(levelsUq(s)[store] || 0);

/// The rows of one storage (`all`: every storage), from `GET /owner/stock`:
/// `{id, name, unit, qty, by: [[storage, qty]...], home}`, non-zero only.
/// Quantities are EXACT decimal TEXT ("0.75"): a storage holds fractions.
export function rowsFor(supplies, store){
  const out = [];
  for (const s of supplies || []) {
    const lv = levelsUq(s);
    const by = Object.entries(lv).filter(([, q]) => q !== 0);
    const uq = store === 'all' ? by.reduce((a, [, q]) => a + q, 0) : lv[store] || 0;
    if (uq !== 0) out.push({ id: s.id, name: s.name || s.id, unit: s.unit || 'g', qty: showUq(uq), by: by.map(([k, q]) => [k, showUq(q)]), home: s.byStore?.home || DEFAULT });
  }
  return out.sort((a, b) => a.name.localeCompare(b.name));
}

/// A transfer's body, or `{error}` (a word key). The quantity in the kitchen's
/// words (`canonQty`: "0,25 кг", "½ шт") read EXACTLY into milli-units and sent
/// as the integer when whole, else as a decimal STRING. `moveId`: one per Move
/// sheet, so a retried tap moves once.
export function moveBody(item, raw, unit, from, to, moveId = ''){
  if (!item) return { error: 'sto_item' };
  const m = C.milli(raw, unit);
  if (m == null || m <= 0) return { error: 'required' };
  if (!from || !to || from === to) return { error: 'sto_to' };
  return { body: { item, qty: wireQty(m), from, to, ...(moveId ? { moveId } : {}) } };
}

/// A fresh move id for one Move sheet (1..64 of A-Z a-z 0-9 _ -).
export const newMoveId = (now = Date.now(), rnd = Math.random) => `mv-${now.toString(36)}-${Math.floor(rnd() * 1e9).toString(36)}`;

/// A count of ONE storage: `{lines, store}`; empty fields are not counted.
/// A fraction is counted as typed ("0,75"), sent as a decimal string.
export function countBody(entries, store, unitOf){
  const lines = [];
  for (const [item, raw] of entries) {
    if (String(raw ?? '').trim() === '') continue;
    const v = C.milli(raw, unitOf(item));
    if (v == null || v < 0) return { error: item };
    lines.push({ item, observed: wireQty(v) });
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
