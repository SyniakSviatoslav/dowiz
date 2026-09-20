// Where to deliver, chosen on the map -- with a waypoint, the way a game shows one.
//
// A courier needs a street and a door; a kernel needs a coordinate to say
// whether the door is inside the area and how far it is. This sheet gives both
// from one gesture: the customer drags a pin to their door, the coordinate is
// read off the pin, and the street is read back from OpenStreetMap's own
// geocoder so the fields fill themselves. The typed parts are still the
// customer's to edit -- a floor, a doorbell, a landmark -- and they are what
// the courier gets.
//
// THE HUD. Over the map floats a compass card: an arrow that points from the
// venue to the pin and turns as the pin moves, the distance, the bearing as a
// cardinal, the way a waypoint reads in a game. "Follow my heading" turns the
// map with the phone (DeviceOrientation, asked for on iOS), so the street on
// the screen lies the way the street in front of the customer does.
//
// OpenStreetMap, not Google: the map tiles are already OpenFreeMap, the policy
// already allows them, and Nominatim asks for nothing but a User-Agent and
// courtesy. One request per settled pin, never per drag frame.
//
// Coordinates travel as MICRO-DEGREE INTEGERS, rounded once here: the whole
// system holds them as integers and a float crossing into an order is the
// thing MANIFESTO C2 forbids.

import { state } from '/store/state.js';
import { t } from '/store/i18n.js';
import { $, esc, icon, sheet, closeSheet, toast, loadMapLib } from '/store/ui.js';

const NOMINATIM = 'https://nominatim.openstreetmap.org/reverse';
const TILES = 'https://tiles.openfreemap.org/styles/liberty';
/// Durrës, for a venue that has not said where it is.
const FALLBACK_CENTRE = { lat: 41.3153, lng: 19.445 };
/// A street is legible at 16; a door at 17.
const ZOOM_STREET = 16;
const ZOOM_DOOR = 17;
/// Nominatim's house-number zoom.
const REVERSE_ZOOM = 18;
/// The sheet rises for this long; the map is told its true size once it has.
const SHEET_RISE_MS = 450;
/// Micro-degrees: six decimals, the precision the order carries.
const MICRO = 1e6;
const GEO_OPTIONS = { enableHighAccuracy: true, timeout: 10_000, maximumAge: 60_000 };
/// The earth, for the distance between two doors.
const EARTH_RADIUS_M = 6_371_000;
/// Under a kilometre the distance reads in metres, rounded to tens.
const METRES_ROUND = 10;
const KM_FROM_M = 1000;
/// The eight winds, clockwise from north.
const WINDS = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
/// The map's bearing follows the phone no more often than this.
const HEADING_MIN_MS = 120;

/// A reverse lookup is cached by a FIFTY-METRE CELL, in this browser.
///
/// Nominatim asks for courtesy -- roughly a request a second -- and a customer
/// nudging the pin around their own building asks the same question of the same
/// doorway a dozen times. The cell is the unit of the answer: fifty metres is
/// one building, and two pins in the same cell get the same street.
///
/// NOT IN KV, and that is deliberate. The lookup happens in the BROWSER, so it
/// costs this platform nothing today; routing it through the Worker to cache it
/// would ADD a Worker request and two KV reads per pin in order to save
/// somebody else's free service. The browser is where the repetition is, so the
/// browser is where the cache belongs.
const CELL_LAT = 0.00045;          // ~50 m of latitude
const CELL_LNG = 0.0006;           // ~50 m of longitude at Durrës's latitude
const REVERSE_TTL_MS = 30 * 24 * 60 * 60 * 1000;
const REVERSE_PREFIX = 'dowiz.rev.';

function cellOf(lat, lng){
  return `${REVERSE_PREFIX}${Math.round(lat / CELL_LAT)},${Math.round(lng / CELL_LNG)}`;
}

function cached(key){
  // Storage can throw outright in a private window; a cache that cannot be
  // read is a cache miss, never a broken address sheet.
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const { t, v } = JSON.parse(raw);
    if (!t || Date.now() - t > REVERSE_TTL_MS) { localStorage.removeItem(key); return null; }
    return v;
  } catch { return null; }
}

function remember(key, value){
  try { localStorage.setItem(key, JSON.stringify({ t: Date.now(), v: value })); } catch { /* full or blocked */ }
}

