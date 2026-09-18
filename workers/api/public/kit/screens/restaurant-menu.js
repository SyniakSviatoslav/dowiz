// Restaurant Details — Figma nodes 1:1432 (Menu), 1:1626 (About),
// 1:1788 (Gallery), 1:1896 (Review); light set 1:11849 and up.
//
// The four are ONE frame in the file with one tab selected, so they are one
// screen here with a tab strip, not four modules that share a header. The tab
// is in the route (`#/restaurant-menu?tab=about`) so a deep link and the back
// button both land on the pane that was open.

import { icon, esc } from '/kit/app.js';
import { plate, heroButtons, paintPlates, ctaBar } from '/kit/parts.js';
import { menu, venue, formatMoney } from '/kit/data.js';

const VENUE = {
  name: 'The Savory Spot',
  tags: 'Italian, American, Mexican, Japanese...',
  rating: 4.8,
  ratingCount: '5.4K',
  address: '88 Bedford Street, New York, NY 10014',
  mins: '10 min', miles: '2.5 Miles', delivery: 'free Delivery',
  itemCount: 48,
};

const FILTERS = [
  { id: 'all',  name: 'All' },
  { id: 'veg',  icon: 'veg-mark' },
  { id: 'non',  icon: 'nonveg-mark' },
  { id: 'best', name: 'Best Seller' },
  { id: 'top',  name: 'Top Rated' },
];

const ITEMS = [
  { id: 'm1', name: 'Italian Pizza',  off: '20% OFF', price: 1400, rating: 4.8, veg: true,  qty: 0 },
  { id: 'm2', name: 'Veggie Pasta',   off: '10% OFF', price: 1600, rating: 4.8, veg: true,  qty: 1 },
  { id: 'm3', name: 'Mexican Tacos',  off: '15% OFF', price: 2200, rating: 4.7, veg: false, qty: 0 },
  { id: 'm4', name: 'Veg Burger',     off: '10% OFF', price: 1200, rating: 4.9, veg: true,  qty: 0 },
];

const TABS = [
  { id: 'menu',    name: 'Menu' },
  { id: 'about',   name: 'About' },
  { id: 'gallery', name: 'Gallery' },
  { id: 'review',  name: 'Review' },
];

// What is on screen: the frame's own venue and items until the host names a
// hub, then that hub's. Kept in one place so the filter, the search and the
// cards cannot end up reading three different lists.
let PLACE = VENUE;
let LIST = ITEMS;

const money = cents => PLACE.currency
  ? formatMoney(cents, PLACE.currency)
  : '$' + (cents / 100).toFixed(2);

const state = { tab: 'menu', filter: 'all', qty: new Map(ITEMS.map(i => [i.id, i.qty])) };

function adopt(body){
  const v = venue(body);
  if (!v) { PLACE = VENUE; LIST = ITEMS; return; }
  const products = (body.categories || [])
    .flatMap(c => (c.products || []).map(p => ({ p, c })));
  PLACE = {
    ...VENUE,
    name: v.name,
    tags: (body.categories || []).map(c => c.name).slice(0, 4).join(', '),
    address: v.address || '',
    mins: v.eta ? `${v.eta} min` : '',
    miles: '',
    delivery: v.deliveryFee === 0 ? 'free Delivery' : '',
    itemCount: products.length,
    currency: v.currency,
    // A venue publishes no rating, and dowiz does not score anyone
    // (DECISIONS.md): the line is left out rather than filled in.
    rating: null,
    ratingCount: null,
  };
  LIST = products.map(({ p, c }) => ({
    id: p.id,
    name: p.name,
    off: null,
    price: p.price,
    imageUrl: p.imageUrl || null,
    rating: null,
    // "Vegetarian" is inferred from the allergen list the venue actually
    // publishes, not asserted: a dish with no allergens declared is not
    // claimed to be anything.
    veg: (p.allergens || []).length
      ? !(p.allergens || []).some(a => /fish|milk|egg|molluscs|crustacean/.test(a))
      : null,
    available: p.available !== false,
  }));
  state.qty = new Map(LIST.map(i => [i.id, 0]));
}

// ── Panes ──────────────────────────────────────────────────────────────────

