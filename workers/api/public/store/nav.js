// The bottom bar -- five tabs, the way a phone app is held.
//
// Menu, Search, Cart, Orders, Info. The bar is glass over the Sea, sits above
// the safe area, and steps out of the way when a sheet is open: a sheet IS the
// screen while it is up. The cart tab carries the count; the floating pill
// above the bar carries the total, which is a <Money> and snaps.
//
// The header keeps the mark and the name and ONE button, which opens the
// preferences sheet: language and reading currency together, because they are
// the same kind of choice -- how this page is rendered for me, changing nothing
// about the order.

import { state, history, fetchRemembered, CURRENCIES, baseCurrency, displayCurrency, setDisplayCurrency, moneyEl, repaintMoney } from '/store/state.js';
import { t, lang, LANGS, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, isSheetOpen, sheetName } from '/store/ui.js';
import { openCart } from '/store/cart.js';
import { openVenue } from '/store/venue.js';
import { focusSearch, scrollTop } from '/store/menu.js';

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
    const name = e.detail?.name;
    if (!e.detail?.open) return light('menu');
    if (name === 'cart' || name === 'checkout' || name === 'pay') light('cart');
    else if (name === 'orders' || name === 'track') light('orders');
    else if (name === 'info') light('info');
  });
  $('#prefsBtn').onclick = openPrefs;
  retranslate(nav);
}
function light(id){
  for (const b of $$('#nav [data-tab]')) b.setAttribute('aria-current', b.dataset.tab === id ? 'page' : 'false');
}

/// Language and currency, one sheet. Choosing either rewrites text in place.
export function openPrefs(){
  const base = baseCurrency();
  const codes = [base, ...CURRENCIES.filter(c => c !== base)];
  sheet(`
    <p class="eyebrow" data-t="prefs"></p>
    <h2 data-t="language"></h2>
    <div class="seg" role="radiogroup">
      ${LANGS.map(l => `<button type="button" class="seg-b ${l === lang ? 'on' : ''}" data-l="${l}" aria-pressed="${l === lang}">${l.toUpperCase()}</button>`).join('')}
    </div>
    <h2 data-t="currency"></h2>
    <div class="seg" role="radiogroup">
      ${codes.map(c => `<button type="button" class="seg-b ${c === displayCurrency() ? 'on' : ''}" data-c="${c}" aria-pressed="${c === displayCurrency()}">${c}</button>`).join('')}
    </div>
    <p class="muted small" data-t="chargedIn"></p>`, { name: 'prefs' });
  for (const b of $$('[data-l]', $('#sheetIn'))) b.onclick = async () => {
    for (const x of $$('[data-l]', $('#sheetIn'))) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    await changeLang?.(b.dataset.l);
  };
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
    if (r.status !== 'fulfilled') return `<div class="hist gone"><span><b>#${esc(String(e.id).slice(0, 8))}</b><span class="muted">${esc(when)}</span></span>${moneyEl(e.total)}</div>`;
    const o = r.value;
    return `<button type="button" class="hist" data-o="${i}"><span><b>#${esc(String(o.id).slice(0, 8))}</b>
      <span class="muted"><span data-t-st="${esc(o.status)}"></span> · ${esc(when)}</span></span>${moneyEl(o.total ?? e.total)}</button>`;
  }).join('');
  retranslate(host); repaintMoney(host);
  for (const b of $$('[data-o]', host)) b.onclick = async () => {
    const o = got[Number(b.dataset.o)].value;
    (await import('/store/track.js')).openTracking(o);
  };
}
export { isSheetOpen };
