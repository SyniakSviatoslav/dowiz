// Item Details — Figma node 1:10420 (dark) / 1:11649 (light).
//
// The screen a dish is chosen on: gallery, price, the size and topping pickers,
// and a sticky bar whose total is recomputed from the chosen options. The total
// here is FOR DISPLAY: dowiz's kernel recomputes every sum before an order is
// accepted, and a client total is never trusted (see workers/api/src, money.rs).
//
// Two templates share one bar and one handler. The FRAME's template is the
// designer's ItaliaCrisp Pizza with its four sizes and five toppings, and it is
// what renders when the route names no dish or the menu is unreachable -- a
// screen that renders nothing when the API is down is the bug this kit started
// with. The DISH template is the venue's record: photograph, price, what is in
// it, what it weighs, what it does to the day, and the allergens -- each drawn
// exactly as the hub declared it, because on a menu a made-up figure is worse
// than a blank.

import { icon, esc, toast } from '/kit/app.js';
import { plate, heroButtons, qtyBar, ctaBar, topBar } from '/kit/parts.js';
import { menu, venue, dishById, formatMoney } from '/kit/data.js';
import { add as addToBasket } from '/kit/basket.js';
import { FACTS, factText, allergenName, isMarked, mark, unmark, count as compareCount,
         LIMIT as COMPARE_LIMIT, fallbackPlates } from '/kit/screens/compare.js';

const ITEM = {
  name: 'ItaliaCrisp Pizza',
  off: '20% OFF',
  rating: 4.8,
  ratingCount: '1.2K',
  venue: { name: 'Brooklyn Bites', address: '789 Park Avenue, New York, N...', free: true },
  cookingTime: '10 min',
  cuisine: 'Italian',
  // Prices are integer cents. The kit prints dollars; dowiz's own money
  // formatter does the same job for a venue's real currency.
  sizes: [
    { id: 's',  label: '8’ - Small',       cents: 1200 },
    { id: 'm',  label: '10’ - Regular',    cents: 1400 },
    { id: 'l',  label: '12’ - Large',      cents: 1600 },
    { id: 'xl', label: '14’ - Extra Larger', cents: 1800 },
  ],
  toppings: [
    { id: 'cheese',   name: 'Cheese',   cents: 100, on: true  },
    { id: 'olives',   name: 'Olives',   cents: 100, on: false },
    { id: 'onion',    name: 'Onion',    cents: 100, on: false },
    { id: 'capsicum', name: 'Capsi...', cents: 100, on: true  },
    { id: 'mushroom', name: 'Mashr...', cents: 100, on: false },
  ],
  description: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod ' +
               'tempor incididunt ut labore et dolore magna aliqua ',
  ingredients: [
    '1 cup Lorem Ipsum',
    '200g dolor sit amet',
    '2 teaspoons consectetur adipiscing',
    '1 tablespoon ac fermentum',
    '2 teaspoons consectetur adipiscing',
    '1/4 cup metus bibendum fermentum',
  ],
};

// State lives in the module, not the DOM, so the total is computed from the
// chosen options rather than read back out of the markup that displays it.
const state = { size: 's', toppings: new Set(['cheese', 'capsicum']), qty: 1 };

// What the bar and the basket read: the frame's own item, or the venue's dish
// reduced to the same shape. `dish` is the venue's full record and is null
// while the frame is showing.
let item = ITEM;
let dish = null;

// A real dish has one price, not four sizes: dowiz models size as a modifier
// group, and a venue that defines none has nothing to choose. Rather than
// inventing three sizes the venue does not sell, the picker disappears and the
// single price stands -- which is also why `total()` reads from `item`.
function adopt(record, body){
  dish = record;
  item = {
    id: record.id,
    name: record.name,
    kind: record.kind || '',
    venue: { name: venue(body)?.name || '', address: venue(body)?.address || '', free: false },
    sizes: [{ id: 's', label: record.sizeCm ? `${record.sizeCm} см` : '', cents: record.priceMinor }],
    toppings: [],
    currency: record.currency,
  };
  state.size = 's';
  state.toppings = new Set();
}

