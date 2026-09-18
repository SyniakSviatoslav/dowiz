// Home — Figma node 1:1123 (dark set) / 1:11340 (light set).
//
// The content is the kit's own, verbatim from the frame, because this is a port
// of a design: the venue's real dishes arrive through the same shapes once the
// screen is wired to /api/public/locations/<slug>/menu, and every card below is
// built from a record rather than written out, so that swap is a data change.

import { icon, esc, go } from '/kit/app.js';
import { plate, navbar } from '/kit/parts.js';
import { menu, catalogue } from '/kit/data.js';
import { photo, soldNote, soldPill, vegOf, guardPhotos } from '/kit/screens/listing.js';

// ── The frame's content ────────────────────────────────────────────────────
const CATEGORIES = [
  { id: 'burger',  name: 'Burger'  },
  { id: 'pizza',   name: 'Pizza'   },
  { id: 'noodles', name: 'Noodles' },
  { id: 'drinks',  name: 'Drinks'  },
];

const DISHES = [
  { id: 'd1', name: 'ItaliaCrisp Pizza', off: '20% OFF', rating: 4.9,
    mins: 15, miles: 3.5, price: '$12.00', veg: true },
  { id: 'd2', name: 'Veggie Pasta', off: '10% OFF', rating: 4.8,
    mins: 10, miles: 2.5, price: '$16.00', veg: true },
  { id: 'd3', name: 'Smoky Ribs', off: '15% OFF', rating: 4.7,
    mins: 20, miles: 4.0, price: '$18.00', veg: false },
];

const RESTAURANTS = [
  { id: 'r1', name: 'Brooklyn Bites', off: '25% OFF', rating: 4.9,
    tags: 'Italian, American, Mexican, Japanese...',
    address: '789 Park Avenue, New York, NY 10021, USA',
    mins: 15, miles: 3.5, delivery: 'Free Delivery' },
  { id: 'r2', name: 'The Savory Spot', off: '10% OFF', rating: 4.8,
    tags: 'Italian, American, Chinese, Japanese...',
    address: '88 Bedford Street, New York, NY 10014',
    mins: 10, miles: 2.5, delivery: 'Free Delivery' },
];

const REST_FILTERS = [
  { id: 'filter',  name: 'Filter',   icon: 'filter1', on: false },
  { id: 'sort',    name: 'Sort',     icon: 'vuesax-linear-arrow-down', on: false },
  { id: 'nearest', name: 'Nearest',  on: true  },
  { id: 'popular', name: 'Popular',  on: true  },
  { id: 'offers',  name: 'Great Offers', on: false },
];

// ── Pieces ─────────────────────────────────────────────────────────────────

const section = (title, target) => `
  <div class="k-sec">
    <h2 class="t-section">${esc(title)}</h2>
    <button class="k-seeall" type="button" data-go="${esc(target)}">See All</button>
  </div>`;

const dishCard = d => `
  <article class="k-dish${d.available === false ? ' is-sold' : ''}" data-dish="${esc(d.id)}"
           data-go="item-details?id=${encodeURIComponent(d.id)}">
    <div class="k-photo">${photo(d)}${soldPill(d)}</div>
    ${d.off ? `<span class="k-off">${esc(d.off)}</span>` : ''}
    <button class="k-heart" type="button" aria-pressed="false"
            aria-label="У обране: ${esc(d.name)}">${icon('heart')}</button>
    <div class="k-dish-body">
      <div class="k-dish-top">
        <span class="k-dish-name">${esc(d.name)}</span>
        ${d.rating ? `<span class="k-rate">${icon('star2')}${d.rating}</span>` : ''}
      </div>
      ${meta(d)}
      ${soldNote(d)}
      <div class="k-price"><b>${esc(d.price)}</b>${d.veg ? veg() : ''}</div>
    </div>
  </article>`;

