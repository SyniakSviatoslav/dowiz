// Compare — two or more dishes side by side, one row per fact.
//
// The kit has no such frame. It exists because a customer choosing between two
// bowls asks the same six questions of both -- what is in it, how much, what
// it does to the day, what it costs -- and answering them by flipping between
// two detail screens is how the wrong one gets ordered.
//
// This module is ALSO the store the detail screen writes to, so the two screens
// cannot disagree about what "marked for comparison" means: one key, one
// reader, one writer. The detail screen imports `mark`/`unmark`/`isMarked`
// from here rather than keeping its own copy of the key.
//
// ABSENT IS NOT ZERO. Every fact is drawn from the record exactly as the hub
// sent it: `null` is "the venue has not declared this" and prints as
// «не вказано»; `0` and `[]` are claims the venue made and print as the claim.
// A comparison that filled a blank with a placeholder figure would be steering
// a customer with a number nobody measured -- worse than the blank.

import { icon, esc, toast } from '/kit/app.js';
import { plate, topBar, paintPlates } from '/kit/parts.js';
import { menu, dishById } from '/kit/data.js';

// ── The store ──────────────────────────────────────────────────────────────
// Ids only. The dishes themselves are re-read from the menu on every render,
// so a price the venue changed overnight is the new price here too, and a dish
// the venue withdrew drops out rather than being compared from a stale copy.
//
// localStorage throws in a private window and can come back empty, so every
// read and write is guarded and an unreadable store behaves as an empty one.
const KEY = 'dowiz.compare.v1';

// Four is the most that reads at a phone's width even with the table
// scrolling; past it the columns are too narrow to hold «не вказано».
export const LIMIT = 4;

const read = () => {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || '[]');
    return Array.isArray(raw) ? raw.filter(x => typeof x === 'string' && x) : [];
  } catch { return []; }
};

const write = ids => {
  try { localStorage.setItem(KEY, JSON.stringify(ids)); } catch { /* private window */ }
  dispatchEvent(new CustomEvent('compare', { detail: { ids } }));
};

/** The ids marked for comparison, oldest first. */
export const marked = () => read();
export const count = () => read().length;
export const isMarked = id => read().includes(String(id));

/** Mark a dish. Returns false when the comparison is already full. */
export function mark(id){
  const ids = read();
  id = String(id);
  if (ids.includes(id)) return true;
  if (ids.length >= LIMIT) return false;
  ids.push(id);
  write(ids);
  return true;
}

export function unmark(id){
  write(read().filter(x => x !== String(id)));
}

export function clearAll(){
  write([]);
}

// ── The facts ──────────────────────────────────────────────────────────────
// One table for both screens, so the detail page and the comparison name the
// same fact the same way. `read` returns the venue's value or `null`, and the
// caller decides how to show a null -- the two screens differ on that (the
// detail omits and summarises, the comparison prints «не вказано» in the cell).
//
// `nutrition` is `{kcal, protein, fat, carbs}`, each optional (owner.rs), and
// data.js already folds a bare `calories` into `dish.calories`, so kcal reads
// from there and the other three from the object.
const nut = (d, k) => (d.nutrition && typeof d.nutrition === 'object') ? d.nutrition[k] ?? null : null;

export const FACTS = [
  { key: 'price',      label: 'Ціна',              read: d => d.price ?? null },
  { key: 'weightG',    label: 'Вага порції',       unit: 'г',    read: d => d.weightG ?? null },
  { key: 'kcal',       label: 'Калорійність',      unit: 'ккал', read: d => d.calories ?? null },
  { key: 'protein',    label: 'Білки',             unit: 'г',    read: d => nut(d, 'protein') },
  { key: 'fat',        label: 'Жири',              unit: 'г',    read: d => nut(d, 'fat') },
  { key: 'carbs',      label: 'Вуглеводи',         unit: 'г',    read: d => nut(d, 'carbs') },
  { key: 'cookingMin', label: 'Час приготування',  unit: 'хв',   read: d => d.cookingMin ?? null },
  { key: 'sizeCm',     label: 'Розмір',            unit: 'см',   read: d => d.sizeCm ?? null },
  { key: 'allergens',  label: 'Алергени',          list: true,   read: d => d.allergens ?? null },
  { key: 'ingredients',label: 'Склад',             list: true,   read: d => d.ingredients ?? null },
];

