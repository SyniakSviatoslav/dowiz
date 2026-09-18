// dowiz kit — the pieces more than one screen draws.
//
// A piece lands here the moment a SECOND screen needs it, not before: the kit
// has 87 frames and a shared-component library written ahead of the screens is
// a guess about which parts are shared. Each of these was already drawn twice.

import { icon, esc } from '/kit/app.js';
import { count as basketCount, onChange as onBasketChange } from '/kit/basket.js';

// ── The dish plate ─────────────────────────────────────────────────────────
// The kit draws a flat grey rectangle wherever a photograph will go. dowiz's
// storefront already answers "no photograph" with a gradient keyed to the name,
// so both surfaces give the same dish the same colour instead of two different
// placeholders. The value is computed, so it is written by CSSOM after the
// markup is in the tree -- `style-src 'self'` forbids the attribute.
export const plate = name => `<div class="k-plate" data-hue="${hueOf(String(name))}"></div>`;

export function hueOf(name){
  let h = 0; for (const ch of name) h = (h * 31 + ch.codePointAt(0)) >>> 0;
  return h % 360;
}

export function paintPlates(root){
  for (const el of (root || document).querySelectorAll('.k-plate[data-hue]')){
    const hue = Number(el.dataset.hue);
    el.style.background =
      `linear-gradient(140deg,hsl(${hue} 46% 58%),hsl(${(hue + 38) % 360} 52% 42%))`;
    el.removeAttribute('data-hue');
  }
}

// ── Detail-screen hero controls ────────────────────────────────────────────
// Back, favourite, share. The favourite is a toggle and says so; the other two
// are plain actions. `data-back` rather than a route name, because where back
// goes depends on where the customer came from.
export const heroButtons = ({ share = true } = {}) => `
  <button class="k-hero-btn k-back" type="button" data-back aria-label="Назад">
    <span class="k-bell-face">${icon('arrow-left')}</span>
  </button>
  <button class="k-hero-btn k-act-1" type="button" aria-pressed="false" aria-label="У обране">
    <span class="k-bell-face">${icon('heart-outline')}</span>
  </button>
  ${share ? `
  <button class="k-hero-btn k-act-2" type="button" data-share aria-label="Поділитися">
    <span class="k-bell-face">${icon('share-btn')}</span>
  </button>` : ''}`;

// ── The sticky bar with a quantity stepper and one call to action ──────────
// `output` rather than a span: the number is a computed result, and a screen
// reader should hear it change.
export const qtyBar = ({ qty = 1, label, total, minusOff = false }) => `
  <div class="k-bar">
    <div class="k-qty">
      <button type="button" data-step="-1" aria-label="Менше"
              ${minusOff ? 'disabled' : ''}>${icon('minus-glyph')}</button>
      <output id="qty" aria-live="polite">${qty}</output>
      <button type="button" data-step="1" aria-label="Більше">${icon('plus-glyph')}</button>
    </div>
    <button class="k-cta" type="button" data-cta>
      <span>${esc(label)}</span><span id="ctaTotal">${esc(total)}</span>
    </button>
  </div>`;

// ── A bar that is only a call to action ────────────────────────────────────
export const ctaBar = (label, { to = '', disabled = false } = {}) => `
  <div class="k-bar">
    <button class="k-cta" type="button" ${to ? `data-go="${esc(to)}"` : 'data-cta'}
            ${disabled ? 'disabled' : ''}>${esc(label)}</button>
  </div>`;

// ── The five-tab navbar ────────────────────────────────────────────────────
const TABS = [
  { id: 'home',       name: 'Home',      icon: 'bold-essentional-ui-home-angle2' },
  { id: 'explore',    name: 'Explore',   icon: 'linear-map-location-compass' },
  { id: 'favourites', name: 'Favorites', icon: 'linear-like-heart' },
  { id: 'cart',       name: 'Cart',      icon: 'linear-shopping-ecommerce-bag4' },
  { id: 'profile',    name: 'Profile',   icon: 'linear-users-user' },
];

export const navbar = current => `
  <nav class="k-nav" aria-label="Головна навігація">
    ${TABS.map(t => `
      <button class="k-tab" type="button" data-go="${t.id}"
              ${t.id === current ? 'aria-current="page"' : ''}>
        ${icon(t.icon)}${t.id === 'cart' ? '<span class="k-tab-badge" hidden></span>' : ''}
        <span>${esc(t.name)}</span>
      </button>`).join('')}
  </nav>`;

// The count on the Cart tab. It is painted from the basket wherever a navbar is
// drawn, so every screen that has a tab bar agrees about how many items there
// are — without each of those screens having to know the basket exists.
//
// The badge SWELLS when the count differs from the count a badge last showed.
// The dish screen and the cart have no tab bar, so the number changes off
// screen and is first seen on the next tabbed screen -- a fresh node, painted
// once. Without this, an item added on the dish screen changed a "2" to a "3"
// somewhere in the corner and nobody saw it land. The remembered count is the
// last one PAINTED, not the last one in the basket, so the swell happens on
// the screen where the change becomes visible and never on a first paint.
let lastShown = null;

