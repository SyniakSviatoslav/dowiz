// Listings — Popular Dishes 1:9038, Popular Restaurants 1:9181, Exclusive
// Offers 1:8954, Category 1:8794, My Favourites 1:6853 / 1:7033.
// Light set: 1:19473, 1:19618, 1:19387, 1:19225, 1:17270 / 1:17451.
//
// Five frames, four of them built from cards this kit already draws: the menu
// card (156x213), the restaurant card (327x249) and the offer card (327x157).
// Only Category brings a new shape, a 64px circle with a 12/400 label. So this
// is one module that says which card a route uses, not five that each re-draw
// a card and then disagree about its corner radius.
//
// The dishes are the VENUE'S when the host names one. `catalogue()` hands back
// every category and every dish, sold-out included, and this module is where
// the kit sorts and narrows them -- so Search, Home and Filter import the sort
// table and the card pieces from here rather than each keeping a copy that
// would order "Price ↑" differently.

import { icon, esc } from '/kit/app.js';
import { topBar, plate, paintPlates, navbar } from '/kit/parts.js';
import { menu, catalogue, formatMoney } from '/kit/data.js';

// `priceMinor` beside the frame's printed price, so the frame's own six dishes
// sort the same way the venue's do rather than by parsing "$12.00" back apart.
const DISHES = [
  { id: 'p1', name: 'ItaliaCrisp Pizza', off: '20% OFF', price: '$12.00', priceMinor: 1200, rating: 4.8, veg: true },
  { id: 'p2', name: 'Veggie Pasta',      off: '10% OFF', price: '$16.00', priceMinor: 1600, rating: 4.8, veg: true },
  { id: 'p3', name: 'Mexican Tacos',     off: '15% OFF', price: '$22.00', priceMinor: 2200, rating: 4.7, veg: false },
  { id: 'p4', name: 'Veg Burger',        off: '10% OFF', price: '$12.00', priceMinor: 1200, rating: 4.9, veg: true },
  { id: 'p5', name: 'Smoky Ribs',        off: '25% OFF', price: '$28.00', priceMinor: 2800, rating: 4.6, veg: false },
  { id: 'p6', name: 'Sushi Platter',     off: '15% OFF', price: '$34.00', priceMinor: 3400, rating: 4.9, veg: false },
];

const PLACES = [
  { id: 'q1', name: 'Brooklyn Bites', off: '25% OFF', rating: 4.9,
    tags: 'Italian, American, Mexican, Japanese...',
    address: '789 Park Avenue, New York, NY 10021, USA',
    mins: 15, miles: 3.5, delivery: 'Free Delivery' },
  { id: 'q2', name: 'The Savory Spot', off: '10% OFF', rating: 4.8,
    tags: 'Italian, American, Chinese, Japanese...',
    address: '88 Bedford Street, New York, NY 10014',
    mins: 10, miles: 2.5, delivery: 'Free Delivery' },
  { id: 'q3', name: 'Food Fusion Hub', off: '15% OFF', rating: 5.0,
    tags: 'Asian, Fusion, Street Food...',
    address: '600 Lexington Ave, New York, NY 10022',
    mins: 15, miles: 3.5, delivery: 'Free Delivery' },
];

const OFFERS = [
  { tag: 'Weekend Offers!', head: 'Get Special Offers', pct: 30 },
  { tag: 'First Order',     head: 'Save On Your First',  pct: 50 },
  { tag: 'Late Night',      head: 'Midnight Cravings',   pct: 20 },
];

const CATEGORIES = [
  { id: 'burger',  name: 'Burger',   icon: 'linear-shopping-ecommerce-bag4' },
  { id: 'pizza',   name: 'Pizza',    icon: 'salad' },
  { id: 'noodles', name: 'Noodles',  icon: 'box-20' },
  { id: 'drinks',  name: 'Drinks',   icon: 'money' },
  { id: 'sushi',   name: 'Sushi',    icon: 'reserve-20' },
  { id: 'grill',   name: 'Grill',    icon: 'scooter-20' },
  { id: 'sweets',  name: 'Sweets',   icon: 'ticket' },
  { id: 'healthy', name: 'Healthy',  icon: 'salad' },
];

