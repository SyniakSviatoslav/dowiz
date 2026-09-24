// The way home, on a quiet map: the venue, your door, and the courier moving
// between them. Nothing else -- a simplified basemap with the colour turned
// down so the three marks are what the eye finds.
//
// The map outlives the tracking sheet's re-render (the sheet is rebuilt on
// every poll); `keep()` lifts its element out and `mount()` puts it back, the
// same way the ocean canvas survives. The courier's dot glides between two
// polls rather than jumping, because a jump reads as a glitch and a glide
// reads as a rider.

import { state } from '/store/state.js';
import { t } from '/store/i18n.js';
import { loadMapLib, icon } from '/store/ui.js';

/// OpenFreeMap's least decorated style: pale roads, few labels.
const STYLE = 'https://tiles.openfreemap.org/styles/positron';
/// Micro-degrees to degrees, at the one boundary where a float is right.
const UDEG = 1e6;
/// Padding around the fitted marks, and the zoom the fit may not exceed.
const FIT_PADDING_PX = 48;
const FIT_MAX_ZOOM = 16;
/// The courier's glide between two positions.
const GLIDE_MS = 1200;
/// A statuses the map is shown for: from acceptance to the door.
const SHOWN = new Set(['CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']);
/// The straight line venue → door, as the simplified route.
const ROUTE_SOURCE = 'dw-route';

let m = null;   // { el, map, venue, door, courier, courierAt, fitted }

/// Where the venue is, from the public location; null when it has no pin.
function venueLngLat(){
  const lat = Number(state.loc?.lat), lng = Number(state.loc?.lng);
  return Number.isFinite(lat) && Number.isFinite(lng) && (lat || lng) ? [lng, lat] : null;
}
/// Where the door is, from the order; null for pickup or an unpinned address.
function doorLngLat(order){
  const a = order?.fulfilment?.address;
  if (!a || order?.fulfilment?.kind === 'pickup') return null;
  const lat = Number(a.lat_udeg), lon = Number(a.lon_udeg);
  return Number.isFinite(lat) && Number.isFinite(lon) && (lat || lon) ? [lon / UDEG, lat / UDEG] : null;
}
function courierLngLat(order){
  const c = order?.eta?.courierAt;
  if (!c) return null;
  const lat = Number(c.latUdeg), lon = Number(c.lonUdeg);
  return Number.isFinite(lat) && Number.isFinite(lon) ? [lon / UDEG, lat / UDEG] : null;
}

/// Whether this order, in this state, gets a map at all.
export function wanted(order){
  return SHOWN.has(order?.status) && !!doorLngLat(order) && !!venueLngLat();
}

/// The markup slot the sheet renders; the map is mounted into it afterwards.
export function markup(order){
  if (!wanted(order)) return '';
  const riding = order.status === 'IN_DELIVERY';
  return `<section class="ep-mapwrap" data-tour="track.map"><div class="ep-map" id="epMap" aria-label="${t('mapLegend')}"></div>
    <p class="ep-legend mono"><span><i class="tm-key tm-key-venue"></i>${t('mapVenue')}</span><span><i class="tm-key tm-key-door"></i>${t('mapYou')}</span>${riding ? `<span><i class="tm-key tm-key-courier"></i>${t('mapCourier')}</span>` : ''}</p></section>`;
}

/// Lift the live map out of a sheet about to be rebuilt.
export function keep(){ return m ? m.el : null; }

/// The slot and its legend, gone: a map that cannot draw is not a blank box.
function hideSlot(){ document.querySelector('.ep-mapwrap')?.remove(); }

function markerEl(kind, inner){
  const el = document.createElement('div');
  el.className = `tm ${kind}`;
  el.innerHTML = inner;
  return el;
}