async function reverse(lat, lng){
  const key = cellOf(lat, lng);
  const hit = cached(key);
  if (hit) return hit;
  try {
    const r = await fetch(`${NOMINATIM}?format=jsonv2&lat=${lat}&lon=${lng}&zoom=${REVERSE_ZOOM}&accept-language=${document.documentElement.lang || 'sq'}`,
      { headers: { accept: 'application/json' } });
    if (!r.ok) return null;
    const d = await r.json();
    const a = d.address || {};
    const street = a.road || a.pedestrian || a.footway || a.path || '';
    const house = a.house_number || '';
    const town = a.city || a.town || a.village || a.suburb || '';
    const line = [[street, house].filter(Boolean).join(' '), town].filter(Boolean).join(', ');
    const out = { street, house, town, line: line || d.display_name?.split(',').slice(0, 3).join(',') || '' };
    // Only a real answer is remembered: caching "nothing found" for thirty days
    // would make one bad lookup permanent for that doorway.
    if (out.line) remember(key, out);
    return out;
  } catch { return null; }
}

/// Distance in metres and initial bearing in degrees from A to B.
export function vector(a, b){
  const toRad = d => d * Math.PI / 180;
  const φ1 = toRad(a.lat), φ2 = toRad(b.lat), Δλ = toRad(b.lng - a.lng);
  const x = Math.sin(Δλ) * Math.cos(φ2);
  const y = Math.cos(φ1) * Math.sin(φ2) - Math.sin(φ1) * Math.cos(φ2) * Math.cos(Δλ);
  const bearing = (Math.atan2(x, y) * 180 / Math.PI + 360) % 360;
  const dφ = φ2 - φ1;
  const h = Math.sin(dφ / 2) ** 2 + Math.cos(φ1) * Math.cos(φ2) * Math.sin(Δλ / 2) ** 2;
  const metres = 2 * EARTH_RADIUS_M * Math.asin(Math.sqrt(h));
  return { metres, bearing };
}
export const windOf = bearing => WINDS[Math.round(bearing / (360 / WINDS.length)) % WINDS.length];
export const distanceText = m => m < KM_FROM_M ? `${Math.round(m / METRES_ROUND) * METRES_ROUND} m` : `${(m / KM_FROM_M).toFixed(1)} km`;

