// The bottom bar -- five tabs, the way a phone app is held -- and the two
// header controls.
//
// Menu, Search, Cart, Orders, Info. The bar is glass over the Sea, sits above
// the safe area, and steps out of the way when a sheet is open: a sheet IS the
// screen while it is up. The cart tab carries the count; the floating pill
// above the bar carries the total, which is a <Money> and snaps.
//
// THE HEADER HAS TWO BUTTONS, NOT ONE. Language and reading currency were one
// sheet behind one icon, and a customer who wanted euros opened a sheet titled
// "language". They are different decisions -- what I read in, what I count in
// -- so each has its own control and its own sheet. The allergen filter lives
// in the language sheet rather than in the search bar, where it crowded the
// one field a hungry customer is looking for.

import { state, history, fetchRemembered, CURRENCIES, baseCurrency, displayCurrency, setDisplayCurrency, moneyEl, repaintMoney, ALLERGENS, allergenName, saveAvoid } from '/store/state.js';
import { t, lang, LANGS, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, isSheetOpen, sheetName } from '/store/ui.js';
import { openCart } from '/store/cart.js';
import { openVenue } from '/store/venue.js';
import { focusSearch, scrollTop, applyFilters } from '/store/menu.js';

let changeLang = null;
export function onChangeLang(fn){ changeLang = fn; }

const TABS = [
  ['menu',   'home',         'menu'],
  // ITS OWN KEY. `search` is the field's placeholder ("Search the menu"), and
  // a tab is 78px wide on a phone: that label wrapped to two lines in all
  // three languages. A tab gets one word.
  ['search', 'search',       'tabSearch'],
  ['cart',   'shopping-bag', 'cart'],
  ['orders', 'receipt',      'orders'],
  ['info',   'info-circle',  'info'],
];
/// Which tab a sheet belongs to, so the bar follows what is open.
const TAB_OF_SHEET = { cart: 'cart', checkout: 'cart', pay: 'cart', orders: 'orders', track: 'orders', info: 'info' };
/// The order id is shown short: eight characters is enough to tell two orders
/// apart on a phone and short enough to read aloud to the venue.
const ORDER_ID_SHOWN = 8;

export function mountNav(){
  const nav = $('#nav');
  nav.innerHTML = TABS.map(([id, ic, key]) => `
    <button type="button" class="tab" data-tab="${id}" aria-current="${id === 'menu' ? 'page' : 'false'}">
      <span class="tab-ic">${icon(ic)}${id === 'cart' ? `<span class="tab-n" id="navCartN" hidden>0</span>` : ''}</span>
      <span class="tab-l" data-t="${key}"></span>
    </button>`).join('');
  nav.addEventListener('click', e => {
    const b = e.target.closest('[data-tab]'); if (!b) return;
    const id = b.dataset.tab;
    if (id === 'menu')   { closeSheet(); scrollTop(); }
    if (id === 'search') { closeSheet(); focusSearch(); }
    if (id === 'cart')   { sheetName() === 'cart' ? closeSheet() : openCart(); }
    if (id === 'orders') { sheetName() === 'orders' ? closeSheet() : openHistory(); }
    if (id === 'info')   { sheetName() === 'info' ? closeSheet() : openVenue(); }
    light(id);
  });
  // The tab follows the sheet, so a cart opened from a card lights "Cart".
  addEventListener('dw:sheet', e => {
    if (!e.detail?.open) return light('menu');
    light(TAB_OF_SHEET[e.detail.name] || 'menu');
  });
  $('#langBtn').onclick = openLanguage;
  $('#curBtn').onclick = openCurrency;
  paintCurrencyButton();
  addEventListener('dw:money', paintCurrencyButton);
  retranslate(nav);
}
function light(id){
  for (const b of $$('#nav [data-tab]')) b.setAttribute('aria-current', b.dataset.tab === id ? 'page' : 'false');
}
/// The currency button shows the code it is reading in, so the state is
/// visible without opening anything.
function paintCurrencyButton(){
  const el = $('#curCode'); if (el) el.textContent = displayCurrency();
}

