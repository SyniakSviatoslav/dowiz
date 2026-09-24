// The bottom bar -- six tabs, the way a phone app is held -- and the two
// header controls.
//
// Menu, Search, Table, Cart, Orders, Info. The bar is glass over the Sea, sits above
// the safe area, and steps out of the way when a sheet is open: a sheet IS the
// screen while it is up. The cart tab carries the count; the floating pill
// above the bar carries the total, which is a <Money> and snaps.
//
// THE HEADER HAS TWO BUTTONS, NOT ONE. Language and reading currency were one
// sheet behind one icon, and a customer who wanted euros opened a sheet titled
// "language". They are different decisions -- what I read in, what I count in
// -- so each has its own control and its own sheet.

import { history, fetchRemembered, CURRENCIES, baseCurrency, displayCurrency, setDisplayCurrency, moneyEl, repaintMoney } from '/store/state.js';
import { t, lang, LANGS, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, isSheetOpen, sheetName } from '/store/ui.js';
import { relabel } from '/store/motion.js';
import { openCart } from '/store/cart.js';
import { openVenue } from '/store/venue.js';
import { openBooking } from '/store/booking.js';
import { focusSearch, scrollTop } from '/store/menu.js';
import { choiceList, emptySheet, rows } from '/store/parts.js';

let changeLang = null;
export function onChangeLang(fn){ changeLang = fn; }

// SIX, NOT FIVE. Booking a table was reachable only by somebody who already
// knew the URL, which is the defect class `tools/gates/unreached.py` exists
// for: a capability that is built, tested, and that no live path reaches. It
// gets a tab, beside the menu, because that is where a diner looks. The label
// is one word in all three languages, for the reason `tabSearch` is.
const TABS = [
  ['menu',   'bowl-chopsticks', 'menu'],
  ['book',   'tools-kitchen-2', 'bkTab'],
  // ITS OWN KEY. `search` is the field's placeholder ("Search the menu"), and
  // a tab is 78px wide on a phone: that label wrapped to two lines in all
  // three languages. A tab gets one word.
  ['search', 'search',       'tabSearch'],
  ['cart',   'bento',        'cart'],
  ['orders', 'scroll',       'orders'],
  ['info',   'lantern',      'info'],
];
/// Each tab's learning anchor, as a literal (docs/learn/anchors-store.txt).
const TOUR_OF_TAB = { menu: 'nav.menu', search: 'nav.search', book: 'nav.book', cart: 'nav.cart', orders: 'nav.orders', info: 'nav.info' };
/// Which tab a sheet belongs to, so the bar follows what is open.
const TAB_OF_SHEET = { book: 'book', cart: 'cart', checkout: 'cart', pay: 'cart', orders: 'orders', track: 'orders', info: 'info' };
/// A tab answers the finger with a tap of the phone's own, where it can.
const TAB_HAPTIC_MS = 6;
/// The order id is shown short: eight characters is enough to tell two orders
/// apart on a phone and short enough to read aloud to the venue.
const ORDER_ID_SHOWN = 8;

export function mountNav(){
  const nav = $('#nav');
  nav.innerHTML = TABS.map(([id, ic, key]) => `
    <button type="button" class="tab" data-tab="${id}" data-tour="${TOUR_OF_TAB[id]}" aria-current="${id === 'menu' ? 'page' : 'false'}">
      <span class="tab-ic">${icon(ic)}${id === 'cart' ? `<span class="tab-n" id="navCartN" hidden>0</span>` : ''}</span>
      <span class="tab-l" data-t="${key}"></span>
    </button>`).join('');
  nav.addEventListener('click', e => {
    const b = e.target.closest('[data-tab]'); if (!b) return;
    try { navigator.vibrate?.(TAB_HAPTIC_MS); } catch {}
    const id = b.dataset.tab;
    if (id === 'menu')   { closeSheet(); scrollTop(); }
    if (id === 'search') { closeSheet(); focusSearch(); }
    if (id === 'book')   { sheetName() === 'book' ? closeSheet() : openBooking(); }
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

/// One row of a pick-one list pressed, the rest released.
function pick(all, b){ for (const x of all) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); } }

/// Language. Choosing one rewrites text in place; nothing else moves.
export function openLanguage(){
  sheet(`
    <p class="eyebrow" data-t="language"></p>
    <h2 data-t="chooseLang"></h2>
    ${choiceList(LANGS.map(l => ({ on: l === lang, code: l.toUpperCase(), title: t('langs')[l] || l, data: { l, tour: 'lang.choice' } })), { label: t('chooseLang') })}`, { name: 'lang' });
  for (const b of $$('[data-l]', $('#sheetIn'))) b.onclick = async () => {
    pick($$('[data-l]', $('#sheetIn')), b);
    await changeLang?.(b.dataset.l);
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
    ${choiceList(codes.map(c => ({ on: c === displayCurrency(), code: c, money: true, title: t('curs')[c] || c, data: { c, tour: 'currency.choice' } })), { label: t('readIn') })}
    <p class="muted small"><span data-t="chargedIn"></span>: <b class="money">${esc(base)}</b></p>`, { name: 'currency' });
  for (const b of $$('[data-c]', $('#sheetIn'))) b.onclick = async () => {
    pick($$('[data-c]', $('#sheetIn')), b);
    await relabel(() => setDisplayCurrency(b.dataset.c));
  };
}

/// The orders this browser placed. Each is fetched with its own token; one
/// whose token has expired is SHOWN, greyed, rather than dropped.
export async function openHistory(){
  const list = history();
  if (!list.length) return sheet(emptySheet({ icon: 'receipt', title: 'noOrders' }), { name: 'orders' });
  sheet(`<p class="eyebrow" data-t="orders"></p><h2 data-t="myOrders"></h2>
    <div id="histList">${rows(list.length, t('loading'))}</div>`, { name: 'orders' });
  const got = await Promise.allSettled(list.map(fetchRemembered));
  if (sheetName() !== 'orders') return;
  const host = $('#histList'); if (!host) return;
  host.innerHTML = got.map((r, i) => {
    const e = list[i];
    const when = new Date(e.at).toLocaleDateString(lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq', { day: 'numeric', month: 'short' });
    const short = esc(String(e.id).slice(0, ORDER_ID_SHOWN));
    if (r.status !== 'fulfilled') return `<div class="hist gone"><span><b>#${short}</b><span class="muted">${esc(when)}</span></span>${moneyEl(e.total)}</div>`;
    const o = r.value;
    return `<button type="button" class="hist" data-o="${i}" data-tour="orders.row"><span><b>#${short}</b>
      <span class="muted"><span data-t-st="${esc(o.status)}"></span> · ${esc(when)}</span></span>${moneyEl(o.total ?? e.total)}</button>`;
  }).join('');
  retranslate(host); repaintMoney(host);
  for (const b of $$('[data-o]', host)) b.onclick = async () => {
    const o = got[Number(b.dataset.o)].value;
    (await import('/store/track.js')).openTracking(o);
  };
}
export { isSheetOpen };
