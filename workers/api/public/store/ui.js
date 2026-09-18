// The Sheet's primitives: the one bottom sheet, the toast, escaping, icons,
// the deliberate mark for a dish with no photo, the map library loaded once,
// and the two motions the design language allows -- a sheet that RISES over
// the Sea (never slides in from the side) and a set of cards that SPREAD in
// with a short stagger.
//
// Money never passes through here. It is formatted in state.js from an integer
// and written as text; there is no animated path to a price anywhere.

import { t, retranslate } from '/store/i18n.js';
import { repaintMoney, state } from '/store/state.js';

export const $ = (s, r = document) => r.querySelector(s);
export const $$ = (s, r = document) => [...r.querySelectorAll(s)];
export const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
export const icon = (name, cls = '') => `<i class="ti ti-${name} ${cls}" aria-hidden="true"></i>`;
export const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

/// How long a toast stays: long enough to read a dish name and a quantity,
/// short enough not to cover the next tap.
const TOAST_MS = 2600;
/// A drag on the grip past this many pixels dismisses the sheet; less is a
/// wobble the finger did not mean.
const SHEET_DISMISS_PX = 90;
/// SPREAD: DESIGN.md §6 names it -- fadeSlideUp, one short step per child,
/// capped so a section of thirty cards does not take two seconds to settle.
const SPREAD_STEP_MS = 40;
const SPREAD_CAP_MS = 480;
/// The map library, self-hosted. It is a UMD bundle: it defines a global and
/// has no `default` export, so it is loaded as a classic script, once.
const MAP_JS = '/lib/map/maplibre-gl.js';
const MAP_CSS = '/lib/map/maplibre-gl.css';

// ── the sheet ───────────────────────────────────────────────────────────────
// ONE sheet, reused. It rises with the tide easing, wears the spectral rim,
// and closes on the scrim, on Escape, and on a drag down from the grip.
// `name` tells the bottom navigation which tab to light and lets a re-open
// (a cart quantity change re-drawing the cart) keep the sheet's scroll.
let onClose = null;
export function sheet(html, { name = null, keepScroll = false, attending = false, full = false } = {}){
  const box = $('#sheet'), inner = $('#sheetIn');
  const wasOpen = box.classList.contains('show');
  const top = keepScroll && wasOpen ? box.scrollTop : 0;
  inner.innerHTML = html;
  paintFallbacks(inner);
  retranslate(inner);
  repaintMoney(inner);
  box.classList.toggle('attending', attending);
  box.classList.toggle('full', full);
  box.dataset.name = name || '';
  box.classList.add('show');
  $('#scrim').classList.add('show');
  document.body.classList.add('sheet-open');
  box.scrollTop = top;
  dispatchEvent(new CustomEvent('dw:sheet', { detail: { open: true, name } }));
}
export function closeSheet(){
  const box = $('#sheet');
  if (!box.classList.contains('show')) return;
  box.classList.remove('show', 'attending', 'full');
  $('#scrim').classList.remove('show');
  document.body.classList.remove('sheet-open');
  const fn = onClose; onClose = null;
  dispatchEvent(new CustomEvent('dw:sheet', { detail: { open: false, name: box.dataset.name || null } }));
  box.dataset.name = '';
  if (fn) fn();
}
export const isSheetOpen = () => $('#sheet').classList.contains('show');
export const sheetName = () => $('#sheet').dataset.name || null;
/// Run once, the next time the sheet closes. Used by checkout to wake the Sea.
export function whenSheetCloses(fn){ onClose = fn; }

export function bindSheetChrome(){
  $('#scrim').onclick = closeSheet;
  addEventListener('keydown', e => { if (e.key === 'Escape') closeSheet(); });
  // Drag down from the grip to dismiss -- the gesture every phone teaches. The
  // sheet follows the finger from the grip only, so scrolling the content never
  // fights with closing it.
  const box = $('#sheet'), grip = $('#grab');
  let y0 = null, dy = 0;
  grip.addEventListener('pointerdown', e => { y0 = e.clientY; dy = 0; box.classList.add('dragging'); grip.setPointerCapture(e.pointerId); });
  grip.addEventListener('pointermove', e => {
    if (y0 === null) return;
    dy = Math.max(0, e.clientY - y0);
    box.style.transform = `translateY(${dy}px)`;
  });
  const end = () => {
    if (y0 === null) return;
    box.classList.remove('dragging');
    box.style.transform = '';
    if (dy > SHEET_DISMISS_PX) closeSheet();
    y0 = null; dy = 0;
  };
  grip.addEventListener('pointerup', end);
  grip.addEventListener('pointercancel', end);
}