/// Language, and the allergens to keep off the menu. Choosing a language
/// rewrites text in place; nothing else moves.
export function openLanguage(){
  const avoid = state.avoid || [];
  sheet(`
    <p class="eyebrow" data-t="language"></p>
    <h2 data-t="chooseLang"></h2>
    <div class="choices" role="radiogroup">
      ${LANGS.map(l => `<button type="button" class="choice ${l === lang ? 'on' : ''}" data-l="${l}" aria-pressed="${l === lang}">
        <span class="choice-code">${l.toUpperCase()}</span><span class="choice-name">${esc(t('langs')[l] || l)}</span>${icon('check', 'choice-ck')}</button>`).join('')}
    </div>
    ${ALLERGENS.length ? `
    <h3 class="dsec" data-t="avoid"></h3>
    <p class="muted small" data-t="avoidHint"></p>
    <div class="avoid-in" id="avoidBox">${ALLERGENS.map(([code]) => `<button type="button" class="chip ${avoid.includes(code) ? 'on' : ''}"
       data-avoid="${code}" aria-pressed="${avoid.includes(code)}">${esc(allergenName(code))}</button>`).join('')}</div>
    <p class="muted small mt-2" id="avoidCount"></p>` : ''}`, { name: 'lang' });
  for (const b of $$('[data-l]', $('#sheetIn'))) b.onclick = async () => {
    for (const x of $$('[data-l]', $('#sheetIn'))) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    await changeLang?.(b.dataset.l);
    // The allergen names are the venue's own vocabulary in the new language.
    for (const c of $$('[data-avoid]', $('#sheetIn'))) c.textContent = allergenName(c.dataset.avoid);
  };
  for (const b of $$('[data-avoid]', $('#sheetIn'))) b.onclick = () => {
    const code = b.dataset.avoid;
    state.avoid = state.avoid.includes(code) ? state.avoid.filter(c => c !== code) : [...state.avoid, code];
    saveAvoid();
    const onn = state.avoid.includes(code);
    b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn));
    applyFilters();
  };
}

/// The reading currency. The charge stays in the venue's currency and the
/// cart says so; this only changes what the customer counts in.
export function openCurrency(){
  const base = baseCurrency();
  const codes = [base, ...CURRENCIES.filter(c => c !== base)];
  sheet(`
    <p class="eyebrow" data-t="currency"></p>
    <h2 data-t="readIn"></h2>
    <div class="choices" role="radiogroup">
      ${codes.map(c => `<button type="button" class="choice ${c === displayCurrency() ? 'on' : ''}" data-c="${c}" aria-pressed="${c === displayCurrency()}">
        <span class="choice-code money">${c}</span><span class="choice-name">${esc(t('curs')[c] || c)}</span>${icon('check', 'choice-ck')}</button>`).join('')}
    </div>
    <p class="muted small"><span data-t="chargedIn"></span>: <b class="money">${esc(base)}</b></p>`, { name: 'currency' });
  for (const b of $$('[data-c]', $('#sheetIn'))) b.onclick = async () => {
    for (const x of $$('[data-c]', $('#sheetIn'))) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    await setDisplayCurrency(b.dataset.c);
  };
}

/// The orders this browser placed. Each is fetched with its own token; one
/// whose token has expired is SHOWN, greyed, rather than dropped.
export async function openHistory(){
  const list = history();
  if (!list.length) return sheet(`<div class="empty">${icon('receipt', 'ico-lg')}<b data-t="noOrders"></b></div>`, { name: 'orders' });
  sheet(`<p class="eyebrow" data-t="orders"></p><h2 data-t="myOrders"></h2>
    <div id="histList">${list.map(() => `<div class="skel skel-row"></div>`).join('')}</div>`, { name: 'orders' });
  const got = await Promise.allSettled(list.map(fetchRemembered));
  if (sheetName() !== 'orders') return;
  const host = $('#histList'); if (!host) return;
  host.innerHTML = got.map((r, i) => {
    const e = list[i];
    const when = new Date(e.at).toLocaleDateString(lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq', { day: 'numeric', month: 'short' });
    const short = esc(String(e.id).slice(0, ORDER_ID_SHOWN));
    if (r.status !== 'fulfilled') return `<div class="hist gone"><span><b>#${short}</b><span class="muted">${esc(when)}</span></span>${moneyEl(e.total)}</div>`;
    const o = r.value;
    return `<button type="button" class="hist" data-o="${i}"><span><b>#${short}</b>
      <span class="muted"><span data-t-st="${esc(o.status)}"></span> · ${esc(when)}</span></span>${moneyEl(o.total ?? e.total)}</button>`;
  }).join('');
  retranslate(host); repaintMoney(host);
  for (const b of $$('[data-o]', host)) b.onclick = async () => {
    const o = got[Number(b.dataset.o)].value;
    (await import('/store/track.js')).openTracking(o);
  };
}
export { isSheetOpen };
