// THE BAG CARD'S WELCOME on the storefront (W-QR). A guest who scanned the card
// in a delivery bag lands on `/?src=bag[&c=...]`; this file remembers that for
// the visit (sessionStorage, `bag-core.js`), shows the venue's ONE public offer
// on a banner and in the cart, sends the landing in the order body at checkout,
// and then forgets it. The offer is the hub's (`GET .../welcome`, the same for
// everyone); whether THIS phone gets it is decided by the hub at placement, and
// a second use is answered with a polite sentence, never a refused order.
//
// Imported by cart.js, which app.js loads at once, so the landing is read on
// the first paint, before anything can rewrite the address.
//
// ASCII QUOTES ONLY in this file.

import { API, SLUG, money } from '/store/state.js';
import { t } from '/store/i18n.js';
import { esc, icon, toast } from '/store/ui.js';
import '/store/bag-words.js';
import { capture, bodyFor, forget, outcome } from '/store/bag-core.js';

function session(){ try { return globalThis.sessionStorage; } catch { return null; } }
const store = session() || { getItem: () => null, setItem(){}, removeItem(){} };

/// This visit's landing, or null.
export let LANDING = capture(globalThis.location?.search || '', store);

let offer = null;
/// The venue's public offer, asked once and only for a bag guest. The request
/// carries no landing, no campaign and nothing about the guest.
async function load(){
  if (!LANDING || offer) return offer;
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/welcome`);
    offer = r.ok ? await r.json() : { on: false };
  } catch { offer = { on: false }; }
  return offer;
}

/// The bonus in words, in the reader's currency through the one money formatter.
export function bonus(o){
  if (!o || !o.on) return '';
  if (o.kind === 'fixed') return o.min ? t('bg_fixedMin').replace('{value}', money(o.value)).replace('{min}', money(o.min)) : t('bg_fixed').replace('{value}', money(o.value));
  if (o.kind === 'gift') return t('bg_gift').replace('{dish}', o.gift?.name || '');
  if (o.kind === 'stamps') return t('bg_stamps');
  return '';
}

/// The line the cart and the checkout show while the landing is remembered.
export function bagLine(){
  const b = LANDING ? bonus(offer) : '';
  if (!b) return '';
  return `<p class="geo ok" id="bagLine">${icon('ticket')}<span>${esc(t('bg_banner').replace('{bonus}', b))}<br><small>${esc(t('bg_once'))}</small></span></p>`;
}

/// The order body's member (only at checkout).
export const bagBody = () => bodyFor(LANDING);

/// After a placement: forget the landing, and say what the hub decided.
export function bagPlaced(order){
  if (!LANDING) return;
  forget(store);
  LANDING = null;
  const k = outcome(order);
  if (k) toast(t(k));
}

/// The banner above the menu, once the offer is known.
async function banner(){
  const o = await load();
  if (!bonus(o) || document.getElementById('bagBanner')) return;
  const el = document.createElement('p');
  el.id = 'bagBanner'; el.className = 'geo ok wrap';
  el.innerHTML = `${icon('ticket')}<span>${esc(t('bg_banner').replace('{bonus}', bonus(o)))}<br><small>${esc(t('bg_once'))}</small></span>`;
  document.getElementById('app')?.before(el);
}
if (LANDING && globalThis.document) banner();
