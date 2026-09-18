// Search — Figma nodes 1:5703 (empty), 1:5938 (Dishes), 1:6104 (Restaurant);
// light set 1:16113, 1:16349, 1:16516.
//
// Three frames of one screen in three states. Empty it shows what was searched
// before; with a query it shows what matched, split into the two kinds of thing
// a query can match. Three modules would each own a copy of the search field
// and the recents, and the recents would stop agreeing about what is recent.
//
// With a venue behind it the query runs over the venue's dishes -- name AND
// description, because a sushi menu names its rolls "Coral Sake" and the
// customer is looking for "avokado" -- and the recents are the customer's own,
// kept in the browser, not the frame's six words.

import { icon, esc } from '/kit/app.js';
import { plate, paintPlates, navbar } from '/kit/parts.js';
import { menu, catalogue } from '/kit/data.js';
import { photo, soldNote, soldPill, vegOf, sortChips, sortItems, sortId, paintSortChips,
         guardPhotos } from '/kit/screens/listing.js';

const RECENT = ['Pizza', 'Tacos', 'Breakfast', 'Burger', 'Noodles', 'Pasta'];

const DISHES = [
  { id: 'd1', name: 'Mexican Tacos', off: '15% OFF', mins: 20, miles: 4.2, rating: 4.7,
    price: '$22.00', priceMinor: 2200, veg: false },
  { id: 'd2', name: 'Veg Burger', off: '10% OFF', mins: 15, miles: 3.5, rating: 4.9,
    price: '$12.00', priceMinor: 1200, veg: true },
  { id: 'd3', name: 'ItaliaCrisp Pizza', off: '20% OFF', mins: 15, miles: 3.5, rating: 4.8,
    price: '$12.00', priceMinor: 1200, veg: true },
];

const PLACES = [
  { id: 'r1', name: 'The Savory Spot', rating: 4.8, mins: 10, miles: 2.5,
    address: '88 Bedford Street, New York, N...' },
  { id: 'r2', name: 'Brooklyn Bites', rating: 4.9, mins: 15, miles: 3.5,
    address: '789 Park Avenue, New York, NY...' },
  { id: 'r3', name: 'Food Fusion Hub', rating: 5.0, mins: 15, miles: 3.5,
    address: '600 Lexington Ave, New York, N...' },
];

// `kind` is which result frame is showing. It only means anything once there is
// a query, which is why the tab strip is not drawn on the empty state.
const state = { q: '', kind: 'dishes', recent: [...RECENT], sort: 'menu' };

let CAT = null;   // the venue's catalogue, when the host names one

// The customer's own recent searches, kept in the browser. Submitting is what
// makes a query recent (see bind), and a private window that refuses storage
// simply starts empty rather than failing the screen.
const RECENT_KEY = 'dowiz.search.recent';
const readRecent = () => {
  try { const v = JSON.parse(localStorage.getItem(RECENT_KEY) || '[]');
        return Array.isArray(v) ? v.filter(t => typeof t === 'string').slice(0, 8) : []; }
  catch { return []; }
};
const writeRecent = list => {
  try { localStorage.setItem(RECENT_KEY, JSON.stringify(list)); } catch { /* fine */ }
};

const dishCard = d => {
  const veg = vegOf(d);
  return `
  <article class="k-vcard${d.available === false ? ' is-sold' : ''}" data-dish="${esc(d.id)}"
           data-go="item-details?id=${encodeURIComponent(d.id)}">
    <div class="k-photo">${photo(d)}${soldPill(d)}</div>
    ${d.off ? `<span class="k-off k-pill-off">${esc(d.off)}</span>` : ''}
    <button class="k-heart" type="button" aria-pressed="false"
            aria-label="У обране: ${esc(d.name)}">${icon('heart-outline')}</button>
    <div class="k-vcard-body">
      <div class="k-mcard-top">
        <span class="k-mcard-name">${esc(d.name)}</span>
        ${d.rating ? `<span class="k-rate">${icon('star2')}${d.rating}</span>` : ''}
      </div>
      ${d.mins && d.miles ? `
      <span class="k-rrow-line">${icon('clock')}${d.mins} Min
        ${icon('ellipse3293', 'k-dot')}${d.miles} Miles</span>`
      : d.kind ? `<span class="k-meta">${esc(d.kind)}</span>` : ''}
      ${soldNote(d)}
      <div class="k-mcard-foot"><b>${esc(d.price)}</b>
        ${veg == null ? '' : icon(veg ? 'veg-mark' : 'nonveg-mark')}</div>
    </div>
  </article>`;
};