// The frame's dish card carries a time and a distance. A real venue quotes one
// delivery estimate for the whole hub and measures no per-dish distance, so the
// line shows what is actually known rather than a number nobody measured.
const meta = d => {
  const parts = [];
  if (d.mins) parts.push(`${esc(String(d.mins))} Min`);
  if (d.miles) parts.push(`${d.miles} Miles`);
  if (!parts.length) return d.kind ? `<div class="k-meta">${esc(d.kind)}</div>` : '';
  return `<div class="k-meta">${icon('clock')}${parts.join(' · ')}</div>`;
};

// The little green square that marks a vegetarian dish in the frame.
const veg = () => `<span class="k-veg" aria-label="вегетаріанська"></span>`;

// The frame's category chip is a disc and a word, and the first one is lit. A
// venue's chip is the same disc with the plate keyed to the category's name in
// it, and it GOES somewhere: the Category screen, opened on that category. The
// frame's chips only light up, which the gate accepts as a toggle but a
// customer reports as "I tapped Sushi and nothing happened".
const catChip = (c, i) => `
  <button class="k-cat${i === 0 ? ' on' : ''}" type="button" data-cat="${esc(c.id)}"
          aria-pressed="${i === 0}">
    <span class="k-cat-img"></span>${esc(c.name)}
  </button>`;

const liveCatChip = c => `
  <button class="k-cat" type="button" data-go="category?cat=${encodeURIComponent(c.id)}">
    <span class="k-cat-img is-plate">${plate(c.name)}</span>${esc(c.name)}
  </button>`;

const restCard = r => `
  <article class="k-rest" data-rest="${esc(r.id)}" data-go="restaurant-menu">
    <div class="k-photo">${plate(r.name)}</div>
    ${r.off ? `<span class="k-off">${esc(r.off)}</span>` : ''}
    <button class="k-heart" type="button" aria-pressed="false"
            aria-label="У обране: ${esc(r.name)}">${icon('heart')}</button>
    <div class="k-rest-body">
      <div class="k-dish-top">
        <span class="t-section">${esc(r.name)}</span>
        <span class="k-rate">${icon('star2')}${r.rating}</span>
      </div>
      <p class="k-rest-tags">${esc(r.tags)}</p>
      <p class="k-rest-line">${icon('location')}${esc(r.address)}</p>
      <p class="k-rest-line">${icon('clock')}${r.mins} Min · ${r.miles} Miles · ${esc(r.delivery)}</p>
    </div>
  </article>`;

// ── The screen ─────────────────────────────────────────────────────────────

