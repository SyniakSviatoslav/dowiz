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
// The card is the Wolt shape: a full-width photograph, four by three, corners
// rounded, the price on the photo in the money face, the name and one line of
// what is in it beneath, the add control in reach of a thumb.

import { state, findProduct, normalise, hiddenBecause, ALLERGENS, allergenName, saveAvoid, moneyEl } from '/store/state.js';
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

let openDishFn = null;
export function onOpenDish(fn){ openDishFn = fn; }
let onAdd = null;
export function onQuickAdd(fn){ onAdd = fn; }

function card(p, catId){
  const out = !p.available;
  const tags = Array.isArray(p.tags) ? p.tags : [];
  const facts = [];
  if (Number.isFinite(p.cookingMin)) facts.push(`${icon('clock')}${p.cookingMin} <span data-t="etaMin"></span>`);
  if (Number.isFinite(p.weightG)) facts.push(`${p.weightG} g`);
  const kcal = p.calories ?? p.nutrition?.kcal;
  if (Number.isFinite(kcal)) facts.push(`${kcal} <span data-t="kcal"></span>`);
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
      ${ALLERGENS.length ? `<button type="button" class="tag ${state.avoid?.length ? 'on warn' : ''}" id="avoidGo"
          aria-expanded="${state.avoidOpen ? 'true' : 'false'}" aria-controls="avoidBox">
        ${icon('alert-circle')}<span><span data-t="avoid"></span><span id="avoidN">${state.avoid?.length ? ` · ${state.avoid.length}` : ''}</span></span></button>` : ''}
      <label class="tag chk"><input type="checkbox" id="availOnly" ${state.availOnly ? 'checked' : ''}><span data-t="onlyAvail"></span></label>
      <label class="tag sel">${icon('filter')}<select id="sort" data-t-attr="aria-label:sortBy">
        <option value="pop" ${state.sort === 'pop' ? 'selected' : ''} data-t="sortPop"></option>
        <option value="low" ${state.sort === 'low' ? 'selected' : ''} data-t="sortLow"></option>
        <option value="high" ${state.sort === 'high' ? 'selected' : ''} data-t="sortHigh"></option>
        <option value="az" ${state.sort === 'az' ? 'selected' : ''} data-t="sortAz"></option>
      </select></label>
    </div>
    <div class="avoid" id="avoidBox" ${state.avoidOpen ? '' : 'hidden'}>
      <p class="avoid-h" data-t="avoidHint"></p>
      <div class="avoid-in">${ALLERGENS.map(([code]) => `<button type="button" class="chip ${state.avoid?.includes(code) ? 'on' : ''}"
         data-avoid="${code}" aria-pressed="${state.avoid?.includes(code) ? 'true' : 'false'}">${esc(allergenName(code))}</button>`).join('')}</div>
      <p class="avoid-h" id="avoidCount"></p>
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
      if (ok && state.availOnly && el.dataset.avail !== '1') ok = false;
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
    if (state.sort !== 'pop') {
      const key = state.sort === 'low' ? (a, b) => a.dataset.price - b.dataset.price
        : state.sort === 'high' ? (a, b) => b.dataset.price - a.dataset.price
        : (a, b) => findProduct(a.dataset.p).name.localeCompare(findProduct(b.dataset.p).name, lang);
      const wrap = sec.querySelector('.cards');
      for (const el of [...cards].sort(key)) wrap.appendChild(el);
    } else {
      const wrap = sec.querySelector('.cards');
      for (const el of [...cards].sort((a, b) => a.dataset.sort - b.dataset.sort)) wrap.appendChild(el);
    }
    sec.hidden = n === 0;
    const rc = $(`.rail-chip[data-c="${CSS.escape(sec.dataset.cat)}"]`);
    if (rc) rc.hidden = n === 0;
    const sn = sec.querySelector('.sec-n'); if (sn) sn.textContent = n;
    shown += n;
  }
  $('#noHits').hidden = shown > 0;
  const count = $('#avoidCount');
  if (count) {
    const avoidOn = state.avoid?.length;
    count.hidden = !avoidOn;
    if (avoidOn) count.innerHTML = `${hidden.contains} ${esc(t('avoidOn'))}${hidden.undeclared ? `, ${hidden.undeclared} ${esc(t('avoidUnknown'))}` : ''}
      · <button type="button" class="linky" id="avoidClear">${esc(t('clearAvoid'))}</button>`;
    const clr = $('#avoidClear'); if (clr) clr.onclick = () => { state.avoid = []; saveAvoid(); syncAvoidChips(); applyFilters(); };
  }
  const n = $('#avoidN'); if (n) n.textContent = state.avoid?.length ? ` · ${state.avoid.length}` : '';
  $('#avoidGo')?.classList.toggle('on', !!state.avoid?.length);
  $('#avoidGo')?.classList.toggle('warn', !!state.avoid?.length);
  if (animate) spread($$('.card:not([hidden])').slice(0, 24));
}

function syncAvoidChips(){
  for (const b of $$('[data-avoid]')) {
    const onn = state.avoid.includes(b.dataset.avoid);
    b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn));
  }
}

// ── scroll-spy: the rail follows the reader ─────────────────────────────────
let spyIO = null;
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
let spyPaused = false;

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
      spread($$('.card:not([hidden])', sec).slice(0, 16));
      setTimeout(() => { spyPaused = false; }, 700);
      return;
    }
    const tag = e.target.closest('[data-tag]');
    if (tag) {
      state.tag = tag.dataset.tag || null;
      for (const b of $$('[data-tag]')) { const onn = (b.dataset.tag || null) === state.tag; b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn)); }
      applyFilters({ animate: true });
      return;
    }
    const av = e.target.closest('[data-avoid]');
    if (av) {
      const code = av.dataset.avoid;
      state.avoid = state.avoid.includes(code) ? state.avoid.filter(c => c !== code) : [...state.avoid, code];
      saveAvoid(); syncAvoidChips(); applyFilters();
      return;
    }
    if (e.target.closest('#avoidGo')) {
      state.avoidOpen = !state.avoidOpen;
      $('#avoidBox').hidden = !state.avoidOpen;
      $('#avoidGo').setAttribute('aria-expanded', String(state.avoidOpen));
      return;
    }
    if (e.target.closest('#qx')) { state.q = ''; $('#q').value = ''; $('#qx').hidden = true; applyFilters(); $('#q').focus(); return; }
    if (e.target.closest('#qreset')) {
      state.q = ''; state.tag = null; state.availOnly = false; state.sort = 'pop';
      $('#q').value = ''; $('#qx').hidden = true; $('#availOnly').checked = false; $('#sort').value = 'pop';
      for (const b of $$('[data-tag]')) { const onn = !b.dataset.tag; b.classList.toggle('on', onn); b.setAttribute('aria-pressed', String(onn)); }
      applyFilters({ animate: true });
      return;
    }
  });
  // Search runs on every keystroke because it is local; debounced only to
  // coalesce a fast typist's frames, never to wait for a server.
  const q = $('#q');
  const run = debounce(() => { state.q = q.value; $('#qx').hidden = !state.q; applyFilters(); }, 60);
  q.addEventListener('input', run);
  $('#sort').onchange = e => { state.sort = e.target.value; applyFilters({ animate: true }); };
  $('#availOnly').onchange = e => { state.availOnly = e.target.checked; applyFilters(); };
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

export const focusSearch = () => { $('#find')?.scrollIntoView({ block: 'start', behavior: 'smooth' }); setTimeout(() => $('#q')?.focus(), 350); };
export const scrollTop = () => window.scrollTo({ top: 0, behavior: 'smooth' });