// ── toast ───────────────────────────────────────────────────────────────────
export function toast(msg){
  const el = $('#toast'); el.textContent = msg; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), TOAST_MS);
}

// ── a dish with no photo ────────────────────────────────────────────────────
// THE VENUE'S OWN MARK, not a grey rectangle and not a letter. A dish the
// venue has not photographed yet is still this venue's dish, and the customer
// should see whose it is. Only a venue with no mark at all gets the hashed
// gradient: the hue is a stable hash of the name so the same dish always
// looks the same, and it goes through CSSOM because `style-src 'self'`
// forbids the attribute.
export function fallbackArt(name){
  const logo = state.loc?.logoUrl;
  if (logo) return `<div class="fallback logo" aria-hidden="true"><img src="${esc(logo)}" alt="" loading="lazy" decoding="async"></div>`;
  let h = 0; for (const ch of String(name)) h = (h * 31 + ch.codePointAt(0)) >>> 0;
  return `<div class="fallback" data-hue="${h % 360}" aria-hidden="true">${esc(String(name).trim()[0] || '·')}</div>`;
}
export function paintFallbacks(root){
  for (const el of (root || document).querySelectorAll('.fallback[data-hue]')){
    const hue = Number(el.dataset.hue);
    el.style.background = `linear-gradient(140deg,hsl(${hue} 46% 48%),hsl(${(hue + 38) % 360} 52% 32%))`;
    el.removeAttribute('data-hue');
  }
}
// A photo that 404s swaps itself for the mark. `error` does not bubble but it
// does capture, so one listener covers every image on the page.
document.addEventListener('error', e => {
  const img = e.target;
  if (!(img instanceof HTMLImageElement) || !img.dataset.fb) return;
  const holder = img.parentNode; if (!holder) return;
  holder.innerHTML = fallbackArt(img.dataset.fb);
  paintFallbacks(holder);
}, true);

// ── the map library, once ───────────────────────────────────────────────────
// `import('/lib/map/maplibre-gl.js')` returned a module with no `default`,
// because the file is a UMD bundle that assigns `window.maplibregl` -- so
// "Pick on the map" threw on `.Map` and the sheet stayed blank. The bundle is
// loaded the way it was built to be, and its stylesheet with it: without the
// stylesheet the canvas has no size and the pin has no place.
let mapLib = null;
export function loadMapLib(){
  if (mapLib) return mapLib;
  mapLib = new Promise((ok, no) => {
    if (!document.querySelector(`link[href="${MAP_CSS}"]`)) {
      const css = document.createElement('link'); css.rel = 'stylesheet'; css.href = MAP_CSS;
      document.head.appendChild(css);
    }
    if (window.maplibregl) return ok(window.maplibregl);
    const s = document.createElement('script'); s.src = MAP_JS; s.async = true;
    s.onload = () => window.maplibregl ? ok(window.maplibregl) : no(new Error('maplibre did not define itself'));
    s.onerror = () => no(new Error('maplibre failed to load'));
    document.head.appendChild(s);
  }).catch(e => { mapLib = null; throw e; });
  return mapLib;
}

// ── SPREAD ──────────────────────────────────────────────────────────────────
// Cards arrive with a short stagger from the tap outward. Written through
// CSSOM as a delay per element; reduced motion sets no delay and the CSS
// turns the movement off.
export function spread(els){
  const step = reduced() ? 0 : SPREAD_STEP_MS;
  let i = 0;
  for (const el of els) {
    el.classList.remove('rise');
    // reflow so the animation restarts on a card that already rose once
    void el.offsetWidth;
    el.style.animationDelay = `${Math.min(i * step, SPREAD_CAP_MS)}ms`;
    el.classList.add('rise');
    i++;
  }
}

// ── small helpers ───────────────────────────────────────────────────────────
export const debounce = (fn, ms) => { let h; return (...a) => { clearTimeout(h); h = setTimeout(() => fn(...a), ms); }; };
export const stars = n => '★'.repeat(Math.round(n || 0)) + '☆'.repeat(5 - Math.round(n || 0));
export const noop = () => {};
export { t };
