// Filter — Figma node 1:6267 (dark) / 1:16680 (light).
//
// Groups of 35px pills (r178) that wrap, a star row, a price range, and two
// actions at the foot. Read with design/spec.py 1:6267: headings are 18/500,
// pills 16/400 with 16px of side padding, selected pills orange on white.
//
// With a venue behind it the groups are the venue's: its categories in place
// of the frame's cuisines, the catalogue's sort orders in place of "Fastest
// Delivery", and a price range that spans what the venue actually charges, in
// its currency. The star row and the offers group are the frame's only: dowiz
// publishes no rating (DECISIONS.md) and this hub sells no offers, and a
// filter over a number nobody measured narrows nothing.
//
// Apply carries the answer to the Popular Dishes screen in its route
// (`popular-dishes?cat=bowls,maki&sort=price-asc&max=1500`), so the grid, a
// reload of it and the back button all agree about what was asked for.

import { icon, esc, go } from '/kit/app.js';
import { topBar } from '/kit/parts.js';
import { menu, catalogue, formatMoney } from '/kit/data.js';
import { SORTS } from '/kit/screens/listing.js';

// Options are `{ id, name }` in both sets, so a pill's identity is not its
// caption: the venue's category "Sete" and a caption typed twice are two ids.
const opts = names => names.map(n => ({ id: n, name: n }));

const FRAME_GROUPS = [
  { id: 'cuisine', head: 'Cuisine', multi: true,
    options: opts(['All', 'American', 'Italian', 'Mexican', 'Japanese', 'Chinese']), chosen: ['All'] },
  { id: 'sort', head: 'Sort By', multi: false,
    options: opts(['Recommended', 'Fastest Delivery', 'Top Rated', 'Spa & Massage Services']),
    chosen: ['Recommended'] },
  { id: 'offer', head: 'Offers', multi: true,
    options: opts(['Free Delivery', 'Great Offers', 'Buy 1 Get 1']), chosen: [] },
];

const STARS = [5, 4, 3, 2, 1];

// The frame's range is $5–$100 in $5 steps. A venue's is its cheapest to its
// dearest dish in its own minor units; the step is coarse enough that the
// thumb moves in visible amounts on a 327px track.
const FRAME_PRICE = { min: 500, max: 10000, step: 500, start: 5000, currency: null };

let GROUPS = FRAME_GROUPS;
let PRICE = FRAME_PRICE;
let CAT = null;   // the venue's catalogue, when the host names one

const state = {
  picked: new Map(GROUPS.map(g => [g.id, new Set(g.chosen)])),
  stars: 4,
  maxPrice: PRICE.start,
};

const money = minor => PRICE.currency
  ? formatMoney(minor, PRICE.currency)
  : `$${Math.round(minor / 100)}`;

function adopt(cat){
  CAT = cat;
  if (!cat){ GROUPS = FRAME_GROUPS; PRICE = FRAME_PRICE; return; }
  const cats = [...cat.categories].sort((a, b) => a.sortOrder - b.sortOrder);
  GROUPS = [
    { id: 'cat', head: 'Categories', multi: true,
      options: [{ id: 'all', name: 'All' }, ...cats.map(c => ({ id: c.id, name: c.name }))],
      chosen: ['all'] },
    { id: 'sort', head: 'Sort By', multi: false,
      options: SORTS.map(s => ({ id: s.id, name: s.long })), chosen: ['menu'] },
  ];
  const prices = cat.items.map(d => Number(d.priceMinor) || 0);
  const lo = Math.min(...prices), hi = Math.max(...prices);
  // One step is a twentieth of the span, rounded to a round number of the
  // currency's minor units, and never zero: a range whose min equals its max
  // (a one-dish venue) still needs a step the browser accepts.
  const step = Math.max(50, Math.round((hi - lo) / 20 / 50) * 50) || 50;
  PRICE = { min: Math.floor(lo / step) * step, max: Math.ceil(hi / step) * step || step,
            step, start: Math.ceil(hi / step) * step || step, currency: cat.currency };
}

const group = g => `
  <div class="k-fgroup">
    <h2>${esc(g.head)}</h2>
    <div class="k-pills" role="group" aria-label="${esc(g.head)}">
      ${g.options.map(o => `
        <button class="k-pill" type="button" data-group="${esc(g.id)}" data-opt="${esc(o.id)}"
                aria-pressed="${state.picked.get(g.id).has(o.id)}">${esc(o.name)}</button>`).join('')}
    </div>
  </div>`;

// What the pills currently say, as the route the listing reads.
function query(){
  const q = new URLSearchParams();
  const cats = [...state.picked.get('cat') || []].filter(id => id !== 'all');
  if (cats.length) q.set('cat', cats.join(','));
  const sort = [...state.picked.get('sort') || []][0];
  if (sort && sort !== 'menu') q.set('sort', sort);
  if (state.maxPrice < PRICE.max) q.set('max', String(state.maxPrice));
  return q.toString();
}

