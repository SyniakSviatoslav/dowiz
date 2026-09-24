// THE DELIVERY AREA, drawn by the owner (POST /api/owner/zones). Before this
// file the route existed and nothing in the console called it: a venue could
// only get an area by somebody posting JSON by hand, and `/api/public/reach`
// answered "unrestricted" for every venue that had none.
//
// WHAT IS SENT is exactly what `services/venue/zones.rs::set_zones` reads:
// `{ zones: [...] }` and nothing else (the struct is deny_unknown_fields, so
// the venue goes in the query, never the body). A circle is
// { kind: 'circle', lat, lon, radius_m } in integer micro-degrees and metres;
// `kind` is written first because the hub's reader splits on `{"kind":`.
// An EMPTY list is how the area is removed: every address is accepted again.
// Polygons drawn elsewhere are kept as they are and can only be removed here.
//
// PURE HALF (rowsOf, circleZone, bodyOf, render, readRows) takes values and
// returns values, so `zones.test.mjs` runs it in node; `open()` takes the
// console's core and i18n as arguments for the same reason.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console. No apostrophes inside the words below.
import { btn, field, iconBtn, rowDiv, empty } from './parts.js';

/// A circle's radius is refused past this: a typo of 300 for 3.00 km would
/// otherwise accept a whole country.
export const MAX_KM = 50;
/// The radius a new circle starts with.
export const DEFAULT_KM = 3;
const UDEG = 1e6;

export const WORDS = {
  sq: { deliveryArea: 'Zona e dorëzimit', zHint: 'Rrathë rreth lokalit. Një adresë jashtë çdo rrethi refuzohet para porosisë.',
        zNone: 'Asnjë zonë: pranohet çdo adresë.', zCircle: 'Rreth', zPolygon: 'Poligon', zPoints: 'pika',
        zLat: 'Gjerësia', zLng: 'Gjatësia', zKm: 'Rrezja (km)', zAdd: 'Shto rreth', zClear: 'Hiq zonën: dorëzo kudo',
        zBadCentre: 'Qendra nuk është e vlefshme: gjerësia -90..90, gjatësia -180..180.', zBadKm: 'Rrezja duhet të jetë mbi 0 dhe deri në 50 km.' },
  en: { deliveryArea: 'Delivery area', zHint: 'Circles around the venue. An address outside every circle is refused before the order.',
        zNone: 'No area: every address is accepted.', zCircle: 'Circle', zPolygon: 'Polygon', zPoints: 'points',
        zLat: 'Latitude', zLng: 'Longitude', zKm: 'Radius (km)', zAdd: 'Add a circle', zClear: 'Remove the area: deliver everywhere',
        zBadCentre: 'The centre is not valid: latitude -90..90, longitude -180..180.', zBadKm: 'The radius must be above 0 and at most 50 km.' },
  uk: { deliveryArea: 'Зона доставки', zHint: 'Кола навколо закладу. Адресу поза всіма колами відхиляють ще до замовлення.',
        zNone: 'Зони немає: приймається будь-яка адреса.', zCircle: 'Коло', zPolygon: 'Полігон', zPoints: 'точок',
        zLat: 'Широта', zLng: 'Довгота', zKm: 'Радіус (км)', zAdd: 'Додати коло', zClear: 'Прибрати зону: доставляти всюди',
        zBadCentre: 'Центр недійсний: широта -90..90, довгота -180..180.', zBadKm: 'Радіус має бути більше 0 і не більше 50 км.' },
};

/// Merge this file's words into the console's table (T[lang]).
export function install(T, langs){ for (const l of langs) Object.assign(T[l], WORDS[l]); }

/// The venue's own position, when it has one: the default centre.
export function centreOf(venue){
  const lat = Number(venue?.lat), lng = Number(venue?.lng);
  return venue && venue.lat != null && venue.lng != null && Number.isFinite(lat) && Number.isFinite(lng) ? { lat, lng } : null;
}

/// A new circle, centred on the venue when it has a position.
export function newRow(venue){
  const c = centreOf(venue);
  return { kind: 'circle', lat: c ? String(c.lat) : '', lng: c ? String(c.lng) : '', km: String(DEFAULT_KM) };
}

/// The stored list as editable rows. A circle becomes degrees and km; a
/// polygon is carried whole; anything else is what the hub ignores too.
export function rowsOf(zones){
  const out = [];
  for (const z of Array.isArray(zones) ? zones : []) {
    if (z && z.kind === 'circle') out.push({ kind: 'circle', lat: String(z.lat / UDEG), lng: String(z.lon / UDEG), km: String(z.radius_m / 1000) });
    else if (z && z.kind === 'polygon') out.push({ kind: 'polygon', zone: z });
  }
  return out;
}

const num = s => (String(s ?? '').trim() === '' ? NaN : Number(String(s).trim().replace(',', '.')));