const menuCard = it => {
  const n = state.qty.get(it.id) || 0;
  return `
  <article class="k-mcard" data-item="${esc(it.id)}">
    <div class="k-photo">${plate(it.name)}</div>
    ${it.off ? `<span class="k-off k-pill-off">${esc(it.off)}</span>` : ''}
    <button class="k-heart" type="button" aria-pressed="false"
            aria-label="У обране: ${esc(it.name)}">${icon('heart-outline')}</button>
    <div class="k-mcard-body">
      ${it.veg === null ? '' : `<span>${icon(it.veg ? 'veg-mark' : 'nonveg-mark')}</span>`}
      <div class="k-mcard-top">
        <span class="k-mcard-name">${esc(it.name)}</span>
        ${it.rating ? `<span class="k-rate">${icon('star2')}${it.rating}</span>` : ''}
      </div>
      <div class="k-mcard-foot">
        <b>${money(it.price)}</b>
        ${n
          ? `<span class="k-qty k-qty-sm">
               <button type="button" data-item-step="-1" aria-label="Менше">${icon('minus-glyph')}</button>
               <output>${n}</output>
               <button type="button" data-item-step="1" aria-label="Більше">${icon('plus-glyph')}</button>
             </span>`
          : `<button class="k-add" type="button" data-item-step="1"
                     aria-label="Додати ${esc(it.name)}">${icon('plus-glyph')}</button>`}
      </div>
    </div>
  </article>`;
};

const menuPane = () => `
  <div class="k-menu-head">
    <p class="t-section">Menu <span class="k-accent">(${PLACE.itemCount} Items)</span></p>
    <button class="k-seeall" type="button" data-go="popular-dishes">View Full Menu</button>
  </div>
  <label class="k-field k-field-flat">
    ${icon('search')}
    <input id="q" type="search" placeholder="Search Items" aria-label="Пошук у меню">
  </label>
  <div class="k-chips" role="group" aria-label="Фільтр меню">
    ${FILTERS.map(f => `
      <button class="k-chip${f.id === state.filter ? ' on' : ''}" type="button"
              data-filter="${esc(f.id)}" aria-pressed="${f.id === state.filter}"
              ${f.name ? '' : `aria-label="${f.id === 'veg' ? 'Вегетаріанське' : 'З м’ясом'}"`}>
        ${f.icon ? icon(f.icon) : ''}${f.name ? esc(f.name) : ''}
      </button>`).join('')}
  </div>
  <div class="k-grid" id="items">${LIST.map(menuCard).join('')}</div>`;

const aboutPane = () => `
  <h2 class="k-pick-h">About</h2>
  <p class="k-desc is-open">${esc(PLACE.name)} — ${esc(PLACE.tags)}</p>
  <div class="k-rule"></div>
  ${PLACE.address ? `<p class="k-venue-addr">${icon('pin-17')}${esc(PLACE.address)}</p>` : ''}
  ${[PLACE.mins, PLACE.miles, PLACE.delivery].filter(Boolean).length ? `
  <p class="k-venue-addr">${icon('clock-17')}${
    [PLACE.mins, PLACE.miles, PLACE.delivery].filter(Boolean).map(esc).join(' · ')}</p>` : ''}`;

// The venue's own photographs, and each one opens the dish it is of. A cell
// with no photograph behind it is a TILE, not a button: six tappable blanks is
// what the interaction gate reported here as `DEAD button.k-gal-cell`, once per
// cell, and a control that leads nowhere is the defect, not the empty gallery.
const galleryPane = () => {
  const shots = (LIST || []).filter(it => it.imageUrl);
  if (!shots.length) return `
  <div class="k-grid k-gal-grid">
    ${[0,1,2,3,4,5].map(i => `
      <div class="k-gal-cell is-empty" role="presentation">
        ${plate(PLACE.name + i)}
      </div>`).join('')}
  </div>
  <p class="k-page-p">Заклад ще не завантажив фотографій.</p>`;

  return `
  <div class="k-grid k-gal-grid">
    ${shots.map(it => `
      <button class="k-gal-cell" type="button" aria-label="${esc(it.name)}"
              data-go="item-details?id=${encodeURIComponent(it.id)}">
        <img src="${esc(it.imageUrl)}" alt="${esc(it.name)}" loading="lazy">
      </button>`).join('')}
  </div>`;
};

const reviewPane = () => `
  <div class="k-mcard-top">
    ${PLACE.rating
      ? `<span class="k-rate-18">${icon('star-18')}<b>${PLACE.rating}</b>${
          PLACE.ratingCount ? ` (${esc(PLACE.ratingCount)})` : ''}</span>`
      : '<span class="t-body muted2">Ще без оцінок</span>'}
    <button class="k-seeall" type="button" data-go="leave-review">Leave Review</button>
  </div>
  <p class="k-desc is-open muted2">Відгуки підтягуються з хабу закладу.</p>`;

const PANES = { menu: menuPane, about: aboutPane, gallery: galleryPane, review: reviewPane };

// ── The screen ─────────────────────────────────────────────────────────────