/// Put the map into the freshly rendered slot, creating it the first time,
/// and move the courier to where the hub last saw them.
export async function mount(order, keptEl){
  const slot = document.getElementById('epMap');
  if (!slot) { destroy(); return; }
  const venue = venueLngLat(), door = doorLngLat(order), courier = courierLngLat(order);
  if (!venue || !door) { destroy(); return; }
  if (m && keptEl) {
    slot.replaceWith(keptEl);
    m.map.resize();
  } else if (!m) {
    let lib;
    try { lib = await loadMapLib(); } catch { return; }
    if (!document.getElementById('epMap')) return;
    // A phone without WebGL, or a WebGL that refuses this canvas, gets no map
    // and no error: the sheet keeps its words, the slot goes away.
    let map;
    try {
      map = new lib.Map({ container: 'epMap', style: STYLE, center: venue, zoom: 13, attributionControl: false, interactive: true, dragRotate: false, pitchWithRotate: false, failIfMajorPerformanceCaveat: false });
    } catch (e) { hideSlot(); return; }
    map.on('error', e => { if (!m || !m.map.loaded()) { try { map.remove(); } catch {} m = null; hideSlot(); } });
    map.addControl(new lib.AttributionControl({ compact: true }));
    map.touchZoomRotate.disableRotation();
    const venueM = new lib.Marker({ element: markerEl('tm-venue', `<b>${(state.loc?.stage?.seal || '').slice(0, 1) || icon('torii')}</b>`), anchor: 'center' }).setLngLat(venue).addTo(map);
    const doorM = new lib.Marker({ element: markerEl('tm-door', icon('home')), anchor: 'bottom' }).setLngLat(door).addTo(map);
    m = { el: document.getElementById('epMap'), map, lib, venueM, doorM, courierM: null, courierAt: null, fitted: false };
    map.on('load', () => {
      map.addSource(ROUTE_SOURCE, { type: 'geojson', data: { type: 'Feature', geometry: { type: 'LineString', coordinates: [venue, door] } } });
      map.addLayer({ id: ROUTE_SOURCE, type: 'line', source: ROUTE_SOURCE, paint: { 'line-color': '#b98a3a', 'line-width': 2, 'line-dasharray': [2, 2], 'line-opacity': .8 } });
      fit();
    });
  }
  ride(courier);
}

function fit(){
  if (!m) return;
  const pts = [m.venueM.getLngLat(), m.doorM.getLngLat()];
  if (m.courierM) pts.push(m.courierM.getLngLat());
  const b = pts.reduce((bb, p) => bb.extend(p), new m.lib.LngLatBounds(pts[0], pts[0]));
  m.map.fitBounds(b, { padding: FIT_PADDING_PX, maxZoom: FIT_MAX_ZOOM, duration: m.fitted ? 900 : 0 });
  m.fitted = true;
}

/// The courier's dot: created on the first fix, glided to each next one,
/// removed when the hub stops sending one.
function ride(to){
  if (!m) return;
  if (!to) { if (m.courierM) { m.courierM.remove(); m.courierM = null; m.courierAt = null; } return; }
  if (!m.courierM) {
    m.courierM = new m.lib.Marker({ element: markerEl('tm-courier', icon('bike')), anchor: 'center' }).setLngLat(to).addTo(m.map);
    m.courierAt = to;
    if (m.map.loaded()) fit();
    return;
  }
  const from = m.courierAt || to;
  if (from[0] === to[0] && from[1] === to[1]) return;
  const t0 = performance.now();
  const step = now => {
    if (!m || !m.courierM) return;
    const k = Math.min(1, (now - t0) / GLIDE_MS), e = 1 - Math.pow(1 - k, 3);
    m.courierM.setLngLat([from[0] + (to[0] - from[0]) * e, from[1] + (to[1] - from[1]) * e]);
    if (k < 1) requestAnimationFrame(step); else m.courierAt = to;
  };
  requestAnimationFrame(step);
  // Keep the rider in view without re-fitting on every poll: only when they
  // leave the frame.
  if (!m.map.getBounds().contains(to)) fit();
}

export function destroy(){
  if (!m) return;
  try { m.map.remove(); } catch {}
  m = null;
}