// The fourteen allergens the EU requires a menu to name, in the customer's
// language. The same list the storefront uses (public/app.js); a code that is
// not in it is printed as the venue wrote it rather than dropped, because a
// dropped allergen is the one that hurts somebody.
const ALLERGENS = {
  gluten: 'Глютен', crustaceans: 'Ракоподібні', eggs: 'Яйця', fish: 'Риба',
  peanuts: 'Арахіс', soy: 'Соя', milk: 'Молоко', nuts: 'Горіхи', celery: 'Селера',
  mustard: 'Гірчиця', sesame: 'Кунжут', sulphites: 'Сульфіти', lupin: 'Люпин',
  molluscs: 'Молюски',
};
export const allergenName = code => ALLERGENS[String(code).toLowerCase()] || String(code);

/**
 * What to print for one fact of one dish, or `null` when the venue has not
 * declared it. A declared value is always a non-empty string; an empty list is
 * a claim («немає») and a zero is «0 г», never a blank.
 *
 * Anything of an unexpected shape -- an object where a number should be -- is
 * treated as not declared rather than printed, because "[object Object]" on a
 * menu is the placeholder the interaction gate exists to catch.
 */
export function factText(fact, dish){
  const v = fact.read(dish);
  if (v == null) return null;
  if (fact.list){
    if (!Array.isArray(v)) return null;
    if (!v.length) return 'немає';
    const names = v.map(x => fact.key === 'allergens' ? allergenName(x) : String(x))
                   .filter(s => s.trim());
    return names.length ? names.join(', ') : 'немає';
  }
  if (typeof v === 'number') return Number.isFinite(v) ? `${v}${fact.unit ? ' ' + fact.unit : ''}` : null;
  if (typeof v === 'string') return v.trim() ? `${v.trim()}${fact.unit ? ' ' + fact.unit : ''}` : null;
  return null;
}

// ── The screen ─────────────────────────────────────────────────────────────

const NOT_DECLARED = 'не вказано';

// The header of one column: the photograph (or the name-keyed plate the whole
// kit uses for a dish without one), the name, and the one control that acts
// on this column. The photo opens the dish; it is a button rather than
// decoration because a customer comparing two dishes wants to get back to one.
const head = d => `
  <th scope="col">
    <div class="k-cmp-head">
      <button class="k-cmp-photo" type="button"
              data-go="item-details?id=${encodeURIComponent(d.id)}"
              aria-label="Відкрити ${esc(d.name)}">
        ${d.imageUrl ? `<img src="${esc(d.imageUrl)}" alt="" loading="lazy" data-plate="${esc(d.name)}">`
                     : plate(d.name)}
      </button>
      <span class="k-cmp-name">${esc(d.name)}</span>
      ${d.available ? '' : `<span class="k-cmp-out">немає в наявності</span>`}
      <button class="k-cmp-rm" type="button" data-unmark="${esc(d.id)}"
              aria-label="Прибрати ${esc(d.name)} з порівняння">
        ${icon('chip-remove')}<span>Прибрати</span>
      </button>
    </div>
  </th>`;

const cell = (fact, d) => {
  const text = factText(fact, d);
  return text == null
    ? `<td class="is-none">${NOT_DECLARED}</td>`
    : `<td>${esc(text)}</td>`;
};

const table = dishes => `
  <div class="k-cmp-scroll">
    <table class="k-cmp">
      <caption class="k-cmp-cap">Страви поруч, рядок на кожен факт</caption>
      <thead>
        <tr><th scope="col" class="k-cmp-corner"><span>Факт</span></th>${dishes.map(head).join('')}</tr>
      </thead>
      <tbody>
        ${FACTS.map(f => `
        <tr><th scope="row">${esc(f.label)}</th>${dishes.map(d => cell(f, d)).join('')}</tr>`).join('')}
      </tbody>
    </table>
  </div>
  <p class="k-page-p k-cmp-foot">«${NOT_DECLARED}» — заклад не оголосив цей факт. Нуль означає нуль:
     цифри тут лише ті, що заявив заклад.</p>`;