export async function render(params, routeName = 'restaurant-menu'){
  adopt(await menu('uk'));
  // The route may name the tab (`restaurant-about`) or carry it as a query
  // (`restaurant-menu?tab=about`); the design links both ways.
  const fromRoute = routeName.replace(/^restaurant-/, '');
  state.tab = PANES[params?.get('tab')] ? params.get('tab')
            : PANES[fromRoute] ? fromRoute : 'menu';
  return `
  <div class="k-hero">
    <div class="k-hero-img">${plate(PLACE.name)}</div>
    ${heroButtons()}
    <div class="k-gal" role="group" aria-label="Галерея">
      ${[0,1,2,3,4].map(i => `
        <!-- A thumbnail opens the venue's full gallery. It used to carry
             data-tab="gallery" -- the tab it is already inside -- so tapping it
             re-rendered the identical pane and read as a dead control. -->
        <button class="k-gal-th" type="button" data-go="gallery" aria-label="Фото ${i + 1}">
          ${plate(PLACE.name + 'g' + i)}
        </button>`).join('')}
      <span class="k-gal-more">${icon('gallery-more')}</span>
    </div>
  </div>

  <div class="wrap">
    <div class="k-mcard-top">
      <h1 class="k-venue-h">${esc(PLACE.name)}</h1>
      ${PLACE.rating ? `<span class="k-rate-18">${icon('star-18')}<b>${PLACE.rating}</b>${
        PLACE.ratingCount ? ` (${esc(PLACE.ratingCount)})` : ''}</span>` : ''}
    </div>
    <p class="muted2 t-body">${esc(PLACE.tags)}</p>
    <div class="k-rule"></div>
    ${PLACE.address ? `<p class="k-venue-addr">${icon('pin-17')}${esc(PLACE.address)}</p>` : ''}
    ${[PLACE.mins, PLACE.miles, PLACE.delivery].filter(Boolean).length ? `
    <p class="k-venue-addr">${icon('clock-17')}${
      [PLACE.mins, PLACE.miles, PLACE.delivery].filter(Boolean).map(esc)
        .join(icon('ellipse3293'))}</p>` : ''}

    <div class="k-tabs" role="tablist">
      ${TABS.map(t => `
        <button type="button" role="tab" data-tab="${esc(t.id)}"
                aria-selected="${t.id === state.tab}">${esc(t.name)}</button>`).join('')}
    </div>

    <div id="pane" role="tabpanel">${PANES[state.tab]()}</div>
  </div>

  ${ctaBar('Book a Table', { to: 'book-a-table' })}`;
}

export function bind(root){
  root.addEventListener('click', e => {
    const tab = e.target.closest('[data-tab]');
    if (tab){
      state.tab = tab.dataset.tab;
      for (const b of root.querySelectorAll('[role="tab"]'))
        b.setAttribute('aria-selected', String(b.dataset.tab === state.tab));
      const pane = root.querySelector('#pane');
      pane.innerHTML = PANES[state.tab]();
      paintPlates(pane);
      // The route carries the tab so a reload and the back button agree with
      // what is on screen.
      history.replaceState(null, '', `#/restaurant-menu?tab=${state.tab}`);
      return;
    }

    const step = e.target.closest('[data-item-step]');
    if (step){
      e.stopPropagation();
      const card = step.closest('[data-item]');
      const id = card.dataset.item;
      const next = Math.max(0, Math.min(99, (state.qty.get(id) || 0) + Number(step.dataset.itemStep)));
      state.qty.set(id, next);
      const item = LIST.find(i => i.id === id);
      card.outerHTML = menuCard(item);
      paintPlates(root.querySelector(`[data-item="${CSS.escape(id)}"]`));
      return;
    }

    // The heart is NOT handled here. It used to be -- this screen was the one
    // place in the kit that wired it -- and because it stopped the event the
    // shell's own handler never ran, so the same heart saved nothing while the
    // hearts on every other screen did nothing at all. One handler, in app.js,
    // and it persists.

    const chip = e.target.closest('[data-filter]');
    if (chip){
      state.filter = chip.dataset.filter;
      for (const b of root.querySelectorAll('[data-filter]')){
        const on = b === chip;
        b.setAttribute('aria-pressed', String(on));
        b.classList.toggle('on', on);
      }
      applyFilter(root);
    }
  });

  root.addEventListener('input', e => {
    if (e.target.id === 'q') applyFilter(root);
  });
}

function applyFilter(root){
  const needle = (root.querySelector('#q')?.value || '').trim().toLowerCase();
  for (const card of root.querySelectorAll('[data-item]')){
    const item = LIST.find(i => i.id === card.dataset.item);
    const byKind = state.filter === 'all'
      || (state.filter === 'veg' && item.veg === true)
      || (state.filter === 'non' && item.veg === false)
      // "Best Seller" and "Top Rated" have no field in the frame's data; rating
      // is the only ordering the kit shows, so it is what they mean here.
      || (state.filter === 'best' && item.rating >= 4.8)
      || (state.filter === 'top' && item.rating >= 4.9)
      // With no rating published there is nothing to rank by, so those two
      // chips show everything rather than silently showing nothing.
      || ((state.filter === 'best' || state.filter === 'top') && item.rating == null);
    const byText = !needle || item.name.toLowerCase().includes(needle);
    card.hidden = !(byKind && byText);
  }
}