// The frame prints dollars; a venue is charged in its own currency, and the
// server's integer minor units are the only amount anybody trusts.
const money = cents => item.currency
  ? formatMoney(cents, item.currency)
  : '$' + (cents / 100).toFixed(2);

function total(){
  const size = item.sizes.find(s => s.id === state.size) || item.sizes[0];
  const tops = [...state.toppings]
    .map(id => item.toppings.find(t => t.id === id))
    .filter(Boolean)
    .reduce((n, t) => n + t.cents, 0);
  return (size.cents + tops) * state.qty;
}

const chosenSizeLabel = () =>
  (item.sizes.find(s => s.id === state.size) || item.sizes[0]).label;

const chosenToppingLabel = () => {
  const names = item.toppings.filter(t => state.toppings.has(t.id)).map(t => t.name);
  return names.length ? names.join(', ') : 'немає';
};

// ── The venue's dish ───────────────────────────────────────────────────────

// The facts the detail page lays out as a grid. Price is on its own line under
// the name and the two lists have their own sections, so they are not repeated
// here; the comparison table prints all ten.
const GRID_FACTS = FACTS.filter(f => !f.list && f.key !== 'price');

// The compare control says what it did: pressed or not, and how many are in
// the comparison now. It is DISABLED, with the reason in its text, when the
// comparison is full and this dish is not in it -- a live button that cannot
// act is the defect the interaction gate reports as dead.
const compareState = () => {
  const on = isMarked(dish.id);
  const n = compareCount();
  const full = !on && n >= COMPARE_LIMIT;
  return { on, n, full,
    face: `${icon(on ? 'clipboard-tick' : 'clipboard-text')}<span>${
      full ? `Порівняння повне (${COMPARE_LIMIT})` : on ? 'У порівнянні' : 'Порівняти'}</span>` };
};

function compareButtons(){
  const { on, n, full, face } = compareState();
  return `
  <div class="k-cmp-row">
    <button class="k-cmp-btn" type="button" id="cmpToggle" aria-pressed="${on}"
            ${full ? 'disabled' : ''}>${face}</button>
    <button class="k-cmp-go" type="button" data-go="compare" ${n ? '' : 'hidden'}>
      <span>Порівняння · <b id="cmpCount">${n}</b></span>${icon('arrow-right')}
    </button>
  </div>`;
}

// Repainted IN PLACE, not re-rendered: the tapped button stays the same node.
// The interaction gate reads the state of the element it tapped, and a fresh
// node under the same id leaves it holding the old one -- which never changes,
// and a working toggle is reported as stuck.
function paintCompare(root){
  const { on, n, full, face } = compareState();
  const btn = root.querySelector('#cmpToggle');
  if (!btn) return;
  btn.setAttribute('aria-pressed', String(on));
  btn.disabled = full;
  btn.innerHTML = face;
  const go = root.querySelector('.k-cmp-go');
  go.hidden = n === 0;
  go.querySelector('#cmpCount').textContent = String(n);
}

// Declared facts as a grid; the undeclared ones named once, in a sentence.
// Seven cells of «не вказано» read as a broken screen, and today's venue has
// declared almost none of these, so the common case is the sentence.
function factsBlock(){
  const rows = GRID_FACTS.map(f => ({ f, text: factText(f, dish) }));
  const have = rows.filter(r => r.text != null);
  const miss = rows.filter(r => r.text == null).map(r => r.f.label.toLowerCase());
  if (dish.ingredients == null) miss.push('склад');
  return `
  <h2 class="k-pick-h">Про страву</h2>
  ${have.length ? `
  <dl class="k-facts">
    ${have.map(({ f, text }) => `
    <div class="k-fact"><dt>${esc(f.label)}</dt><dd>${esc(text)}</dd></div>`).join('')}
  </dl>` : ''}
  ${miss.length ? `
  <p class="k-facts-none">Заклад не вказав: ${esc(miss.join(', '))}.</p>` : ''}`;
}