export async function render(params){
  const cat = catalogue(await menu('uk'));
  adopt(cat && cat.items.length ? cat : null);
  state.picked = new Map(GROUPS.map(g => [g.id, new Set(g.chosen)]));
  state.stars = 4;
  state.maxPrice = PRICE.start;
  // Opened from a listing that was already narrowed, the pills start where
  // that listing is, so Apply does not silently widen it back out.
  if (CAT){
    const ids = new Set(GROUPS[0].options.map(o => o.id));
    const want = (params?.get('cat') || '').split(',').filter(id => ids.has(id));
    if (want.length) state.picked.set('cat', new Set(want));
    if (SORTS.some(s => s.id === params?.get('sort'))) state.picked.set('sort', new Set([params.get('sort')]));
    const max = Number(params?.get('max'));
    if (Number.isFinite(max) && max >= PRICE.min && max <= PRICE.max) state.maxPrice = max;
  }

  return `
  ${topBar('Filter')}
  <div class="wrap">
    ${GROUPS.map(group).join('')}

    ${CAT ? '' : `
    <div class="k-fgroup">
      <h2>Reviews</h2>
      <div class="k-stars" role="radiogroup" aria-label="Мінімальна оцінка">
        ${STARS.map(n => `
          <button class="k-star" type="button" role="radio" data-stars="${n}"
                  aria-checked="${n === state.stars}">${icon('star2')}${n}${
            n === 5 ? '' : '+'}</button>`).join('')}
      </div>
    </div>`}

    <div class="k-fgroup">
      <h2>Price</h2>
      <div class="k-range">
        <input id="price" type="range" min="${PRICE.min}" max="${PRICE.max}" step="${PRICE.step}"
               value="${state.maxPrice}" aria-label="Максимальна ціна">
        <output id="priceOut" for="price">до ${esc(money(state.maxPrice))}</output>
      </div>
    </div>
  </div>

  <div class="k-bar k-bar-two">
    <button class="k-cta k-cta-quiet" type="button" id="reset">Reset</button>
    ${CAT
      ? `<button class="k-cta" type="button" id="apply">Apply</button>`
      : `<button class="k-cta" type="button" data-back>Apply</button>`}
  </div>`;
}

const paintPills = root => {
  for (const b of root.querySelectorAll('[data-group]'))
    b.setAttribute('aria-pressed', String(state.picked.get(b.dataset.group).has(b.dataset.opt)));
};

export function bind(root){
  root.addEventListener('click', e => {
    const pill = e.target.closest('[data-group]');
    if (pill){
      const g = GROUPS.find(x => x.id === pill.dataset.group);
      const set = state.picked.get(g.id);
      const id = pill.dataset.opt;
      if (g.multi){
        // "All" is the absence of a narrowing, so it and a category cannot both
        // be lit: choosing a category puts "All" out, choosing "All" clears the
        // categories, and un-choosing the last category brings "All" back.
        const all = g.options[0].id === 'all' || g.options[0].id === 'All' ? g.options[0].id : null;
        if (id === all){ set.clear(); set.add(all); }
        else {
          set.has(id) ? set.delete(id) : set.add(id);
          if (all){ set.delete(all); if (!set.size) set.add(all); }
        }
      } else {
        set.clear(); set.add(id);
      }
      paintPills(root);
      return;
    }

    const star = e.target.closest('[data-stars]');
    if (star){
      state.stars = Number(star.dataset.stars);
      for (const b of root.querySelectorAll('[data-stars]'))
        b.setAttribute('aria-checked', String(Number(b.dataset.stars) === state.stars));
      return;
    }

    if (e.target.closest('#apply')){
      const q = query();
      go(`popular-dishes${q ? '?' + q : ''}`);
      return;
    }

    if (e.target.closest('#reset')){
      for (const g of GROUPS) state.picked.set(g.id, new Set(g.chosen));
      state.stars = 4;
      state.maxPrice = PRICE.start;
      // A reset that leaves the controls showing the old answer is not a reset.
      paintPills(root);
      for (const b of root.querySelectorAll('[data-stars]'))
        b.setAttribute('aria-checked', String(Number(b.dataset.stars) === state.stars));
      const p = root.querySelector('#price');
      p.value = state.maxPrice;
      root.querySelector('#priceOut').textContent = `до ${money(state.maxPrice)}`;
    }
  });

  root.addEventListener('input', e => {
    if (e.target.id !== 'price') return;
    state.maxPrice = Number(e.target.value);
    root.querySelector('#priceOut').textContent = `до ${money(state.maxPrice)}`;
  });
}
