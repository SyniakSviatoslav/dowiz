// Act 2 -- CHOOSE. The menu, built once and then only ever mutated.
//
// THE LAG WAS THE RE-RENDER. Every keystroke, every category tap, every sort
// change and every language switch rebuilt 165 cards from strings, which on a
// phone is a visible stall. The cards are built ONCE here, with the facts a
// filter needs written onto them as data attributes, and from then on a filter
// toggles `hidden`, a sort reorders nodes, a category tap scrolls, and a
// language switch rewrites text by product id. Nothing is rebuilt until the
// venue is reloaded.
//
// The card: a full-width photograph, four by three, the price on the photo in
// the money face, the name in the venue's display face beneath, one line of
// what is in it, the approximate calories, and the add control in reach of a
// thumb. A dish with no photograph wears the venue's own mark.
//
// THE SEARCH BAR IS A SEARCH BAR. The allergen filter and the "available
// only" checkbox sat in it and pushed the tags off the screen; a customer
// looking for salmon saw a warning triangle and a checkbox. Allergens moved
// to the language sheet, where diet belongs beside language; sold-out dishes
// are already marked on the card and do not need a second control.

import { state, findProduct, normalise, hiddenBecause, saveAvoid, moneyEl } from '/store/state.js';
import { t, tagName, lang } from '/store/i18n.js';
import { $, $$, esc, icon, fallbackArt, paintFallbacks, spread, debounce } from '/store/ui.js';
import { heroMarkup } from '/store/venue.js';

// The venue's tags → an icon each. A tag with no icon still gets a chip, with
// a dot; the venue wrote it down and it filters just the same.
const TAG_ICON = {
  salmon: 'fish', tuna: 'fish', shrimp: 'fish', vegetarian: 'leaf', hot: 'flame', popular: 'sparkles',
  sets: 'category', bowls: 'bowl', soups: 'soup', drinks: 'cup', freskuese: 'cup', kafeteria: 'cup',
  'lengje-frutash': 'cup', alkool: 'glass-cocktail', birra: 'beer',
};
// Tags that mirror a category heading are noise in the rail: the customer has
// the category rail for those.
const TAG_SKIP = new Set(['sets', 'bowls', 'soups', 'drinks', 'freskuese', 'kafeteria', 'lengje-frutash', 'alkool', 'birra']);
/// The sort orders the customer can choose; `pop` is the venue's own order.
const SORTS = ['pop', 'low', 'high', 'az'];
/// How many cards spread in on a filter change or a category tap: the first
/// screenful and a little more. The rest simply appear.
const SPREAD_ON_FILTER = 24;
const SPREAD_ON_TAP = 16;
/// The rail stops following the scroll for this long after a tap, so the chip
/// the customer chose stays lit while the page glides to it.
const SPY_PAUSE_MS = 700;
/// Typing is local, so the search runs on every keystroke; this only coalesces
/// a fast typist's frames, never waits for a server.
const SEARCH_DEBOUNCE_MS = 60;
/// A focused search field a moment after the scroll to it has started, so the
/// keyboard rises where the field will be.
const FOCUS_AFTER_SCROLL_MS = 350;
/// The prefix on every figure the venue has not measured. A customer asked for
/// "approximately how many calories" and gets the word "approximately" in the
/// number itself, not a footnote.
const APPROX = '≈';

let openDishFn = null;
export function onOpenDish(fn){ openDishFn = fn; }
let onAdd = null;
export function onQuickAdd(fn){ onAdd = fn; }

/// The calorie figure for a card, or null. Approximate figures carry the mark.
export function kcalText(p){
  const n = p.nutrition || {};
  const kcal = p.calories ?? n.kcal;
  if (!Number.isFinite(kcal)) return null;
  return `${n.approx ? APPROX + ' ' : ''}${kcal}`;
}