// Allergens are the loudest fact on the page. `null` is not "none": it is a
// venue that has not said, and a customer with an allergy needs to hear that
// difference in words, not infer it from a missing row.
function allergensBlock(){
  const a = dish.allergens;
  if (a == null) return `
  <h2 class="k-pick-h">Алергени</h2>
  <p class="k-facts-none">Заклад не вказав алергенів — уточніть перед замовленням.</p>`;
  if (!Array.isArray(a) || !a.length) return `
  <h2 class="k-pick-h">Алергени</h2>
  <p class="k-desc is-open">Заклад заявляє: без алергенів зі списку.</p>`;
  return `
  <h2 class="k-pick-h">Алергени</h2>
  <div class="k-tags k-allergens">
    ${a.map(code => `<span class="k-tag">${esc(allergenName(code))}</span>`).join('')}
  </div>`;
}

function ingredientsBlock(){
  const list = dish.ingredients;
  if (list == null) return '';                      // named in the facts sentence
  if (!Array.isArray(list) || !list.length) return `
  <h2 class="k-ing">${icon('salad')}Склад</h2>
  <p class="k-desc is-open">Заклад вказав порожній склад.</p>`;
  return `
  <h2 class="k-ing">${icon('salad')}Склад</h2>
  <ul class="k-ing-list">
    ${list.map(i => `<li>${icon('bullet-dot')}${esc(String(i))}</li>`).join('')}
  </ul>`;
}

// The frame clamps the description at three lines behind "Read more". A
// one-line description with a "Read more" that reveals nothing is a control
// that does nothing, so the button is drawn only when there is something to
// reveal. 140 characters is about three lines at this width.
const descriptionBlock = text => !text ? '' : `
  <h2 class="k-pick-h">Опис</h2>
  <p class="k-desc${text.length > 140 ? '' : ' is-open'}">${esc(text)}${text.length > 140
    ? `<button class="k-more" type="button" id="readMore">Читати далі</button>` : ''}</p>`;

function dishTemplate(){
  const d = dish;
  return `
  <div class="k-hero">
    <div class="k-hero-img">
      ${d.imageUrl ? `<img class="k-hero-photo" src="${esc(d.imageUrl)}" alt="${esc(d.name)}"
                          data-plate="${esc(d.name)}">`
                   : plate(d.name)}
    </div>
    ${heroButtons()}
  </div>

  <div class="wrap">
    ${d.kind ? `<p class="k-kind">${esc(d.kind)}</p>` : ''}
    <h1 class="t-title" id="itemName">${esc(d.name)}</h1>
    <p class="k-dprice"><b>${esc(d.price)}</b>${d.sizeCm != null ? `<span>${esc(d.sizeCm)} см</span>` : ''}</p>
    ${d.available ? '' : `
    <p class="k-out">Немає в наявності${d.unavailableNote ? ` — ${esc(d.unavailableNote)}` : ''}</p>`}

    ${compareButtons()}

    <div class="k-rule"></div>

    <div class="k-venue">
      <div class="k-venue-img">${plate(item.venue.name)}</div>
      <div class="k-venue-body">
        <span class="k-venue-name">${esc(item.venue.name)}</span>
        <span class="k-venue-addr">${icon('pin-17')}${esc(item.venue.address)}</span>
      </div>
    </div>

    <div class="k-rule"></div>

    ${factsBlock()}
    ${allergensBlock()}
    ${ingredientsBlock()}
    ${descriptionBlock(d.description)}
  </div>

  ${d.available
    ? qtyBar({ qty: state.qty, label: 'Додати', total: money(total()), minusOff: state.qty <= 1 })
    : ctaBar('Немає в наявності', { disabled: true })}`;
}

// ── The frame ──────────────────────────────────────────────────────────────

