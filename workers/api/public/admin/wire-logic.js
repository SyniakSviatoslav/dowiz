// The W-WIRE screens' rules, PURE: text in, numbers out, no DOM, no network,
// no clock -- so each is tested in node (`wire-logic.test.mjs`).
//
// MONEY IS INTEGER MINOR UNITS and a rate is parts per million: nothing here
// multiplies a decimal. A typed "12.50" is read as digits, never as a float.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { venueWallMs, venueWallValue } from '../lib/booking-time.js';

/// Digits typed by a person as integer minor units, with `decimals` places
/// (lek 0, euro 2). In a currency with no decimals a thousands mark is
/// allowed ("1.500", "1 500"); in one with decimals, one decimal mark and no
/// thousands mark, so "12.500" is refused rather than guessed. A sign, a
/// second mark or more places than the currency has is `null`.
export function parseMinor(text, decimals = 0){
  const s = String(text ?? '').trim().replace(/[\s\u00a0']/g, '');
  if (decimals === 0) {
    if (!/^(\d{1,3}(?:[.,]\d{3})+|\d+)$/.test(s)) return null;
    const n = Number(s.replace(/[.,]/g, ''));
    return Number.isSafeInteger(n) ? n : null;
  }
  const m = new RegExp(`^(\\d+)(?:[.,](\\d{1,${decimals}}))?$`).exec(s);
  if (!m) return null;
  const n = Number(m[1]) * 10 ** decimals + Number((m[2] || '').padEnd(decimals, '0'));
  return Number.isSafeInteger(n) ? n : null;
}

/// A percentage typed by a person ("20", "6,5") as parts per million; at
/// most four decimal places, 0 to 100. `null` when it is not one.
export function pctToPpm(text){
  const s = String(text ?? '').trim().replace(',', '.');
  const m = /^(\d{1,3})(?:\.(\d{1,4}))?$/.exec(s);
  if (!m) return null;
  const ppm = Number(m[1]) * 10_000 + Number((m[2] || '').padEnd(4, '0'));
  return ppm <= 1_000_000 ? ppm : null;
}

/// Parts per million as the percentage a person reads: 200000 -> "20", 65000 -> "6.5".
export function ppmToPct(ppm){
  const n = Number(ppm);
  if (!Number.isSafeInteger(n) || n < 0) return '';
  const whole = Math.floor(n / 10_000), frac = String(n % 10_000).padStart(4, '0').replace(/0+$/, '');
  return frac ? `${whole}.${frac}` : String(whole);
}

/// `yyyy-mm-dd` as the epoch ms of that day's midnight IN THE VENUE'S ZONE
/// `tz` (an IANA name, `S.venue.tz`) -- never the phone's (`venue-clock` gate);
/// `null` if it is not a real day.
export function dayToMs(day, tz){
  const s = String(day ?? '').trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return null;
  const ms = venueWallMs(tz, `${s}T00:00`);
  return ms != null && msToDay(ms, tz) === s ? ms : null;
}

/// Epoch ms as the venue-local `yyyy-mm-dd` a date input shows.
export function msToDay(ms, tz){
  const n = Number(ms);
  if (!Number.isFinite(n) || n <= 0) return '';
  return venueWallValue(tz, n).slice(0, 10);
}

/// The orders placed from the venue's midnight starting `from` to the one
/// ending `to` (either may be empty = open), newest first. The end is the NEXT
/// day's midnight, never `+ 24 h`: a day is 23, 24 or 25 hours long.
export function inRange(orders, from, to, tz){
  const lo = from ? dayToMs(from, tz) : null, top = to ? dayToMs(to, tz) : null;
  const hi = top == null ? null : dayToMs(msToDay(top + 36 * 3_600_000, tz), tz);
  return (orders || [])
    .filter(o => { const at = Number(o.created_at_ms) || 0; return (lo == null || at >= lo) && (hi == null || at < hi); })
    .sort((a, b) => (Number(b.created_at_ms) || 0) - (Number(a.created_at_ms) || 0));
}

/// Sum a list of money amounts (integer minor units).
export const total = orders => (orders || []).reduce((s, o) => s + (Number.isSafeInteger(o.total) ? o.total : 0), 0);

/// An archive id (`log@<generation>`) as its number, for ordering; newest first.
export const archivesNewestFirst = ids => [...(ids || [])].sort((a, b) => (Number(String(b).split('@')[1]) || 0) - (Number(String(a).split('@')[1]) || 0));

/// The seq a venue READ mark should carry: the last customer message.
export function readThrough(messages){
  return (messages || []).filter(m => m.from === 'CUSTOMER' && m.kind === 'TEXT').reduce((n, m) => Math.max(n, Number(m.seq) || 0), 0);
}

/// Opaque RGBA pixels as the flat hex RGB the hub's `branding/extract` reads;
/// a transparent pixel is skipped (a logo's background is not its colour).
export function pixelsHex(rgba){
  let out = '';
  for (let i = 0; i + 3 < rgba.length; i += 4) {
    if (rgba[i + 3] < 128) continue;
    for (let c = 0; c < 3; c++) out += rgba[i + c].toString(16).padStart(2, '0');
  }
  return out;
}

/// The scheduled VAT changes as rows a person edits, and back: the stored
/// `[{since_ms, ppm}]` <-> `[{day, pct}]`. A row with a bad day or rate is
/// named, never dropped.
export function scheduleRows(json, tz){
  let list = [];
  try { list = JSON.parse(json || '[]'); } catch { return []; }
  return Array.isArray(list) ? list.map(r => ({ day: msToDay(r.since_ms, tz), pct: ppmToPct(r.ppm) })) : [];
}
export function scheduleJson(rows, tz){
  const out = [], bad = [];
  (rows || []).forEach((r, i) => {
    if (!String(r.day || '').trim() && !String(r.pct || '').trim()) return;
    const since = dayToMs(r.day, tz), ppm = pctToPpm(r.pct);
    if (since == null || ppm == null) bad.push(i + 1); else out.push({ since_ms: since, ppm });
  });
  return { json: out.length ? JSON.stringify(out) : '', bad };
}

/// A request id for an idempotent write: stable for one press, new for the next.
export const requestId = (prefix, now, salt) => `${prefix}-${Number(now).toString(36)}-${String(salt).replace(/[^a-z0-9]/gi, '').slice(0, 12)}`;