const placeRow = p => `
  <article class="k-rrow" data-place="${esc(p.id)}" data-go="restaurant-menu">
    <div class="k-rrow-img">${plate(p.name)}</div>
    <div class="k-rrow-body">
      <div class="k-rrow-head">
        <span class="k-rrow-name">${esc(p.name)}</span>
        ${p.rating ? `<span class="k-rate">${icon('star2')}${p.rating}</span>` : ''}
      </div>
      <span class="k-rrow-line">${icon('pin-17')}${esc(p.address)}</span>
      ${p.mins ? `
      <span class="k-rrow-line">${icon('clock')}${p.mins} Min
        ${p.miles ? `${icon('ellipse3293', 'k-dot')}${p.miles} Miles` : ''}</span>` : ''}
    </div>
  </article>`;

// The one venue this host serves, in the shape of the frame's restaurant row.
// It has no rating and no distance, so those lines are left out rather than
// filled in (DECISIONS.md: dowiz scores nobody).
const venueRow = () => CAT?.venue ? {
  id: CAT.venue.slug || 'venue', name: CAT.venue.name, rating: null,
  mins: CAT.venue.eta || null, miles: null, address: CAT.venue.address || '',
} : null;

// Accent-blind matching, so "Durres" finds "Durrës" and "kastravec" finds the
// description whichever way the venue spelt it.
const fold = s => String(s || '').toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '');

const hits = () => {
  const n = fold(state.q.trim());
  if (!CAT) return {
    dishes: DISHES.filter(d => !n || fold(d.name).includes(n)),
    places: PLACES.filter(p => !n || fold(p.name).includes(n)),
  };
  const v = venueRow();
  return {
    dishes: sortItems(CAT.items.filter(d =>
      !n || fold(d.name).includes(n) || fold(d.description).includes(n)
         || fold(d.kind).includes(n)), state.sort),
    places: v && (!n || fold(v.name).includes(n)) ? [v] : [],
  };
};

const recentTags = () => `
  <div class="k-rtags">
    ${state.recent.map(t => `
      <span class="k-rtag" data-recent="${esc(t)}" role="button" tabindex="0">${esc(t)}
        <span class="ic-wrap">${icon('chip-remove')}</span></span>`).join('')}
  </div>`;

const emptyState = () => {
  if (!CAT) return `
  <div class="k-block">
    <div class="k-sec"><h2 class="t-title">Recent Search</h2>
      <button class="k-seeall" type="button" id="clearRecent">Clear All</button></div>
    ${recentTags()}
  </div>

  <div class="k-block">
    <div class="k-sec"><h2 class="t-title">Recently Searched Items</h2>
      <button class="k-seeall" type="button" data-go="popular-dishes">See All</button></div>
    <div class="k-rail">${DISHES.slice(0, 2).map(dishCard).join('')}</div>
  </div>

  <div class="k-block">
    <div class="k-sec"><h2 class="t-title">Recently Viewed</h2>
      <button class="k-seeall" type="button" data-go="popular-restaurants">See All</button></div>
    ${PLACES.map(placeRow).join('')}
  </div>`;

  // With a venue: the customer's recents if there are any, otherwise the
  // venue's categories in the same tag shape -- named as categories, because a
  // heading that says "Recent Search" over words nobody searched is the frame's
  // placeholder problem in a new coat. Then the first of the venue's dishes,
  // under the name Home gives the same rail.
  const cats = [...CAT.categories].sort((a, b) => a.sortOrder - b.sortOrder);
  return `
  <div class="k-block">
    ${state.recent.length ? `
    <div class="k-sec"><h2 class="t-title">Recent Search</h2>
      <button class="k-seeall" type="button" id="clearRecent">Clear All</button></div>
    ${recentTags()}` : `
    <div class="k-sec"><h2 class="t-title">Categories</h2>
      <button class="k-seeall" type="button" data-go="category">See All</button></div>
    <div class="k-rtags">
      ${cats.map(c => `
        <button class="k-rtag" type="button"
                data-go="category?cat=${encodeURIComponent(c.id)}">${esc(c.name)}</button>`).join('')}
    </div>`}
  </div>

  <div class="k-block">
    <div class="k-sec"><h2 class="t-title">Popular Dishes</h2>
      <button class="k-seeall" type="button" data-go="popular-dishes">See All</button></div>
    <div class="k-rail">${CAT.items.slice(0, 6).map(dishCard).join('')}</div>
  </div>`;
};