export function paintBasketBadge(root = document){
  const badge = root.querySelector('.k-tab-badge');
  if (!badge) return;
  const n = basketCount();
  badge.textContent = n > 99 ? '99+' : String(n);
  badge.hidden = n === 0;
  // Announced as part of the tab's name rather than as a loose number.
  badge.setAttribute('aria-label', `${n} у кошику`);
  if (lastShown !== null && lastShown !== n && n > 0){
    // Removed and re-added so a second change replays it; the read in between
    // is what forces the browser to notice the removal (one 18px element).
    badge.classList.remove('is-bump');
    void badge.offsetWidth;
    badge.classList.add('is-bump');
  }
  lastShown = n;
}

// A basket that changes under an open tabbed screen -- another tab of the app,
// or the cart's own stepper -- repaints the badge in place.
onBasketChange(() => paintBasketBadge());

// iOS Safari applies `:active` to a tapped element only when the page has
// registered interest in touches; without this listener every press response
// in kit.css is silently dropped on the one platform it matters most on. The
// listener is passive, so it costs the scroll nothing.
document.addEventListener('touchstart', () => {}, { passive: true });

// ── A plain screen header: back arrow, centred title, optional right slot ──
export const topBar = (title, right = '') => `
  <header class="k-top">
    <button class="k-top-btn" type="button" data-back aria-label="Назад">
      ${icon('arrow-left')}
    </button>
    <h1 class="k-top-title">${esc(title)}</h1>
    <span class="k-top-right">${right}</span>
  </header>`;

// ── An order line ──────────────────────────────────────────────────────────
// The same 327 card on My Cart (1:2937), Track Order (1:7331) and E-Receipt
// (1:4687): an 84px plate, the name, "kind · variant", the price and the
// add-on chips. What differs is the foot -- a stepper in the cart, nothing in
// the other two -- so the foot is the caller's.
export const orderLine = (l, { foot = '', attrs = '' } = {}) => `
  <div class="k-box k-line-card" ${attrs}>
    <div class="k-line">
      <div class="k-line-img">${plate(l.name)}</div>
      <div class="k-line-body">
        <span class="k-line-name">${esc(l.name)}</span>
        <span class="k-line-sub">${esc(l.kind)}${icon('ellipse3293')}${esc(l.variant)}</span>
        <span class="k-line-price">${esc(l.price)}</span>
      </div>
    </div>
    ${l.addons?.length ? `
      <div class="k-addons">
        <p>Add-ons</p>
        <div class="k-tags">${l.addons.map(a => `<span class="k-tag">${esc(a)}</span>`).join('')}</div>
      </div>` : ''}
    ${foot ? `<div class="k-line-foot">${foot}</div>` : ''}
  </div>`;

// ── A row that names something and shows an icon for it ────────────────────
// Order ID on Track Order, and every row of the settings and profile lists.
export const factRow = ({ icon: name, title, sub, right = '', attrs = '' }) => `
  <div class="k-box" ${attrs}>
    <div class="k-choice">
      <span class="k-choice-ring">${icon(name)}</span>
      <span class="k-choice-body">
        <span class="k-choice-t">${esc(title)}</span>
        ${sub ? `<span class="k-choice-s">${esc(sub)}</span>` : ''}
      </span>
      ${right}
    </div>
  </div>`;

// ── A settings list ────────────────────────────────────────────────────────
// The row Profile (1:9321), Settings (1:10015), Manage Address (1:9629) and
// every other list screen in the kit is made of: a 42px round icon chip, a 16px
// label, a 21px chevron, and a rule between rows but not after the last.
//
// `to` is a route; `tone: 'danger'` is the Log out row, which the frame draws
// in the same grey as the rest -- the colour here is dowiz's, because a row
// that signs you out should not look like a row that opens a page.
//
// `href` is a destination OFF this app -- a telephone number, the venue's own
// site. It is an ANCHOR, not a button, because `tel:` is the phone's job and a
// button that a script has to turn into a call cannot be long-pressed, copied
// or opened in another tab. A row with neither `to` nor `href` is not drawn as
// a control at all: it is a line of information, and the interaction gate is
// right to call a tappable one that leads nowhere dead.
const rowBody = r => `
        <span class="k-row-ic">${icon(r.icon)}</span>
        <span class="k-row-label">${esc(r.label)}</span>
        ${r.value ? `<span class="k-row-value">${esc(r.value)}</span>` : ''}
        ${r.right || `<span class="k-row-chev">${icon('arrow-right')}</span>`}`;

export const menuList = rows => `
  <div class="k-list">
    ${rows.map(r => {
      const cls = `k-row${r.tone === 'danger' ? ' is-danger' : ''}`;
      if (r.href) return `
      <a class="${cls}" href="${esc(r.href)}" ${r.external ? 'target="_blank" rel="noopener"' : ''}
         ${r.attrs || ''}>${rowBody(r)}</a>`;
      if (!r.to && !r.attrs) return `
      <div class="${cls} is-static">${rowBody(r)}</div>`;
      return `
      <button class="${cls}" type="button"
              ${r.to ? `data-go="${esc(r.to)}"` : ''} ${r.attrs || ''}>${rowBody(r)}</button>`;
    }).join('')}
  </div>`;
