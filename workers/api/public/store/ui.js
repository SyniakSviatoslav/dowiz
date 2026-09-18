// The Sheet's primitives: the one bottom sheet, the toast, escaping, icons,
// the deliberate mark for a dish with no photo, and the two motions the design
// language allows -- a sheet that RISES over the Sea (never slides in from the
// side) and a set of cards that SPREAD in with a short stagger.
//
// Money never passes through here. It is formatted in state.js from an integer
// and written as text; there is no animated path to a price anywhere.

import { t, retranslate } from '/store/i18n.js';
import { repaintMoney } from '/store/state.js';

export const $ = (s, r = document) => r.querySelector(s);
export const $$ = (s, r = document) => [...r.querySelectorAll(s)];
export const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
export const icon = (name, cls = '') => `<i class="ti ti-${name} ${cls}" aria-hidden="true"></i>`;
export const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

// ── the sheet ───────────────────────────────────────────────────────────────
// ONE sheet, reused. It rises with the tide easing, wears the spectral rim,
// and closes on the scrim, on Escape, and on a drag down from the grip.
// `name` tells the bottom navigation which tab to light and lets a re-open
// (a cart quantity change re-drawing the cart) keep the sheet's scroll.
let onClose = null;
export function sheet(html, { name = null, keepScroll = false, attending = false } = {}){
  const box = $('#sheet'), inner = $('#sheetIn');
  const wasOpen = box.classList.contains('show');
  const top = keepScroll && wasOpen ? box.scrollTop : 0;
  inner.innerHTML = html;
  paintFallbacks(inner);
  retranslate(inner);
  repaintMoney(inner);
  box.classList.toggle('attending', attending);
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
  box.classList.remove('show', 'attending');
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
    if (dy > 90) closeSheet();
    y0 = null; dy = 0;
  };
  grip.addEventListener('pointerup', end);
  grip.addEventListener('pointercancel', end);
}

// ── toast ───────────────────────────────────────────────────────────────────
export function toast(msg){
  const el = $('#toast'); el.textContent = msg; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 2600);
}

// ── a dish with no photo ────────────────────────────────────────────────────
// A deliberate mark, not a grey rectangle. Hue is a stable hash of the name so
// the same dish always looks the same. The gradient is per-dish and computed,
// so it goes through CSSOM: `style-src 'self'` forbids the attribute.
export function fallbackArt(name){
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

// ── SPREAD ──────────────────────────────────────────────────────────────────
// Cards arrive with a short stagger from the tap outward -- DESIGN.md §6 names
// it: fadeSlideUp .35s, .05s per child. Written through CSSOM as a delay per
// element, capped so a section of thirty cards does not take two seconds to
// finish; reduced motion sets no delay and the CSS turns the movement off.
export function spread(els, from = 0){
  const step = reduced() ? 0 : 40;
  let i = 0;
  for (const el of els) {
    el.classList.remove('rise');
    // reflow so the animation restarts on a card that already rose once
    void el.offsetWidth;
    el.style.animationDelay = `${Math.min(i * step, 480)}ms`;
    el.classList.add('rise');
    i++;
  }
  void from;
}

// ── small helpers ───────────────────────────────────────────────────────────
export const debounce = (fn, ms) => { let h; return (...a) => { clearTimeout(h); h = setTimeout(() => fn(...a), ms); }; };
export const stars = n => '★'.repeat(Math.round(n || 0)) + '☆'.repeat(5 - Math.round(n || 0));
export const noop = () => {};
export { t };