// The block under the tab strip: the sort chips and the matches of the chosen
// kind. Drawn on its own so a tab switch can replace this and nothing else --
// rebuilding the strip along with it detached the very tab that was tapped,
// and the interaction gate read its state off the dead node (`STATE STUCK
// button aria-selected=false`, once per tab).
const resultBlock = ({ dishes, places }) => {
  const rows = state.kind === 'dishes' ? dishes : places;
  return `
  ${CAT && state.kind === 'dishes' ? sortChips(state.sort) : ''}
  <div class="k-block">
    ${rows.length
      ? (state.kind === 'dishes'
          ? `<div class="k-grid">${rows.map(dishCard).join('')}</div>`
          : rows.map(placeRow).join(''))
      : `<div class="k-empty">${icon('search')}
           <p class="t-title">Нічого не знайшли</p>
           <p class="t-body muted2">Спробуйте іншу назву.</p></div>`}
  </div>`;
};

const results = () => {
  const found = hits();
  return `
  <div class="k-tabs k-tabs-wide k-block" role="tablist">
    <button type="button" role="tab" data-kind="dishes"
            aria-selected="${state.kind === 'dishes'}">Dishes (${found.dishes.length})</button>
    <button type="button" role="tab" data-kind="places"
            aria-selected="${state.kind === 'places'}">Restaurants (${found.places.length})</button>
  </div>
  <div id="results">${resultBlock(found)}</div>`;
};

const panel = () => (state.q.trim() ? results() : emptyState());

export async function render(params){
  CAT = catalogue(await menu('uk'));
  if (CAT && !CAT.items.length) CAT = null;   // a hub with no dishes keeps the frame
  state.q = params?.get('q') || '';
  state.sort = sortId(params?.get('sort'));
  state.recent = CAT ? readRecent() : [...RECENT];
  return `
  <div class="k-search-top">
    <button class="k-top-btn" type="button" data-back aria-label="Назад">
      ${icon('arrow-left')}
    </button>
    <label class="k-field">
      ${icon('search')}
      <input id="q" type="search" placeholder="Search..." aria-label="Пошук"
             enterkeyhint="search" value="${esc(state.q)}">
    </label>
  </div>

  <div class="wrap" id="panel">${panel()}</div>
  ${navbar('')}`;
}

export function bind(root){
  guardPhotos(root);
  const repaint = () => {
    const host = root.querySelector('#panel');
    host.innerHTML = panel();
    paintPlates(host);
  };
  const remember = list => { state.recent = list; if (CAT) writeRecent(list); };

  root.addEventListener('input', e => {
    if (e.target.id !== 'q') return;
    state.q = e.target.value;
    repaint();
  });

  // Submitting is what makes a query "recent": every keystroke would fill the
  // list with the prefixes of one word.
  root.addEventListener('keydown', e => {
    if (e.target.id !== 'q' || e.key !== 'Enter') return;
    const q = state.q.trim();
    if (!q) return;
    remember([q, ...state.recent.filter(t => t.toLowerCase() !== q.toLowerCase())].slice(0, 8));
  });

  root.addEventListener('click', e => {
    const kind = e.target.closest('[data-kind]');
    if (kind){
      state.kind = kind.dataset.kind;
      for (const t of root.querySelectorAll('[data-kind]'))
        t.setAttribute('aria-selected', String(t.dataset.kind === state.kind));
      const block = root.querySelector('#results');
      if (block){ block.innerHTML = resultBlock(hits()); paintPlates(block); }
      else repaint();
      return;
    }

    const sort = e.target.closest('[data-sort]');
    if (sort){
      state.sort = sortId(sort.dataset.sort);
      paintSortChips(root, state.sort);
      // Only the grid changes order; the tabs and chips above it stay put.
      const grid = root.querySelector('.k-grid');
      if (grid){ grid.innerHTML = hits().dishes.map(dishCard).join(''); paintPlates(grid); }
      return;
    }

    if (e.target.closest('#clearRecent')){ remember([]); repaint(); return; }

    const tag = e.target.closest('[data-recent]');
    if (tag){
      // The × removes the term; the term itself searches for it.
      if (e.target.closest('.ic-wrap')){
        remember(state.recent.filter(t => t !== tag.dataset.recent));
      } else {
        state.q = tag.dataset.recent;
        root.querySelector('#q').value = state.q;
      }
      repaint();
    }
  });
}