function frameTemplate(){
  const ITEM_ = item;
  return `
  <div class="k-hero">
    <div class="k-hero-img">${plate(ITEM_.name)}</div>
    ${heroButtons()}
    <div class="k-gal" role="group" aria-label="Галерея">
      ${[0,1,2,3,4].map(i => `
        <button class="k-gal-th" type="button" data-go="gallery" aria-label="Фото ${i + 1}">
          ${plate(ITEM_.name + i)}
        </button>`).join('')}
      <span class="k-gal-more">${icon('gallery-more')}</span>
    </div>
  </div>

  <div class="wrap">
    <div class="k-mcard-top">
      <span class="k-pill-off">${icon('discount-mark')}${esc(ITEM_.off)}</span>
      <span class="k-rate-18">${icon('star-18')}<b>${ITEM_.rating}</b> (${esc(ITEM_.ratingCount)})</span>
    </div>
    <h1 class="t-title" id="itemName">${esc(ITEM_.name)}</h1>

    <div class="k-rule"></div>

    <div class="k-venue">
      <div class="k-venue-img">${plate(ITEM_.venue.name)}</div>
      <div class="k-venue-body">
        <span class="k-pill-free">Free Delivery</span>
        <span class="k-venue-name">${esc(ITEM_.venue.name)}</span>
        <span class="k-venue-addr">${icon('pin-17')}${esc(ITEM_.venue.address)}</span>
      </div>
    </div>

    <div class="k-rule"></div>

    <div class="k-stats">
      <div class="k-stat">
        <span class="k-stat-ring">${icon('clock-20')}</span>
        <span><span class="k-stat-k">Cooking Time</span>
              <span class="k-stat-v">${esc(ITEM_.cookingTime)}</span></span>
      </div>
      <span class="k-stat-rule"></span>
      <div class="k-stat">
        <span class="k-stat-ring">${icon('reserve-20')}</span>
        <span><span class="k-stat-k">Cuisine</span>
              <span class="k-stat-v">${esc(ITEM_.cuisine)}</span></span>
      </div>
    </div>

    <div class="k-rule"></div>

    <h2 class="k-pick-h">Select Size : <span id="sizeLabel">${esc(chosenSizeLabel())}</span></h2>
    <div class="k-sizes" role="group" aria-label="Розмір">
      ${ITEM_.sizes.map(s => `
        <button class="k-size" type="button" data-size="${esc(s.id)}"
                aria-pressed="${s.id === state.size}">
          <i>${esc(s.label)}</i><b>${money(s.cents)}</b>
        </button>`).join('')}
    </div>

    <h2 class="k-pick-h">Choose Toppings : <span id="topLabel">${esc(chosenToppingLabel())}</span></h2>
    <div class="k-tops" role="group" aria-label="Топінги">
      ${ITEM_.toppings.map(t => `
        <button class="k-topping" type="button" data-top="${esc(t.id)}"
                aria-pressed="${state.toppings.has(t.id)}">
          <span class="k-topping-ring"></span>
          <span class="k-topping-name">${esc(t.name)}</span>
        </button>`).join('')}
    </div>

    <h2 class="k-pick-h">Description</h2>
    <p class="k-desc">${esc(ITEM_.description)}<button class="k-more" type="button"
       id="readMore">Read more</button></p>

    <h2 class="k-ing">${icon('salad')}Ingredients</h2>
    <ul class="k-ing-list">
      ${ITEM_.ingredients.map(i => `<li>${icon('bullet-dot')}${esc(i)}</li>`).join('')}
    </ul>
  </div>

  ${qtyBar({ qty: state.qty, label: 'Add item', total: money(total()),
             minusOff: state.qty <= 1 })}`;
}

// The route named a dish and the menu could not say which. Showing the frame's
// pizza under a tapped "Sake Futomaki" is the wrong dish with a straight face,
// so the screen says what it knows and offers the way back to the menu.
const missing = (title, text) => `
  ${topBar('Страва')}
  <div class="wrap k-cmp-how">
    <h2 class="k-pick-h">${esc(title)}</h2>
    <p class="k-desc is-open">${esc(text)}</p>
  </div>
  ${ctaBar('До меню', { to: 'restaurant-menu' })}`;

