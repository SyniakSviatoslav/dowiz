// Storefront -- the entry. Boots the three acts and wires the modules together.
//
// It decides NOTHING about money or order state: prices come from the server
// and every status transition is the kernel's answer. It also renders nothing
// itself: the menu is built once by store/menu.js and thereafter only mutated,
// which is what makes a category tap instant and a language switch a change
// of words rather than a change of screen.

import { state, API, SLUG, indexProducts, applyTheme, applyStage, resolveCurrency, repaintMoney, tokenFor, fetchRemembered } from '/store/state.js';
import { t, lang, LANGS, setLang, retranslate } from '/store/i18n.js';
import { $, esc, icon, bindSheetChrome } from '/store/ui.js';
import { safeGet, safeSet } from '/store/storage.js';
import { buildMenu, patchTexts, onOpenDish, onQuickAdd } from '/store/menu.js';
import { openDish, quickAdd, onDishAdded } from '/store/dish.js';
import { refreshBar, bounceBar, openCart } from '/store/cart.js';
import { openVenue, refreshHero } from '/store/venue.js';
import { mountNav, onChangeLang } from '/store/nav.js';
import { initSea, seaArrive, seaTouch } from '/store/sea.js';
import { relabel } from '/store/motion.js';
import { LEAVES } from '/store/venue.js';

/// A touch on the venue's mark is a touch on the water beneath it.
const TOUCH_STRENGTH = 0.55;
/// The stage drifts up at this fraction of the scroll, behind the page:
/// a garden seen through a window, not a picture pasted on one.
const PARALLAX = 0.28;
const PARALLAX_MAX_PX = 140;

/// THE LOADER IS THE FIRST FRAME OF THE FILM, not a grey placeholder, and
/// not a spinner. While the menu is on its way the page shows the venue's
/// mark being MADE: ink blooms on the paper, the branch draws itself from its
/// root, the leaves open one by one, the enso is brushed round, the seal is
/// stamped down with a press, the name rises letter by letter, a hairline
/// grows under it; then the whole frame breathes -- leaves flutter, dust
/// drifts -- until the menu arrives and the frame dissolves upward. The
/// venue's colours, seal and name are REMEMBERED from the last visit, so a
/// returning customer never sees the platform's paper flash before their
/// venue's own; the first visit gets the same scene in dowiz's tones.
const BOOT_KEY = 'dw_boot_' + SLUG;
const ENSO_R = 78;
/// How long the frame takes to dissolve when the menu arrives.
const LOADER_OUT_MS = 450;
const loader = ({ name, seal, motif }) => `
  <div class="loader" aria-busy="true">
    <div class="ld-scene" aria-hidden="true">
      <svg class="ld-bloom" viewBox="0 0 200 200"><defs><filter id="ld-soft"><feGaussianBlur stdDeviation="12"/></filter></defs><circle cx="100" cy="100" r="70" filter="url(#ld-soft)"/></svg>
      ${motif === 'leaf' ? `<svg class="ld-branch" viewBox="0 0 300 240">
        <defs><symbol id="ld-lf" viewBox="-2 -14 44 28"><path class="blade" d="M0 0C8-13 26-15 38-3 26 9 8 9 0 0z"/><path class="vein" d="M2 0C12-4 24-6 36-3"/></symbol></defs>
        <path class="bark ld-draw" pathLength="100" d="M4 236C50 190 70 130 120 92S210 30 296 14"/>
        <path class="bark thin ld-draw" pathLength="100" d="M150 70C200 40 240 24 296 14"/>
        <path class="bark thin ld-draw" pathLength="100" d="M110 100C130 80 150 78 175 84"/>
        <path class="bark thin ld-draw" pathLength="100" d="M60 165C80 150 95 148 118 152"/>
        ${LEAVES.map(([x, y, a, sc, gold], i) => `<g transform="translate(${x} ${y}) rotate(${a}) scale(${sc})"><use href="#ld-lf" class="leaf ld-leaf ${gold ? 'gold' : ''}" data-i="${i}" width="44" height="28" x="-2" y="-14"/></g>`).join('')}
      </svg>` : ''}
      <svg class="ld-enso" viewBox="0 0 200 200">
        <defs><filter id="ld-ink" x="-10%" y="-10%" width="120%" height="120%"><feTurbulence type="fractalNoise" baseFrequency=".055" numOctaves="3" seed="7" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="7" xChannelSelector="R" yChannelSelector="G"/></filter></defs>
        <g filter="url(#ld-ink)"><circle class="enso-dry" cx="100" cy="100" r="${ENSO_R}" pathLength="100"/><circle class="ld-stroke" cx="100" cy="100" r="${ENSO_R}" pathLength="100"/></g>
      </svg>
      ${seal ? `<span class="ld-seal">${esc(seal)}</span><span class="ld-press"></span>` : ''}
    </div>
    <p class="loader-name">${[...String(name)].map((ch, i) => `<i data-i="${i}">${ch === ' ' ? '&nbsp;' : esc(ch)}</i>`).join('')}</p>
    <span class="loader-line" aria-hidden="true"></span>
  </div>`;