function card(p, catId){
  const out = !p.available;
  const tags = Array.isArray(p.tags) ? p.tags : [];
  const facts = [];
  const kcal = kcalText(p);
  if (kcal) facts.push(`${icon('flame')}${kcal} <span data-t="kcal"></span>`);
  if (Number.isFinite(p.weightG)) facts.push(`${p.nutrition?.approx ? APPROX + ' ' : ''}${p.weightG} g`);
  if (Number.isFinite(p.cookingMin)) facts.push(`${icon('clock')}${p.cookingMin} <span data-t="etaMin"></span>`);
  return `<article class="card ${out ? 'sold-out' : ''}" data-p="${esc(p.id)}" data-cat="${esc(catId)}"
      data-price="${p.price | 0}" data-avail="${out ? 0 : 1}" data-tags="${esc(tags.join(' '))}"
      data-search="${esc(normalise(`${p.name} ${p.description || ''}`))}" data-sort="${p.sortOrder | 0}">
    <button type="button" class="card-hit" data-open="${esc(p.id)}" ${out ? 'aria-disabled="true"' : ''}
            aria-label="${esc(p.name)}">
      <span class="card-media">${p.imageUrl
        ? `<img src="${esc(p.imageUrl)}" alt="" loading="lazy" decoding="async" data-fb="${esc(p.name)}">`
        : fallbackArt(p.name)}
        <span class="card-price">${moneyEl(p.price)}</span>
        ${out ? `<span class="card-out" data-t="soldOut"></span>` : ''}
        ${tags.includes('popular') ? `<span class="card-flag">${icon('sparkles')}<span data-t-tag="popular"></span></span>` : ''}
      </span>
      <span class="card-body">
        <span class="card-name">${esc(p.name)}</span>
        ${p.description ? `<span class="card-desc">${esc(p.description)}</span>` : ''}
        ${facts.length ? `<span class="card-facts">${facts.map(f => `<span>${f}</span>`).join('')}</span>` : ''}
      </span>
    </button>
    ${out ? '' : `<button type="button" class="card-add" data-add="${esc(p.id)}" aria-label="${esc(t('add'))}: ${esc(p.name)}">${icon('plus')}</button>`}
  </article>`;
}

function railMarkup(cats){
  return `<nav class="rail" id="rail" aria-label="${esc(t('menu'))}"><div class="rail-in">
    ${cats.map((c, i) => `<button type="button" class="rail-chip" data-c="${esc(c.id)}" ${i === 0 ? 'aria-current="true"' : ''}>${esc(c.name)}</button>`).join('')}
  </div></nav>`;
}

function filtersMarkup(cats){
  const seen = new Map();
  for (const c of cats) for (const p of (c.products || [])) for (const tg of (p.tags || [])) {
    if (TAG_SKIP.has(tg)) continue;
    seen.set(tg, (seen.get(tg) || 0) + 1);
  }
  const tags = [...seen.entries()].sort((a, b) => b[1] - a[1]).map(([k]) => k);
  return `<div class="find" id="find">
    <label class="srch">
      ${icon('search')}
      <input id="q" type="search" inputmode="search" autocomplete="off"
             data-t-attr="placeholder:search aria-label:search" value="${esc(state.q || '')}">
      <button id="qx" type="button" data-t-attr="aria-label:clear" ${state.q ? '' : 'hidden'}>${icon('x')}</button>
    </label>
    <div class="tags" id="tags" role="group" aria-label="${esc(t('filters'))}">
      <button type="button" class="tag ${!state.tag ? 'on' : ''}" data-tag="" aria-pressed="${!state.tag}">
        ${icon('category')}<span data-t="all"></span></button>
      ${tags.map(tg => `<button type="button" class="tag ${state.tag === tg ? 'on' : ''}" data-tag="${esc(tg)}" aria-pressed="${state.tag === tg}">
        ${TAG_ICON[tg] ? icon(TAG_ICON[tg]) : '<span class="tag-dot"></span>'}<span data-t-tag="${esc(tg)}"></span></button>`).join('')}
      <label class="tag sel">${icon('filter')}<select id="sort" data-t-attr="aria-label:sortBy">
        ${SORTS.map(s => `<option value="${s}" ${state.sort === s ? 'selected' : ''} data-t="sort${s[0].toUpperCase()}${s.slice(1)}"></option>`).join('')}
      </select></label>
    </div>
  </div>`;
}

