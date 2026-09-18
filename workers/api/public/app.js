// Storefront -- the entry. Boots the three acts and wires the modules together.
//
// It decides NOTHING about money or order state: prices come from the server
// and every status transition is the kernel's answer. It also renders nothing
// itself: the menu is built once by store/menu.js and thereafter only mutated,
// which is what makes a category tap instant and a language switch a change
// of words rather than a change of screen.

import { state, API, SLUG, indexProducts, loadAvoid, applyTheme, applyStage, resolveCurrency, repaintMoney, tokenFor, fetchRemembered } from '/store/state.js';
import { t, lang, setLang, retranslate } from '/store/i18n.js';
import { $, esc, icon, bindSheetChrome } from '/store/ui.js';
import { buildMenu, patchTexts, onOpenDish, onQuickAdd } from '/store/menu.js';
import { openDish, quickAdd, onDishAdded } from '/store/dish.js';
import { refreshBar, bounceBar, openCart } from '/store/cart.js';
import { openVenue, refreshHero } from '/store/venue.js';
import { mountNav, onChangeLang } from '/store/nav.js';
import { initSea, seaArrive, seaTouch } from '/store/sea.js';

/// A touch on the venue's mark is a touch on the water beneath it.
const TOUCH_STRENGTH = 0.55;

/// How many placeholder cards the skeleton shows while the menu loads.
const SKELETON_CARDS = 3;

const skeleton = () => `
  <div class="hero"><div class="skel skel-eyebrow"></div><div class="skel skel-title"></div><div class="skel skel-chips"></div></div>
  <div class="skel skel-rail"></div>
  <div class="cards">${'<div class="skel skel-card"></div>'.repeat(SKELETON_CARDS)}</div>`;

async function fetchMenu(){
  const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/menu?locale=${lang}`);
  if (!r.ok) throw new Error('HTTP ' + r.status);
  const d = await r.json();
  // The key rides beside the location in the payload; the checkout reads it
  // off the location. Carried across once, here, so no module has to know.
  if (d.location && d.stripePublishableKey) d.location.stripePublishableKey = d.stripePublishableKey;
  // A degraded answer says so on the console, where a developer looks.
  if (Array.isArray(d.warnings) && d.warnings.length) console.warn('menu:', d.warnings.join('; '));
  return d;
}

function paintHeader(L){
  document.title = L.name;
  $('#brandName').textContent = L.name;
  const mark = $('#brandMark');
  if (L.logoUrl) { mark.src = L.logoUrl; mark.alt = L.name; mark.hidden = false; } else mark.hidden = true;
}

async function load(){
  $('#app').innerHTML = skeleton();
  try {
    const d = await fetchMenu();
    state.loc = d.location; state.cats = d.categories || [];
    indexProducts(state.cats);
    state.avoid = loadAvoid();
    applyTheme(d.location.theme);
    applyStage(d.location.stage);
    paintHeader(d.location);
    document.documentElement.lang = lang;
    await resolveCurrency();
    buildMenu(state.cats);
    refreshBar();
    retranslate(document);
    repaintMoney(document);
    dispatchEvent(new Event('dw:money'));
    initSea().then(seaArrive);
    netState();
    returnFromCard();
  } catch (e) {
    const off = !navigator.onLine;
    $('#app').innerHTML = `<div class="empty" role="alert">
      ${icon(off ? 'wifi-off' : 'alert-triangle', 'ico-lg')}
      <b>${esc(off ? t('offline') : t('loadFail'))}</b>
      <span class="reason">${esc(String(e.message || e))}</span>
      <button class="btn mt-3" id="retry">${esc(t('retry'))}</button>
      ${state.loc?.phone ? `<a class="btn btn-ghost mt-1" href="tel:${esc(state.loc.phone)}">${esc(state.loc.phone)}</a>` : ''}
    </div>`;
    $('#retry').onclick = load;
  }
}

/// A language switch is a change of WORDS. The static copy is rewritten in
/// place by i18n.js; the venue's own words are re-fetched in the new locale
/// and patched by id; money is re-formatted on the same nodes. No screen is
/// rebuilt, nothing scrolls, an open sheet stays open.
async function changeLang(code){
  if (!setLang(code)) return;
  try {
    const d = await fetchMenu();
    state.loc = d.location; state.cats = d.categories || [];
    indexProducts(state.cats);
    patchTexts(state.cats);
  } catch { /* the words stay as they were; the next load will fetch them */ }
  refreshHero();
  repaintMoney(document);
  refreshBar();
  retranslate(document);
  document.documentElement.lang = lang;
}

/// Back from a card payment: Stripe returns to `/?order=<id>`.
async function returnFromCard(){
  const id = new URLSearchParams(location.search).get('order');
  if (!id || !tokenFor(id)) return;
  try {
    const o = await fetchRemembered({ id, t: tokenFor(id) });
    (await import('/store/track.js')).openTracking(o);
  } catch {}
  window.history.replaceState(null, '', location.pathname);
}

function netState(){
  const el = $('#offline');
  el.hidden = navigator.onLine;
  el.textContent = t('offline') + (state.loc?.phone ? ` · ${state.loc.phone}` : '');
}

// ── wiring ──────────────────────────────────────────────────────────────────
bindSheetChrome();
mountNav();
onChangeLang(changeLang);
onOpenDish(openDish);
onQuickAdd(quickAdd);
onDishAdded(() => { refreshBar(); bounceBar(); });
addEventListener('dw:cart', refreshBar);
$('#cartPill').onclick = openCart;
$('#app').addEventListener('click', e => {
  const v = e.target.closest('[data-open-venue]');
  if (v) openVenue(v.dataset.openVenue || null);
});
$('#app').addEventListener('pointerdown', e => { if (e.target.closest('.hero')) seaTouch(e.clientX, e.clientY, TOUCH_STRENGTH); }, { passive: true });
addEventListener('online', netState); addEventListener('offline', netState);
load();