export async function render(params){
  const id = params?.get('id');
  const body = await menu('uk');
  dish = null;
  item = ITEM;
  if (!id) return frameTemplate();
  if (!body) return missing('Меню зараз недоступне',
    'Дані про страву приходять від закладу, і вони не завантажилися. Перевірте зв’язок і спробуйте ще раз.');
  const record = dishById(body, id);
  if (!record) return missing('Такої страви немає в меню',
    'Заклад міг прибрати її або змінити посилання.');
  adopt(record, body);
  return dishTemplate();
}

export function bind(root){
  fallbackPlates(root);
  root.addEventListener('click', e => {
    const size = e.target.closest('[data-size]');
    if (size){
      state.size = size.dataset.size;
      for (const b of root.querySelectorAll('[data-size]'))
        b.setAttribute('aria-pressed', String(b === size));
      const lbl = root.querySelector('#sizeLabel');
      if (lbl) lbl.textContent = chosenSizeLabel();
      return repaintTotal(root);
    }

    const top = e.target.closest('[data-top]');
    if (top){
      const id = top.dataset.top;
      state.toppings.has(id) ? state.toppings.delete(id) : state.toppings.add(id);
      top.setAttribute('aria-pressed', String(state.toppings.has(id)));
      const tl = root.querySelector('#topLabel');
      if (tl) tl.textContent = chosenToppingLabel();
      return repaintTotal(root);
    }

    const step = e.target.closest('[data-step]');
    if (step){
      // One is the floor, not zero: removing the item is what the back button
      // and the cart's own remove control are for.
      state.qty = Math.max(1, Math.min(99, state.qty + Number(step.dataset.step)));
      root.querySelector('#qty').value = state.qty;
      // A control that cannot act must not look like one that can. At one, the
      // minus does nothing — so it says so, instead of being tapped and ignored.
      const minus = root.querySelector('[data-step="-1"]');
      if (minus) minus.disabled = state.qty <= 1;
      const plus = root.querySelector('[data-step="1"]');
      if (plus) plus.disabled = state.qty >= 99;
      return repaintTotal(root);
    }

    // THE BUTTON THIS SCREEN EXISTS FOR.
    // It carried `data-cta` and nothing listened for it, so the most important
    // control in a food app did nothing at all.
    if (e.target.closest('[data-cta]')){
      const size = item.sizes.find(x => x.id === state.size) || item.sizes[0];
      const tops = item.toppings.filter(t => state.toppings.has(t.id));
      addToBasket({
        // The LINE's id distinguishes two sizes of the same dish; the ORDER
        // names the dish. Sending the composite as `product_id` is what made
        // the hub answer `unknown product` at the last possible moment.
        id: `${item.id || 'item'}:${size.id}`,
        productId: item.id || undefined,
        name: item.name,
        kind: item.kind || '',
        variant: size.label,
        // Integer cents, one portion. The basket multiplies by quantity; it does
        // not store a pre-multiplied number that a later edit would contradict.
        cents: size.cents + tops.reduce((n, t) => n + t.cents, 0),
        qty: state.qty,
        addons: tops.map(t => t.name),
      });
      toast(`${item.name} — у кошику`);
      return;
    }

    // Mark for comparison. The button flips, the count beside it moves, and the
    // toast says so: three things a person can see, so a tap is never silent.
    if (e.target.closest('#cmpToggle') && dish){
      if (isMarked(dish.id)) unmark(dish.id);
      else if (!mark(dish.id)){
        toast(`У порівнянні вже ${COMPARE_LIMIT} — приберіть одну`);
        return;
      }
      paintCompare(root);
      const n = compareCount();
      toast(isMarked(dish.id)
        ? `${dish.name} — у порівнянні (${n})`
        : `${dish.name} — прибрано з порівняння (${n})`);
      return;
    }

    if (e.target.closest('#readMore')){
      root.querySelector('.k-desc').classList.toggle('is-open');
    }
    // `data-go`, `data-back`, the heart and the share button are the shell's:
    // app.js listens for them on the document, once, for every screen.
  });
}

function repaintTotal(root){
  root.querySelector('#ctaTotal').textContent = money(total());
}