function bootRecord(){ try { return JSON.parse(safeGet(BOOT_KEY) || 'null') || {}; } catch { return {}; } }
function rememberBoot(L){ safeSet(BOOT_KEY, JSON.stringify({ name: L.name, theme: L.theme, stage: L.stage, logoUrl: L.logoUrl })); }

const fetchMenu = () => fetchMenuIn(lang);
async function fetchMenuIn(locale){
  const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/menu?locale=${locale}`);
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
  // The tab's icon and the home-screen icon are the venue's mark.
  if (L.logoUrl) { $('#favicon').href = L.logoUrl; $('#touchIcon').href = L.logoUrl; }
}

// ── the storefront as an app ────────────────────────────────────────────────
// Chrome offers to install through an event that must be caught and kept;
// the Info sheet shows the row when it has one. iOS never fires it and is
// told how instead. The worker keeps the shell for an installed app that
// opens without a connection.
let installPrompt = null;
addEventListener('beforeinstallprompt', e => { e.preventDefault(); installPrompt = e; dispatchEvent(new Event('dw:installable')); });
addEventListener('appinstalled', () => { installPrompt = null; dispatchEvent(new Event('dw:installable')); });
export const canInstall = () => !!installPrompt;
export const isStandalone = () => matchMedia('(display-mode: standalone)').matches || navigator.standalone === true;
export async function promptInstall(){
  if (!installPrompt) return false;
  installPrompt.prompt();
  const { outcome } = await installPrompt.userChoice;
  if (outcome === 'accepted') installPrompt = null;
  return outcome === 'accepted';
}
if ('serviceWorker' in navigator) addEventListener('load', () => navigator.serviceWorker.register('/sw.js').catch(() => {}));

async function load(){
  // The remembered venue, before the network: colours, seal, name.
  const boot = bootRecord();
  if (boot.theme) applyTheme(boot.theme);
  if (boot.stage) applyStage(boot.stage);
  if (boot.name) paintHeader({ name: boot.name, logoUrl: boot.logoUrl });
  $('#app').innerHTML = loader({ name: boot.name || 'dowiz', seal: boot.stage?.seal || '', motif: boot.stage?.motif || 'leaf' });
  // Each letter's and leaf's delay through CSSOM: `style-src 'self'` forbids the attribute.
  for (const el of $('#app').querySelectorAll('.loader-name i, .ld-leaf')) el.style.setProperty('--i', el.dataset.i);
  // The dust drifts from the first frame, in the remembered tones.
  initSea({ colour: boot.stage?.warm, leaf: boot.stage?.sage }).then(seaArrive);
  try {
    const d = await fetchMenu();
    state.loc = d.location; state.cats = d.categories || [];
    indexProducts(state.cats);
    applyTheme(d.location.theme);
    applyStage(d.location.stage);
    paintHeader(d.location);
    rememberBoot(d.location);
    document.documentElement.lang = lang;
    await resolveCurrency();
    // The frame dissolves upward before the spread takes its place.
    const ld = $('.loader');
    if (ld && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
      ld.classList.add('out');
      await new Promise(r => setTimeout(r, LOADER_OUT_MS));
    }
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
  if (!LANGS.includes(code) || code === lang) return;
  // The words are fetched FIRST, in the new language, so the fog has
  // something to leave behind; then everything is rewritten under it.
  let d = null;
  try { d = await fetchMenuIn(code); } catch { /* the words stay as they were */ }
  await relabel(() => {
    setLang(code);
    if (d) { state.loc = d.location; state.cats = d.categories || []; indexProducts(state.cats); patchTexts(state.cats); }
    refreshHero();
    repaintMoney(document);
    refreshBar();
    retranslate(document);
    document.documentElement.lang = lang;
  });
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
/// How long the card shows the tick after an add.
const ADDED_MS = 900;
onDishAdded((p, q, el) => {
  refreshBar(); bounceBar();
  const card = el?.closest?.('.card') || $(`.card[data-p="${CSS.escape(p.id)}"]`);
  if (!card) return;
  card.classList.add('added');
  setTimeout(() => card.classList.remove('added'), ADDED_MS);
});
addEventListener('dw:cart', refreshBar);
$('#cartPill').onclick = openCart;
$('#app').addEventListener('click', e => {
  const v = e.target.closest('[data-open-venue]');
  if (v) openVenue(v.dataset.openVenue || null);
});
$('#app').addEventListener('pointerdown', e => { if (e.target.closest('.hero')) seaTouch(e.clientX, e.clientY, TOUCH_STRENGTH); }, { passive: true });
// Parallax through CSSOM, one write per frame, none when motion is reduced.
let parallaxQueued = false;
addEventListener('scroll', () => {
  if (parallaxQueued || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  parallaxQueued = true;
  requestAnimationFrame(() => {
    parallaxQueued = false;
    const art = $('.stage-art'); if (!art) return;
    art.style.transform = `translateY(${Math.min(PARALLAX_MAX_PX, scrollY * PARALLAX)}px)`;
  });
}, { passive: true });
addEventListener('online', netState); addEventListener('offline', netState);
load();