/// One circle row as the hub's zone, or `{ err }` naming the words to show.
export function circleZone(row){
  const lat = num(row.lat), lng = num(row.lng), km = num(row.km);
  if (!Number.isFinite(lat) || !Number.isFinite(lng) || Math.abs(lat) > 90 || Math.abs(lng) > 180) return { err: 'zBadCentre' };
  if (!Number.isFinite(km) || km > MAX_KM || Math.round(km * 1000) < 1) return { err: 'zBadKm' };
  return { zone: { kind: 'circle', lat: Math.round(lat * UDEG), lon: Math.round(lng * UDEG), radius_m: Math.round(km * 1000) } };
}

/// The POST body for these rows, or `{ err }` for the first row that is wrong.
export function bodyOf(rows){
  const zones = [];
  for (const r of rows) {
    if (r.kind === 'polygon') { zones.push(r.zone); continue; }
    const c = circleZone(r);
    if (c.err) return { err: c.err };
    zones.push(c.zone);
  }
  return { body: { zones } };
}

/// The path, with the venue in the query (never the body; see the top).
export const zonesPath = loc => '/owner/zones?location_id=' + encodeURIComponent(loc || '');

function circleRow(r, i){
  return `<div class="zone-row grid3" data-i="${i}">
    ${field({ key: 'zLat', value: r.lat, inputmode: 'decimal', data: { f: 'lat' } })}
    ${field({ key: 'zLng', value: r.lng, inputmode: 'decimal', data: { f: 'lng' } })}
    ${field({ key: 'zKm', value: r.km, inputmode: 'decimal', data: { f: 'km' } })}
    ${iconBtn({ icon: 'trash', ariaKey: 'remove', data: { zdel: String(i) } })}</div>`;
}
function polygonRow(r, i){
  const n = Array.isArray(r.zone.points) ? r.zone.points.length : 0;
  return `<div class="zone-row" data-i="${i}">${rowDiv({ leading: '', title: { t: 'zPolygon' },
    sub: `<span class="mono">${n} <span data-t="zPoints"></span></span>`,
    trailing: iconBtn({ icon: 'trash', ariaKey: 'remove', data: { zdel: String(i) } }) })}</div>`;
}

/// The editor's body for these rows.
export function render(rows){
  const list = rows.length
    ? rows.map((r, i) => (r.kind === 'polygon' ? polygonRow(r, i) : circleRow(r, i))).join('')
    : empty('map-pin', { key: 'zNone' });
  return `<div id="zRows">${list}</div>
    <div class="btn-row">${btn({ id: 'zAdd', icon: 'plus', key: 'zAdd' })}${rows.length ? btn({ id: 'zClear', icon: 'trash', key: 'zClear' }) : ''}</div>
    <div class="btn-row">${btn({ id: 'zSave', variant: 'primary', icon: 'check', key: 'save' })}</div>`;
}

/// The rows as typed now: circles re-read from their inputs, polygons as held.
export function readRows(root, rows){
  return [...root.querySelectorAll('.zone-row')].map(el => {
    const r = rows[Number(el.dataset.i)];
    if (r.kind === 'polygon') return r;
    const v = f => el.querySelector(`[data-f="${f}"]`).value;
    return { kind: 'circle', lat: v('lat'), lng: v('lng'), km: v('km') };
  });
}

/// Open the editor. `core` is /admin/core.js, `i18n` /admin/i18n.js and
/// `reload` refreshes S.venue after a save; each defaults to the real module.
export async function open(deps = {}){
  const c = deps.core || await import('/admin/core.js');
  const i18n = deps.i18n || await import('/admin/i18n.js');
  const reload = deps.reload || (async () => (await import('/admin/app.js')).loadVenue());
  install(i18n.T, i18n.LANGS);
  const venue = c.S.venue || {};
  let rows = rowsOf(venue.deliveryZones);
  const head = '<p class="eyebrow" data-t="settings"></p><h2 data-t="deliveryArea"></h2><p class="muted small" data-t="zHint"></p>';
  const fail = e => c.toast(String(e.message || e));
  const save = async (el, list) => {
    const b = bodyOf(list);
    if (b.err) return c.toast(c.t(b.err));
    try {
      await c.busy(el, () => c.post(zonesPath(c.store.loc), b.body));
      c.toast(c.t('saved')); await reload(); c.closeSheet();
    } catch (e) { fail(e); }
  };
  const draw = () => {
    c.sheet(`${head}<div id="zBody">${render(rows)}</div>`, { name: 'zones', keepScroll: true });
    const body = c.$('#zBody');
    c.$('#zAdd').onclick = () => { rows = [...readRows(body, rows), newRow(venue)]; draw(); };
    const clear = c.$('#zClear');
    if (clear) clear.onclick = () => save(clear, []);
    c.$('#zSave').onclick = () => save(c.$('#zSave'), readRows(body, rows));
    for (const x of body.querySelectorAll('[data-zdel]')) {
      x.onclick = () => { const i = Number(x.dataset.zdel); rows = readRows(body, rows).filter((_, j) => j !== i); draw(); };
    }
  };
  draw();
}
