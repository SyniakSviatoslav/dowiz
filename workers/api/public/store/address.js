// Where to deliver, chosen on the map.
//
// A courier needs a street and a door; a kernel needs a coordinate to say
// whether the door is inside the area and how far it is. This sheet gives both
// from one gesture: the customer drags a pin to their door, the coordinate is
// read off the pin, and the street is read back from OpenStreetMap's own
// geocoder so the field fills itself. The typed line is still the customer's
// to edit -- a floor, a doorbell, a landmark -- and it is what the courier gets.
//
// OpenStreetMap, not Google: the map tiles are already OpenFreeMap, the policy
// already allows them, and Nominatim asks for nothing but a User-Agent and
// courtesy. One request per confirmed pin, never per drag frame.
//
// Coordinates travel as MICRO-DEGREE INTEGERS, rounded once here: the whole
// system holds them as integers and a float crossing into an order is the
// thing MANIFESTO C2 forbids.

import { state } from '/store/state.js';
import { t } from '/store/i18n.js';
import { $, esc, icon, sheet, closeSheet, toast } from '/store/ui.js';

const NOMINATIM = 'https://nominatim.openstreetmap.org/reverse';

async function reverse(lat, lng){
  try {
    const r = await fetch(`${NOMINATIM}?format=jsonv2&lat=${lat}&lon=${lng}&zoom=18&accept-language=${document.documentElement.lang || 'sq'}`,
      { headers: { accept: 'application/json' } });
    if (!r.ok) return null;
    const d = await r.json();
    const a = d.address || {};
    const street = [a.road || a.pedestrian || a.footway || a.path, a.house_number].filter(Boolean).join(' ');
    const town = a.city || a.town || a.village || a.suburb || '';
    const line = [street, town].filter(Boolean).join(', ');
    return line || d.display_name?.split(',').slice(0, 3).join(',') || null;
  } catch { return null; }
}

/// Opens the map sheet. Resolves to `{ line, lat, lng }` on confirm, or null.
export function pickOnMap({ initial = null } = {}){
  return new Promise(resolve => {
    const L = state.loc;
    const start = initial && Number.isFinite(initial.lat) ? initial
      : (Number.isFinite(L?.lat) ? { lat: L.lat, lng: L.lng } : { lat: 41.3153, lng: 19.445 });
    let picked = { ...start }, line = initial?.line || '';
    let settled = false;
    const finish = v => { if (settled) return; settled = true; resolve(v); };

    sheet(`
      <div class="mapsheet">
        <p class="eyebrow" data-t="address"></p>
        <h2 data-t="pickOnMap"></h2>
        <p class="muted" data-t="dragPin"></p>
        <div class="pickmap" id="pickMap"></div>
        <div class="pickrow">
          <button type="button" class="btn btn-ghost" id="myLoc">${icon('current-location')}<span data-t="useMyLocation"></span></button>
        </div>
        <p class="geo" id="pickLine">${esc(line)}</p>
        <button class="btn" id="pickOk"><span data-t="confirmPin"></span>${icon('check')}</button>
      </div>`, { name: 'map' });

    const lineEl = $('#pickLine');
    const setLine = v => { line = v || ''; lineEl.textContent = line || ''; lineEl.className = 'geo' + (line ? ' ok' : ''); };
    const lookup = async () => {
      lineEl.textContent = t('findingAddress'); lineEl.className = 'geo';
      const v = await reverse(picked.lat, picked.lng);
      setLine(v);
    };

    (async () => {
      let maplibregl;
      try { ({ default: maplibregl } = await import('/lib/map/maplibre-gl.js')); }
      catch { $('#pickMap').textContent = t('loadFail'); return; }
      const map = new maplibregl.Map({
        container: $('#pickMap'), style: 'https://tiles.openfreemap.org/styles/liberty',
        center: [start.lng, start.lat], zoom: 16, attributionControl: true,
      });
      map.addControl(new maplibregl.NavigationControl({ showCompass: false }), 'top-right');
      if (Number.isFinite(L?.lat)) new maplibregl.Marker({ color: '#888' }).setLngLat([L.lng, L.lat]).addTo(map);
      const pin = new maplibregl.Marker({ draggable: true }).setLngLat([start.lng, start.lat]).addTo(map);
      pin.on('dragend', () => { const p = pin.getLngLat(); picked = { lat: p.lat, lng: p.lng }; lookup(); });
      // A tap places the pin where the finger landed; dragging the map does not.
      map.on('click', e => { picked = { lat: e.lngLat.lat, lng: e.lngLat.lng }; pin.setLngLat(e.lngLat); lookup(); });
      $('#myLoc').onclick = () => {
        if (!navigator.geolocation) return toast(t('noGeo'));
        navigator.geolocation.getCurrentPosition(pos => {
          picked = { lat: pos.coords.latitude, lng: pos.coords.longitude };
          pin.setLngLat([picked.lng, picked.lat]); map.easeTo({ center: [picked.lng, picked.lat], zoom: 17 });
          lookup();
        }, () => toast(t('noGeo')), { enableHighAccuracy: true, timeout: 10000, maximumAge: 60000 });
      };
      if (!line) lookup();
    })();

    $('#pickOk').onclick = () => {
      finish({ line, lat: Math.round(picked.lat * 1e6) / 1e6, lng: Math.round(picked.lng * 1e6) / 1e6 });
      closeSheet();
    };
    // Closing without confirming resolves null. The checkout re-opens itself.
    const onSheet = e => { if (e.detail && !e.detail.open && e.detail.name === 'map') { removeEventListener('dw:sheet', onSheet); finish(null); } };
    addEventListener('dw:sheet', onSheet);
  });
}