const FAV_TABS = [{ id: 'dishes', name: 'Dishes' }, { id: 'places', name: 'Restaurants' }];

// ── Ordering, shared with Search and Filter ────────────────────────────────
// One table, because "Price ↑" on the listing and "Price: low to high" on the
// filter screen are the same order and must carry the same id through the
// route (`?sort=price-asc`), or Apply on one screen lands on a chip the other
// screen does not recognise.
export const SORTS = [
  { id: 'menu',       name: 'Recommended', long: 'Recommended' },
  { id: 'price-asc',  name: 'Price ↑',     long: 'Price: low to high' },
  { id: 'price-desc', name: 'Price ↓',     long: 'Price: high to low' },
  { id: 'name',       name: 'A–Z',         long: 'Name A–Z' },
];

export const sortId = s => SORTS.some(x => x.id === s) ? s : 'menu';

export function sortItems(items, sort){
  const out = [...items];
  switch (sortId(sort)){
    case 'price-asc':  return out.sort((a, b) => a.priceMinor - b.priceMinor);
    case 'price-desc': return out.sort((a, b) => b.priceMinor - a.priceMinor);
    // Locale-aware and case-blind: "ebi" and "Ebi" are one word to a customer,
    // and a plain `<` would put every capital before every lower-case name.
    case 'name':       return out.sort((a, b) =>
      a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
    // The venue's own order -- the categories as the hub sorts them, the dishes
    // as the venue listed them. `catalogue()` already delivers that order.
    default:           return out;
  }
}

// ── Card pieces, shared with Home and Search ───────────────────────────────

// The venue's photograph where there is one; otherwise the plate keyed to the
// name, so the same dish is the same colour on every screen that has no photo.
// The name rides on the image so `guardPhotos` can draw that same plate if the
// file turns out not to be a picture.
export const photo = d => d.imageUrl
  ? `<img src="${esc(d.imageUrl)}" alt="" loading="lazy" decoding="async"
          data-plate="${esc(d.name)}">`
  : plate(d.name);

// A PHOTO THAT DOES NOT DECODE BECOMES THE PLATE.
//
// Measured on dubin-sushi: the hub serves item-01's photograph as 200 OK,
// image/jpeg, 4012 bytes -- and the bytes are a JFIF header followed by noise,
// no frame, no scan, no end marker. Chromium reports naturalWidth 0 and draws
// the broken-image glyph on a white box, which is the one thing worse than no
// photo. `onerror` in the markup is inline script and the policy forbids it, so
// the screen listens once, in the capture phase (error does not bubble), and
// swaps the image for the plate it would have drawn without one.
export function guardPhotos(root){
  root.addEventListener('error', e => {
    const img = e.target;
    if (!(img instanceof HTMLImageElement) || !img.dataset.plate) return;
    const box = img.parentElement;
    img.insertAdjacentHTML('afterend', plate(img.dataset.plate));
    img.remove();
    paintPlates(box);
  }, true);
}

// A sold-out dish STAYS on the card grid with the venue's reason on it. Hiding
// it is what makes a customer who cannot find yesterday's dish decide the app
// is broken (see products.unavailable_note); the note is the venue's own words
// and only the wording of the default is ours.
export const soldNote = d => d.available === false
  ? `<span class="k-sold-note">${esc(d.unavailableNote || 'Немає в наявності')}</span>`
  : '';

export const soldPill = d => d.available === false
  ? `<span class="k-sold-pill">Sold out</span>`
  : '';

// "Vegetarian" is inferred from the allergen list the venue actually publishes,
// never asserted: a dish with no allergens declared is not claimed to be
// anything, and the mark is left off rather than guessed.
export const vegOf = d => Array.isArray(d.allergens) && d.allergens.length
  ? !d.allergens.some(a => /fish|milk|egg|molluscs|crustacean/.test(a))
  : (typeof d.veg === 'boolean' ? d.veg : null);

// The sort chips, drawn the way the restaurant screen draws its filter row:
// one row of pills, the chosen one orange. A `<select>` is not in this kit.
export const sortChips = (current, extra = '') => `
  <div class="k-chips" role="group" aria-label="Сортування">
    ${SORTS.map(s => `
      <button class="k-chip${s.id === sortId(current) ? ' on' : ''}" type="button"
              data-sort="${esc(s.id)}" aria-pressed="${s.id === sortId(current)}">${esc(s.name)}</button>`).join('')}
    ${extra}
  </div>`;

export function paintSortChips(root, current){
  for (const b of root.querySelectorAll('[data-sort]')){
    const on = b.dataset.sort === sortId(current);
    b.setAttribute('aria-pressed', String(on));
    b.classList.toggle('on', on);
  }
}

// ── State ──────────────────────────────────────────────────────────────────
// `cats` is the set of category ids the Popular Dishes grid is narrowed to
// (empty means all), `max` a price ceiling in minor units or null. Both arrive
// from the Filter screen through the route and are kept in the route, so a
// reload and the back button show the same grid the customer was looking at.
const state = {
  tab: 'dishes', category: 'burger', liked: new Set(DISHES.map(d => d.id)),
  sort: 'menu', cats: new Set(), max: null,
};

let CAT = null;   // the venue's catalogue, when the host names one

// The venue's categories in the hub's order. Sorted here, once, because every
// row of chips and cells below draws them and two of those disagreeing about
// order is how "Bowls" is the third chip on one screen and the fifth on another.
const liveCats = () => CAT
  ? [...CAT.categories].sort((a, b) => a.sortOrder - b.sortOrder)
  : [];

// ── Cards ──────────────────────────────────────────────────────────────────

const dishCard = d => {
  const veg = vegOf(d);
  return `
  <article class="k-mcard${d.available === false ? ' is-sold' : ''}" data-dish="${esc(d.id)}"
           data-go="item-details?id=${encodeURIComponent(d.id)}">
    <div class="k-photo">${photo(d)}${soldPill(d)}</div>
    ${d.off ? `<span class="k-off k-pill-off">${esc(d.off)}</span>` : ''}
    <button class="k-heart" type="button" aria-pressed="${state.liked.has(d.id)}"
            aria-label="У обране: ${esc(d.name)}">${icon('heart-outline')}</button>
    <div class="k-mcard-body">
      ${veg == null ? '' : `<span>${icon(veg ? 'veg-mark' : 'nonveg-mark')}</span>`}
      <div class="k-mcard-top">
        <span class="k-mcard-name">${esc(d.name)}</span>
        ${d.rating ? `<span class="k-rate">${icon('star2')}${d.rating}</span>` : ''}
      </div>
      ${soldNote(d)}
      <div class="k-mcard-foot"><b>${esc(d.price)}</b></div>
    </div>
  </article>`;
};

const placeCard = p => `
  <article class="k-rest k-rest-lg" data-place="${esc(p.id)}" data-go="restaurant-menu">
    <div class="k-photo">${plate(p.name)}</div>
    ${p.off ? `<span class="k-off k-pill-off">${esc(p.off)}</span>` : ''}
    <button class="k-heart" type="button" aria-pressed="false"
            aria-label="У обране: ${esc(p.name)}">${icon('heart-outline')}</button>
    <div class="k-rest-body">
      <div class="k-dish-top">
        <span class="t-section">${esc(p.name)}</span>
        ${p.rating ? `<span class="k-rate">${icon('star2')}${p.rating}</span>` : ''}
      </div>
      <p class="k-rest-tags">${esc(p.tags)}</p>
      <p class="k-rest-line">${icon('location')}${esc(p.address)}</p>
      <p class="k-rest-line">${icon('clock')}${p.mins} Min ${icon('ellipse3293')}${p.miles} Miles
        ${icon('ellipse3293')}${esc(p.delivery)}</p>
    </div>
  </article>`;

// An offer is a way to a coupon, and the coupon screen is where a code is
// picked up and carried to the cart. Without that destination the three cards
// on Exclusive Offers were three buttons that did nothing — `DEAD button.k-offer`
// from the interaction gate, once per card.
const offerCard = o => `
  <button class="k-offer" type="button" data-offer="${esc(o.head)}" data-go="coupon">
    <span class="k-offer-art"></span>
    <span class="k-offer-tag">${esc(o.tag)}</span>
    <span class="k-offer-h">${esc(o.head)}</span>
    <span class="k-offer-row">
      <span class="k-offer-upto">Up to</span>
      <span class="k-offer-num">${o.pct}<span class="k-offer-pct">%</span></span>
    </span>
    <span class="k-offer-cta">Get Now</span>
  </button>`;

// The frame's cells carry a glyph each. A venue's category has no glyph of its
// own, so the ring shows the plate keyed to the category's name with its first
// letter over it -- the same rule the dish photo box uses, so "Bowls" is one
// colour everywhere rather than the kit's bag icon standing in for sushi.
const categoryCell = c => `
  <button class="k-cat-cell" type="button" data-cat="${esc(c.id)}"
          aria-pressed="${c.id === state.category}">
    <span class="k-cat-ring${c.icon ? '' : ' is-plate'}">${c.icon
      ? icon(c.icon)
      : `${plate(c.name)}<span class="k-cat-initial">${esc([...c.name][0] || '')}</span>`}</span>
    <span class="k-cat-name">${esc(c.name)}</span>
  </button>`;

const categoryChip = c => `
  <button class="k-chip${state.cats.has(c.id) ? ' on' : ''}" type="button"
          data-cat="${esc(c.id)}" aria-pressed="${state.cats.has(c.id)}">${esc(c.name)}</button>`;

const empty = () => `
  <div class="k-empty">${icon('search')}
    <p class="t-title">Нічого не знайшли</p>
    <p class="t-body muted2">Спробуйте іншу категорію або ціну.</p></div>`;

// ── Routes ─────────────────────────────────────────────────────────────────

const LISTINGS = {
  'popular-dishes':      { title: 'Popular Dishes',      kind: 'dishes' },
  'popular-restaurants': { title: 'Popular Restaurants', kind: 'places' },
  'exclusive-offers':    { title: 'Exclusive Offers',    kind: 'offers' },
  category:              { title: 'Category',            kind: 'categories' },
  'my-favourites':       { title: 'My Favourites',       kind: 'favourites', search: true },
};

// The Popular Dishes grid: every dish the venue sells, narrowed by the chosen
// categories and the price ceiling, in the chosen order. Sold-out dishes are
// NOT filtered here -- see `soldNote`.
const dishRows = () => {
  if (!CAT) return DISHES;
  return sortItems(CAT.items.filter(d =>
    (!state.cats.size || state.cats.has(d.categoryId))
    && (state.max == null || d.priceMinor <= state.max)), state.sort);
};

const grid = rows => rows.length
  ? `<div class="k-grid">${rows.map(dishCard).join('')}</div>`
  : empty();

// The dishes of the one category chosen on the Category screen.
const categoryRows = () => {
  const c = liveCats().find(x => x.id === state.category) || liveCats()[0];
  return c ? sortItems(c.items, state.sort) : [];
};

const categoryHead = () => {
  const c = liveCats().find(x => x.id === state.category) || liveCats()[0];
  return c ? `<h2 class="t-section" id="catName">${esc(c.name)}
    <span class="k-accent">(${c.items.length})</span></h2>` : '';
};

// A price ceiling arrived from the Filter screen is shown as a chip of its own
// with a ×, because a grid that is silently missing every dish over 15 ALL is a
// grid the customer will report as incomplete.
const maxChip = () => state.max == null || !CAT ? '' : `
  <button class="k-chip on" type="button" data-clear-max
          aria-label="Прибрати межу ціни ${esc(formatMoney(state.max, CAT.currency))}">
    ≤ ${esc(formatMoney(state.max, CAT.currency))}<span class="ic-wrap">${icon('chip-remove')}</span>
  </button>`;

const body = kind => {
  switch (kind){
    case 'dishes':
      if (!CAT) return `<div class="k-grid">${DISHES.map(dishCard).join('')}</div>`;
      return `
        <div class="k-chips" role="group" aria-label="Категорії">
          <button class="k-chip${state.cats.size ? '' : ' on'}" type="button" data-cat=""
                  aria-pressed="${!state.cats.size}">All</button>
          ${liveCats().map(categoryChip).join('')}
        </div>
        ${sortChips(state.sort, maxChip())}
        <div id="grid" class="k-block">${grid(dishRows())}</div>`;
    case 'places':
      return PLACES.map(placeCard).join('');
    case 'offers':
      return OFFERS.map(offerCard).join('');
    case 'categories':
      if (!CAT) return `<div class="k-cat-grid">${CATEGORIES.map(categoryCell).join('')}</div>`;
      return `
        <div class="k-cat-grid">${liveCats().map(categoryCell).join('')}</div>
        <div class="k-sec">${categoryHead()}</div>
        ${sortChips(state.sort)}
        <div id="grid" class="k-block">${grid(categoryRows())}</div>`;
    case 'favourites': {
      const rows = state.tab === 'dishes'
        ? (CAT ? CAT.items : DISHES).filter(d => state.liked.has(d.id))
        : PLACES;
      if (!rows.length)
        return `<div class="k-empty">${icon('heart-outline')}
          <p class="t-title">Порожньо</p>
          <p class="t-body muted2">Те, що ви вподобаєте, з’явиться тут.</p></div>`;
      return state.tab === 'dishes'
        ? `<div class="k-grid">${rows.map(dishCard).join('')}</div>`
        : rows.map(placeCard).join('');
    }
    default:
      return '';
  }
};

// What the route says about the grid, read back out so a deep link, a reload
// and Apply on the Filter screen all land on the same narrowing.
function readParams(params, kind){
  state.sort = sortId(params?.get('sort'));
  const ids = new Set(liveCats().map(c => c.id));
  if (kind === 'dishes'){
    state.cats = new Set((params?.get('cat') || '').split(',').filter(id => ids.has(id)));
    const max = Number(params?.get('max'));
    state.max = params?.has('max') && Number.isFinite(max) && max > 0 ? max : null;
  }
  if (kind === 'categories'){
    const want = params?.get('cat');
    state.category = CAT
      ? (ids.has(want) ? want : liveCats()[0]?.id)
      : (CATEGORIES.some(c => c.id === want) ? want : 'burger');
  }
}

// The route carries the narrowing so a reload and the back button agree with
// what is on screen; `replaceState` rather than a hash assignment, because a
// hash change re-routes and that would rebuild the screen under the thumb.
function writeParams(routeName){
  const q = new URLSearchParams();
  if (routeName === 'category' && state.category) q.set('cat', state.category);
  if (routeName === 'popular-dishes' && state.cats.size) q.set('cat', [...state.cats].join(','));
  if (routeName === 'popular-dishes' && state.max != null) q.set('max', String(state.max));
  if (state.sort !== 'menu') q.set('sort', state.sort);
  const s = q.toString();
  history.replaceState(null, '', `#/${routeName}${s ? '?' + s : ''}`);
}

export async function render(params, routeName = 'popular-dishes'){
  const l = LISTINGS[routeName] || LISTINGS['popular-dishes'];
  if (l.kind === 'dishes' || l.kind === 'favourites' || l.kind === 'categories'){
    CAT = catalogue(await menu('uk'));
    if (CAT && !CAT.items.length) CAT = null;   // a hub with no dishes keeps the frame
    if (CAT && l.kind === 'favourites') state.liked = new Set(CAT.items.slice(0, 4).map(d => d.id));
  }
  readParams(params, l.kind);
  if (l.kind === 'favourites')
    state.tab = FAV_TABS.some(t => t.id === params?.get('tab')) ? params.get('tab') : 'dishes';

  return `
  ${topBar(l.title, l.search
    ? `<button class="k-top-btn" type="button" data-go="search" aria-label="Пошук">
         ${icon('search')}</button>` : '')}
  <div class="wrap" data-listing="${esc(routeName)}">
    ${l.kind === 'favourites' ? `
      <div class="k-tabs k-tabs-wide" role="tablist">
        ${FAV_TABS.map(t => `
          <button type="button" role="tab" data-tab="${esc(t.id)}"
                  aria-selected="${t.id === state.tab}">${esc(t.name)}</button>`).join('')}
      </div>` : ''}
    <div id="listing" class="k-block">${body(l.kind)}</div>
  </div>
  ${navbar(l.kind === 'favourites' ? 'favourites' : '')}`;
}

export function bind(root){
  guardPhotos(root);
  const host = root.querySelector('[data-listing]');
  const routeName = host.dataset.listing;
  const kind = LISTINGS[routeName].kind;
  const repaint = () => {
    const el = root.querySelector('#listing');
    el.innerHTML = body(kind);
    paintPlates(el);
  };
  // Only the grid is redrawn when a chip changes. Redrawing the chip rows too
  // would reset their horizontal scroll, and the chip the customer just tapped
  // at the far right would jump back under the left edge.
  const repaintGrid = () => {
    const el = root.querySelector('#grid');
    if (!el) return repaint();
    el.innerHTML = grid(kind === 'categories' ? categoryRows() : dishRows());
    paintPlates(el);
    const h = root.querySelector('#catName');
    if (h) h.outerHTML = categoryHead();
    writeParams(routeName);
  };

  root.addEventListener('click', e => {
    const heart = e.target.closest('.k-heart');
    if (heart){
      e.stopPropagation();
      const card = heart.closest('[data-dish]');
      const on = heart.getAttribute('aria-pressed') !== 'true';
      heart.setAttribute('aria-pressed', String(on));
      if (card) on ? state.liked.add(card.dataset.dish) : state.liked.delete(card.dataset.dish);
      // On My Favourites, un-liking removes the card: leaving it there would
      // claim the list still holds it.
      if (kind === 'favourites' && !on) repaint();
      return;
    }

    const tab = e.target.closest('[data-tab]');
    if (tab){
      state.tab = tab.dataset.tab;
      for (const b of root.querySelectorAll('[role="tab"]'))
        b.setAttribute('aria-selected', String(b.dataset.tab === state.tab));
      history.replaceState(null, '', `#/my-favourites?tab=${state.tab}`);
      return repaint();
    }

    const sort = e.target.closest('[data-sort]');
    if (sort){
      state.sort = sortId(sort.dataset.sort);
      paintSortChips(root, state.sort);
      return repaintGrid();
    }

    if (e.target.closest('[data-clear-max]')){
      state.max = null;
      e.target.closest('[data-clear-max]').remove();
      return repaintGrid();
    }

    const cat = e.target.closest('[data-cat]');
    if (cat){
      if (kind === 'dishes'){
        // The chips are a multi-select with "All" meaning none chosen. Choosing
        // a category drops "All"; choosing "All" drops every category. A row
        // where "All" and "Bowls" are both orange is a row that says two things.
        const id = cat.dataset.cat;
        if (!id) state.cats.clear();
        else state.cats.has(id) ? state.cats.delete(id) : state.cats.add(id);
        for (const b of root.querySelectorAll('[data-cat]')){
          const on = b.dataset.cat ? state.cats.has(b.dataset.cat) : !state.cats.size;
          b.setAttribute('aria-pressed', String(on));
          b.classList.toggle('on', on);
        }
        return repaintGrid();
      }
      state.category = cat.dataset.cat;
      for (const b of root.querySelectorAll('[data-cat]'))
        b.setAttribute('aria-pressed', String(b.dataset.cat === state.category));
      if (CAT) repaintGrid();
    }
  });
}