export function buildMenu(cats){
  const app = $('#app');
  app.innerHTML = `${heroMarkup()}${filtersMarkup(cats)}${railMarkup(cats)}
    <div id="sections">${cats.map(c => `
      <section class="sec" id="c-${esc(c.id)}" data-cat="${esc(c.id)}">
        <h2 class="sec-h"><span class="sec-name">${esc(c.name)}</span><span class="sec-n muted">${(c.products || []).length}</span></h2>
        <div class="cards">${(c.products || []).map(p => card(p, c.id)).join('')}</div>
      </section>`).join('')}
    </div>
    <div class="empty" id="noHits" hidden>${icon('search-off', 'ico-lg')}<b data-t="noHits"></b>
      <button class="btn btn-ghost mt-3 w-cap" id="qreset" data-t="clear"></button></div>
    <div class="tail-gap"></div>`;
  paintFallbacks(app);
  bind();
  applyFilters({ animate: true });
  spy();
}

// ── filtering: toggles, never rebuilds ──────────────────────────────────────
export function applyFilters({ animate = false } = {}){
  const q = normalise(state.q).trim();
  const terms = q ? q.split(/\s+/) : [];
  let shown = 0;
  const hidden = { contains: 0, undeclared: 0 };
  for (const sec of $$('.sec')) {
    const cards = $$('.card', sec);
    let n = 0;
    for (const el of cards) {
      const p = findProduct(el.dataset.p);
      let ok = !!p;
      if (ok && state.tag && !(` ${el.dataset.tags} `).includes(` ${state.tag} `)) ok = false;
      if (ok) { const why = hiddenBecause(p); if (why) { hidden[why]++; ok = false; } }
      if (ok && terms.length) {
        const hay = `${el.dataset.search} ${normalise(sec.querySelector('.sec-name')?.textContent)}`;
        ok = terms.every(x => hay.includes(x));
      }
      el.hidden = !ok;
      if (ok) n++;
    }
    // Sort by reordering nodes. 'pop' is the venue's own order; a restaurant
    // arranges its menu deliberately and we should not overrule it.
    const key = state.sort === 'low' ? (a, b) => a.dataset.price - b.dataset.price
      : state.sort === 'high' ? (a, b) => b.dataset.price - a.dataset.price
      : state.sort === 'az' ? (a, b) => findProduct(a.dataset.p).name.localeCompare(findProduct(b.dataset.p).name, lang)
      : (a, b) => a.dataset.sort - b.dataset.sort;
    const wrap = sec.querySelector('.cards');
    for (const el of [...cards].sort(key)) wrap.appendChild(el);
    sec.hidden = n === 0;
    const rc = $(`.rail-chip[data-c="${CSS.escape(sec.dataset.cat)}"]`);
    if (rc) rc.hidden = n === 0;
    const sn = sec.querySelector('.sec-n'); if (sn) sn.textContent = n;
    shown += n;
  }
  $('#noHits').hidden = shown > 0;
  // The count of what the allergen filter hid, wherever that panel is open.
  const count = $('#avoidCount');
  if (count) {
    const avoidOn = state.avoid?.length;
    count.hidden = !avoidOn;
    if (avoidOn) count.innerHTML = `${hidden.contains} ${esc(t('avoidOn'))}${hidden.undeclared ? `, ${hidden.undeclared} ${esc(t('avoidUnknown'))}` : ''}
      · <button type="button" class="linky" id="avoidClear">${esc(t('clearAvoid'))}</button>`;
    const clr = $('#avoidClear'); if (clr) clr.onclick = () => {
      state.avoid = []; saveAvoid();
      for (const b of $$('[data-avoid]')) { b.classList.remove('on'); b.setAttribute('aria-pressed', 'false'); }
      applyFilters();
    };
  }
  // The header says when a diet filter is on, since the control is one sheet away.
  $('#langBtn')?.classList.toggle('warn', !!state.avoid?.length);
  if (animate) spread($$('.card:not([hidden])').slice(0, SPREAD_ON_FILTER));
}