// With fewer than two dishes there is nothing to put side by side, and an
// empty grid says "broken" where it should say "here is what to do".
const howTo = dishes => `
  <div class="k-cmp-how">
    <h2 class="k-pick-h">Як порівняти</h2>
    <ol class="k-cmp-steps">
      <li>Відкрийте страву в меню.</li>
      <li>Натисніть «Порівняти» під її назвою.</li>
      <li>Поверніться сюди, коли позначите дві або більше.</li>
    </ol>
  </div>
  ${dishes.length ? `
  <h2 class="k-pick-h">Поки що одна страва</h2>
  <div class="k-box k-cmp-one">
    <div class="k-cmp-head">
      <button class="k-cmp-photo" type="button"
              data-go="item-details?id=${encodeURIComponent(dishes[0].id)}"
              aria-label="Відкрити ${esc(dishes[0].name)}">
        ${dishes[0].imageUrl ? `<img src="${esc(dishes[0].imageUrl)}" alt="" loading="lazy"
                                    data-plate="${esc(dishes[0].name)}">`
                             : plate(dishes[0].name)}
      </button>
      <span class="k-cmp-name">${esc(dishes[0].name)}</span>
      <span class="k-cmp-price">${esc(dishes[0].price)}</span>
      <button class="k-cmp-rm" type="button" data-unmark="${esc(dishes[0].id)}"
              aria-label="Прибрати ${esc(dishes[0].name)} з порівняння">
        ${icon('chip-remove')}<span>Прибрати</span>
      </button>
    </div>
  </div>` : ''}`;

// The menu is unreachable and the comparison is built from the menu. The
// marks are kept -- they are ids, and the dishes will be there when the menu
// is -- and the screen says exactly that instead of showing an empty table.
const offline = ids => `
  <div class="k-cmp-how">
    <h2 class="k-pick-h">Меню зараз недоступне</h2>
    <p class="k-desc is-open">Порівняння будується з даних закладу, а вони не завантажилися.
      ${ids.length ? `Позначено страв: ${ids.length} — позначки збережено.` : ''}</p>
  </div>`;

/** The marked dishes that are still on the menu, in the order they were marked. */
function resolve(body){
  const ids = read();
  const found = ids.map(id => dishById(body, id)).filter(Boolean);
  // A dish the venue withdrew since it was marked is dropped from the store
  // as well as the screen, so the count on the detail page's button stays
  // honest about how many are really being compared.
  if (found.length !== ids.length) write(found.map(d => d.id));
  return found;
}

export async function render(){
  const body = await menu('uk');
  const ids = read();
  const dishes = body ? resolve(body) : [];
  const clear = ids.length ? `
      <button class="k-top-btn" type="button" id="cmpClear" aria-label="Очистити порівняння">
        ${icon('trash')}
      </button>` : '';
  return `
  ${topBar('Порівняння', clear)}
  <div class="wrap k-cmp-wrap">
    ${!body ? offline(ids) : dishes.length < 2 ? howTo(dishes) : table(dishes)}
  </div>
  <div class="k-bar">
    <button class="k-cta" type="button" data-go="restaurant-menu">
      ${dishes.length >= 2 && dishes.length < LIMIT ? 'Додати ще страву' : 'До меню'}
    </button>
  </div>`;
}

/**
 * A photograph the browser cannot decode takes the plate's place. Today's venue
 * has exactly one photo and it is a 4 KB file with junk after its JFIF header:
 * every screen that draws it shows a strip of alt text where the dish should
 * be. `error` does not bubble, so it is caught in the capture phase, and the
 * name-keyed plate the whole kit draws for a dish without a photo stands in.
 * Shared by the detail screen, which marks its hero image the same way.
 */
export function fallbackPlates(root){
  root.addEventListener('error', e => {
    const img = e.target;
    if (!(img instanceof HTMLImageElement) || !img.dataset.plate) return;
    const box = img.parentElement;
    img.remove();
    box.insertAdjacentHTML('beforeend', plate(img.dataset.plate));
    paintPlates(box);
  }, true);
}

export function bind(root, params){
  fallbackPlates(root);
  root.addEventListener('click', async e => {
    const rm = e.target.closest('[data-unmark]');
    if (rm){
      unmark(rm.dataset.unmark);
      return repaint(root, params);
    }
    if (e.target.closest('#cmpClear')){
      clearAll();
      toast('Порівняння очищено');
      return repaint(root, params);
    }
  });
}

// The whole screen is a function of the store, so a change repaints it rather
// than patching the one column and leaving the header's count and the empty
// state to drift from the table.
async function repaint(root, params){
  root.innerHTML = await render(params);
  paintPlates(root);
}