/// Opens the map sheet. Resolves to `{ line, street, house, lat, lng }` on confirm, or null.
export function pickOnMap({ initial = null } = {}){
  return new Promise(resolve => {
    const L = state.loc;
    const venue = Number.isFinite(L?.lat) ? { lat: L.lat, lng: L.lng } : null;
    const start = initial && Number.isFinite(initial.lat) ? initial : (venue || FALLBACK_CENTRE);
    let picked = { ...start }, found = { line: initial?.line || '', street: '', house: '' };
    let settled = false;
    const finish = v => { if (settled) return; settled = true; resolve(v); };

    sheet(`
      <div class="mapsheet">
        <p class="eyebrow" data-t="address"></p>
        <h2 data-t="pickOnMap"></h2>
        <p class="muted" data-t="dragPin"></p>
        <div class="pickmap" id="pickMap">
          ${venue ? `<div class="hud" id="hud" aria-live="polite">
            <span class="hud-arrow" id="hudArrow">${icon('navigation')}</span>
            <span class="hud-t"><b id="hudDist">—</b><small><span id="hudWind"></span> · <span data-t="fromVenue"></span></small></span>
            <button type="button" class="hud-btn" id="hudHeading" aria-pressed="false" data-t-attr="title:headingOn aria-label:headingOn">${icon('compass')}</button>
          </div>` : ''}
        </div>
        <div class="pickrow">
          <button type="button" class="btn btn-ghost" id="myLoc">${icon('current-location')}<span data-t="useMyLocation"></span></button>
        </div>
        <p class="geo" id="pickLine">${esc(found.line)}</p>
        <button class="btn" id="pickOk"><span data-t="confirmPin"></span>${icon('check')}</button>
      </div>`, { name: 'map' });

    const lineEl = $('#pickLine');
    const setLine = v => { found = v || { line: '', street: '', house: '' }; lineEl.textContent = found.line || ''; lineEl.className = 'geo' + (found.line ? ' ok' : ''); };
    const lookup = async () => {
      lineEl.textContent = t('findingAddress'); lineEl.className = 'geo';
      setLine(await reverse(picked.lat, picked.lng));
    };
    // the waypoint: from the venue to the pin, as an arrow, a distance and a wind
    let mapBearing = 0;
    const hud = () => {
      if (!venue) return;
      const v = vector(venue, picked);
      const arrow = $('#hudArrow'); if (arrow) arrow.style.transform = `rotate(${v.bearing - mapBearing}deg)`;
      const d = $('#hudDist'); if (d) d.textContent = distanceText(v.metres);
      const w = $('#hudWind'); if (w) w.textContent = windOf(v.bearing);
    };
    hud();

    (async () => {
      let maplibregl;
      try { maplibregl = await loadMapLib(); }
      catch (e) { $('#pickMap').textContent = `${t('loadFail')} · ${e.message}`; return; }
      const map = new maplibregl.Map({
        container: $('#pickMap'), style: TILES,
        center: [start.lng, start.lat], zoom: ZOOM_STREET, attributionControl: true,
      });
      map.addControl(new maplibregl.NavigationControl({ showCompass: true }), 'top-right');
      if (venue) new maplibregl.Marker({ color: '#888' }).setLngLat([venue.lng, venue.lat]).addTo(map);
      const pin = new maplibregl.Marker({ draggable: true }).setLngLat([start.lng, start.lat]).addTo(map);
      const moved = () => { const p = pin.getLngLat(); picked = { lat: p.lat, lng: p.lng }; hud(); };
      pin.on('drag', moved);
      pin.on('dragend', () => { moved(); lookup(); });
      // A tap places the pin where the finger landed; dragging the map does not.
      map.on('click', e => { picked = { lat: e.lngLat.lat, lng: e.lngLat.lng }; pin.setLngLat(e.lngLat); hud(); lookup(); });
      map.on('rotate', () => { mapBearing = map.getBearing(); hud(); });
      // The container was measured while the sheet was still rising.
      setTimeout(() => map.resize(), SHEET_RISE_MS);
      $('#myLoc').onclick = () => {
        if (!navigator.geolocation) return toast(t('noGeo'));
        navigator.geolocation.getCurrentPosition(pos => {
          picked = { lat: pos.coords.latitude, lng: pos.coords.longitude };
          pin.setLngLat([picked.lng, picked.lat]); map.easeTo({ center: [picked.lng, picked.lat], zoom: ZOOM_DOOR });
          hud(); lookup();
        }, () => toast(t('noGeo')), GEO_OPTIONS);
      };
      // Follow my heading: the map turns with the phone.
      const hb = $('#hudHeading');
      if (hb) {
        let following = false, lastAt = 0;
        const onHeading = ev => {
          const now = performance.now(); if (now - lastAt < HEADING_MIN_MS) return; lastAt = now;
          const heading = Number.isFinite(ev.webkitCompassHeading) ? ev.webkitCompassHeading
            : Number.isFinite(ev.alpha) ? (360 - ev.alpha) % 360 : null;
          if (heading === null) return;
          map.rotateTo(heading, { duration: HEADING_MIN_MS });
        };
        hb.onclick = async () => {
          following = !following;
          hb.setAttribute('aria-pressed', String(following));
          hb.title = t(following ? 'headingOff' : 'headingOn');
          if (!following) { removeEventListener('deviceorientation', onHeading); map.rotateTo(0); return; }
          try {
            if (typeof DeviceOrientationEvent?.requestPermission === 'function') {
              const ok = await DeviceOrientationEvent.requestPermission();
              if (ok !== 'granted') throw new Error('denied');
            }
            addEventListener('deviceorientation', onHeading);
          } catch { following = false; hb.setAttribute('aria-pressed', 'false'); toast(t('noGeo')); }
        };
      }
      if (!found.line) lookup();
    })();

    $('#pickOk').onclick = () => {
      finish({ ...found, lat: Math.round(picked.lat * MICRO) / MICRO, lng: Math.round(picked.lng * MICRO) / MICRO });
      closeSheet();
    };
    // Closing without confirming resolves null. The checkout re-opens itself.
    const onSheet = e => { if (e.detail && !e.detail.open && e.detail.name === 'map') { removeEventListener('dw:sheet', onSheet); finish(null); } };
    addEventListener('dw:sheet', onSheet);
  });
}