// ── scroll-spy: the rail follows the reader ─────────────────────────────────
let spyIO = null;
let spyPaused = false;
function spy(){
  if (spyIO) spyIO.disconnect();
  const chips = $$('.rail-chip');
  const light = id => {
    for (const c of chips) c.setAttribute('aria-current', c.dataset.c === id ? 'true' : 'false');
    const on = chips.find(c => c.dataset.c === id);
    on?.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'smooth' });
  };
  const visible = new Map();
  spyIO = new IntersectionObserver(entries => {
    for (const e of entries) visible.set(e.target.dataset.cat, e.isIntersecting ? e.boundingClientRect.top : Infinity);
    let best = null, top = Infinity;
    for (const [id, y] of visible) if (y < top) { top = y; best = id; }
    if (best && !spyPaused) light(best);
  }, { rootMargin: '-140px 0px -60% 0px', threshold: [0, 0.1] });
  for (const s of $$('.sec')) spyIO.observe(s);
}

function bind(){
  const app = $('#app');
  // One delegated listener for the whole menu: the cards are many and change.
  app.addEventListener('click', e => {
    const open = e.target.closest('[data-open]');
    if (open) { const p = findProduct(open.dataset.open); if (p && p.available) openDishFn?.(p, open); return; }
    const add = e.target.closest('[data-add]');
    if (add) { const p = findProduct(add.dataset.add); if (p && p.available) onAdd?.(p, add); return; }
    const chip = e.target.closest('.rail-chip');
    if (chip) {
      const sec = $(`#c-${CSS.escape(chip.dataset.c)}`);
      if (!sec) return;
      spyPaused = true;
      for (const c of $$('.rail-chip')) c.setAttribute('aria-current', c === chip ? 'true' : 'false');
      sec.scrollIntoView({ block: 'start', behavior: 'smooth' });
      // The section's cards SPREAD from the tap: the category change is a
      // re-diffusion, not a page swap.
      spread($$('.card:not([hidden])', sec).slice(0, SPREAD_ON_TAP));
      setTimeout(() => { spyPaused = false; }, SPY_PAUSE_MS);
      return;
    }
    const tag = e.target.closest('[data-tag]');
    if (tag) {
      state.tag = tag.dataset.tag || null;
      for (const b of $$('[data-tag]')) { const onn = (b.dataset.tag || null) === state.tag; b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn)); }
      applyFilters({ animate: true });
      return;
    }
    if (e.target.closest('#qx')) { state.q = ''; $('#q').value = ''; $('#qx').hidden = true; applyFilters(); $('#q').focus(); return; }
    if (e.target.closest('#qreset')) {
      state.q = ''; state.tag = null; state.sort = SORTS[0];
      $('#q').value = ''; $('#qx').hidden = true; $('#sort').value = SORTS[0];
      for (const b of $$('[data-tag]')) { const onn = !b.dataset.tag; b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn)); }
      applyFilters({ animate: true });
      return;
    }
  });
  const q = $('#q');
  const run = debounce(() => { state.q = q.value; $('#qx').hidden = !state.q; applyFilters(); }, SEARCH_DEBOUNCE_MS);
  q.addEventListener('input', run);
  $('#sort').onchange = e => { state.sort = e.target.value; applyFilters({ animate: true }); };
}

/// The venue's own words in a new language, patched by id. Names, descriptions
/// and category headings change; nothing else moves.
export function patchTexts(cats){
  for (const c of cats) {
    const h = $(`#c-${CSS.escape(c.id)} .sec-name`); if (h) h.textContent = c.name;
    const chip = $(`.rail-chip[data-c="${CSS.escape(c.id)}"]`); if (chip) chip.textContent = c.name;
    for (const p of (c.products || [])) {
      const el = $(`.card[data-p="${CSS.escape(p.id)}"]`); if (!el) continue;
      const nm = el.querySelector('.card-name'); if (nm) nm.textContent = p.name;
      let ds = el.querySelector('.card-desc');
      if (p.description) {
        if (!ds) { ds = document.createElement('span'); ds.className = 'card-desc'; nm?.after(ds); }
        ds.textContent = p.description;
      } else if (ds) ds.remove();
      el.dataset.search = normalise(`${p.name} ${p.description || ''}`);
      el.querySelector('.card-hit')?.setAttribute('aria-label', p.name);
    }
  }
  applyFilters();
}

export const focusSearch = () => { $('#find')?.scrollIntoView({ block: 'start', behavior: 'smooth' }); setTimeout(() => $('#q')?.focus(), FOCUS_AFTER_SCROLL_MS); };
export const scrollTop = () => window.scrollTo({ top: 0, behavior: 'smooth' });