export async function render(){
  // The frame's content is the fallback, not a placeholder to be replaced at
  // all costs: on a host that names no venue -- or when the request fails --
  // the screen still renders the design it is a port of.
  const cat = catalogue(await menu('uk'));
  const place = cat?.venue;
  // The rail shows the first eight of the venue's own order, sold-out ones
  // included -- there is no popularity count to rank by, and the hub's first
  // category is the one the venue chose to put first.
  const live = cat?.items.slice(0, 8).map(d => ({ ...d, mins: place?.eta, veg: vegOf(d) === true }));
  const cards = live?.length ? live : DISHES;
  const cats = cat?.categories.length
    ? [...cat.categories].sort((a, b) => a.sortOrder - b.sortOrder) : null;
  const where = place?.address || 'New York, USA';
  const title = place?.name || 'Location';

  return `
  <header class="k-head">
    <img class="k-avatar" alt="" src="/kit/icon/rectangle14.svg" width="46" height="46">
    <div class="k-loc">
      <p class="t-label muted2">${esc(title)}</p>
      <div class="k-loc-row">
        ${icon('vuesax-linear-location')}
        <span class="k-loc-name">${esc(where)}</span>
        ${icon('arrow-left', 'k-loc-chev')}
      </div>
    </div>
    <button class="k-bell" type="button" data-go="notification" aria-label="Сповіщення">
      <span class="k-bell-face">${icon('notification-bing')}</span>
      <span class="k-bell-dot"></span>
    </button>
  </header>

  <div class="wrap">
    <div class="k-search">
      <label class="k-field">
        ${icon('search')}
        <input id="q" type="search" placeholder="Search Dish, Restaurant..."
               aria-label="Пошук страви або ресторану" enterkeyhint="search">
      </label>
      <button class="k-filter" type="button" data-go="filter" aria-label="Фільтри">
        ${icon('filter')}
      </button>
    </div>

    ${section('Exclusive Offers', 'exclusive-offers')}
    <button class="k-offer" type="button" data-go="exclusive-offers">
      <span class="k-offer-art"></span>
      <span class="k-offer-tag">Weekend Offers!</span>
      <span class="k-offer-h">Get Special Offers</span>
      <span class="k-offer-row">
        <span class="k-offer-upto">Up to</span>
        <span class="k-offer-num">30<span class="k-offer-pct">%</span></span>
      </span>
      <span class="k-offer-cta">Get Now</span>
    </button>
    <div class="k-dots" aria-hidden="true">
      <span class="k-dot on"></span><span class="k-dot"></span>
      <span class="k-dot"></span><span class="k-dot"></span>
    </div>

    ${section('Explore Categories', 'category')}
    <div class="k-rail" role="group" aria-label="Категорії">
      ${cats ? cats.map(liveCatChip).join('') : CATEGORIES.map(catChip).join('')}
    </div>

    ${section('Popular Dishes', 'popular-dishes')}
    <div class="k-rail" id="dishes">${cards.map(dishCard).join('')}</div>

    ${section('Popular Restaurants', 'popular-restaurants')}
    <div class="k-chips" role="group" aria-label="Фільтри ресторанів">
      ${REST_FILTERS.map(f => `
        <button class="k-chip${f.on ? ' on' : ''}" type="button" data-chip="${esc(f.id)}"
                aria-pressed="${!!f.on}">
          ${f.icon ? icon(f.icon) : ''}${esc(f.name)}
        </button>`).join('')}
    </div>
    <div id="rests">${RESTAURANTS.map(restCard).join('')}</div>
  </div>

  ${navbar('home')}
  <button class="k-fab" type="button" data-go="chat" aria-label="Чат із підтримкою">
    ${icon('linear-messages-conversation-chat-round-dots')}
  </button>`;
}

export function bind(root){
  guardPhotos(root);
  // One delegated listener rather than a handler per card: this screen renders
  // 3 dishes and 2 restaurants in the frame and an arbitrary number with real
  // data, and a listener per card is a leak waiting for the next render.
  root.addEventListener('click', e => {
    const heart = e.target.closest('.k-heart');
    if (heart){
      e.stopPropagation();
      heart.setAttribute('aria-pressed', heart.getAttribute('aria-pressed') !== 'true');
      return;
    }
    const chip = e.target.closest('[data-chip]');
    if (chip){ toggle(chip); return; }

    const cat = e.target.closest('[data-cat]');
    if (cat){ pickOne(root, '[data-cat]', cat); return; }
  });

  // Search filters what is already on the screen. There is no request to make:
  // the list is in the document, and a five-item filter is microseconds.
  const q = root.querySelector('#q');
  q?.addEventListener('input', () => {
    const needle = q.value.trim().toLowerCase();
    for (const card of root.querySelectorAll('[data-dish],[data-rest]')){
      const hit = !needle || card.textContent.toLowerCase().includes(needle);
      card.hidden = !hit;
    }
  });
  // Enter hands the words to the Search screen, which reads descriptions as
  // well as names: the rail above only has eight cards to narrow, and a
  // customer who typed "avokado" wants every roll with avocado in it.
  q?.addEventListener('keydown', e => {
    if (e.key !== 'Enter' || !q.value.trim()) return;
    go(`search?q=${encodeURIComponent(q.value.trim())}`);
  });
}

function toggle(el){
  const on = el.getAttribute('aria-pressed') !== 'true';
  el.setAttribute('aria-pressed', String(on));
  el.classList.toggle('on', on);
}

function pickOne(root, selector, chosen){
  for (const el of root.querySelectorAll(selector)){
    const on = el === chosen;
    el.setAttribute('aria-pressed', String(on));
    el.classList.toggle('on', on);
  }
}
