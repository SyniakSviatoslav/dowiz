// The menu -- what the venue sells, category by category.
//
// A category folds; a dish is a row with its photograph, price and whether it
// is on sale; a tap opens the dish as a sheet where everything the storefront
// shows can be set: price, stop-list with a reason, photograph, ingredients,
// nutrition, cooking time, tags, and the name and description in the other
// two languages. The stop-list is one switch: a dish off sale is off for the
// customer within the menu's thirty-second cache.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, money, moneyEl, busy, switchEl, store, retranslate, confirm } from '/admin/core.js';
import { lang, LANGS } from '/admin/i18n.js';
import { loadVenue, rerender } from '/admin/app.js';
import { openBulk } from '/admin/bulk.js';

/// The tags the storefront knows how to filter and draw.
const TAGS = ['popular', 'salmon', 'tuna', 'shrimp', 'vegetarian', 'hot'];
/// A photograph is shrunk to this on the phone before upload.
const PHOTO_MAX_PX = 1600;
const PHOTO_QUALITY = 0.86;
/// Filters over the whole menu and the sorts inside a category. `cat` narrows to one
/// category; `state` is all | on | off (stop-list); `photo` is all | none.
const SORTS = ['default', 'name', 'priceAsc', 'priceDesc'];
const view = { open: new Set(), q: '', cat: '', state: 'all', photo: 'all', sort: SORTS[0] };
/// The dishes of a category as the filters and the sort leave them.
function visibleOf(c){
  let list = (c.products || []).filter(p => (view.state === 'all' || (view.state === 'on') === !!p.available) && (view.photo === 'all' || !p.imageUrl));
  if (view.sort === 'name') list = [...list].sort((a, b) => String(a.name).localeCompare(String(b.name), lang));
  else if (view.sort === 'priceAsc') list = [...list].sort((a, b) => (a.price ?? 0) - (b.price ?? 0));
  else if (view.sort === 'priceDesc') list = [...list].sort((a, b) => (b.price ?? 0) - (a.price ?? 0));
  return list;
}
const filtering = () => view.cat || view.state !== 'all' || view.photo !== 'all' || view.sort !== SORTS[0];

const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

function dishRow(p){
  const q = norm(view.q).trim();
  if (q && !norm(`${p.name} ${p.description || ''}`).includes(q)) return '';
  return `<button type="button" class="rowc ${p.available ? '' : 'off'}" data-p="${esc(p.id)}">
    ${p.imageUrl ? `<img class="thumb" src="${esc(p.imageUrl)}" alt="" loading="lazy">` : `<span class="thumb"></span>`}
    <span class="t"><b>${esc(p.name)}</b><small>${esc(p.description || '')}</small></span>
    <span class="money">${money(p.price)}</span>
    <span class="pill ${p.available ? 'ok' : 'bad'}" data-t="${p.available ? 'onSale' : 'stopList'}"></span>
  </button>`;
}

export async function render(host){
  const cats = S.categories;
  const q = norm(view.q).trim();
  host.innerHTML = `
    <div class="screen-h"><div><p class="eyebrow" data-t="tabMenu"></p><h1>${esc(S.venue?.name || '')}</h1></div>
      <div class="btn-row compact"><button type="button" class="act" id="mCats">${icon('adjustments')}<span data-t="categories"></span></button><button type="button" class="act" id="mImport">${icon('download')}</button><button type="button" class="act" id="mRecipes" aria-label="${esc(t('importRecipes'))}">${icon('tools-kitchen-2')}</button><button type="button" class="act pri" id="mNew">${icon('plus')}<span data-t="addDish"></span></button></div></div>
    <p class="screen-hint" data-t="menuHint"></p>
    <label class="srch">${icon('search')}<input id="mq" type="search" value="${esc(view.q)}" data-t-attr="placeholder:search"></label>
    <div class="chips filters">
      <button type="button" class="chip ${!view.cat ? 'on' : ''}" data-fc=""><span data-t="allDishes"></span></button>
      ${cats.map(c => `<button type="button" class="chip ${view.cat === c.id ? 'on' : ''}" data-fc="${esc(c.id)}">${esc(c.name)}</button>`).join('')}
    </div>
    <div class="chips filters">
      <button type="button" class="chip ${view.state === 'on' ? 'on' : ''}" data-fs="on"><span data-t="onSale"></span></button>
      <button type="button" class="chip ${view.state === 'off' ? 'on' : ''}" data-fs="off"><span data-t="stopList"></span></button>
      <button type="button" class="chip ${view.photo === 'none' ? 'on' : ''}" data-fp="none"><span data-t="withoutPhoto"></span></button>
      <select class="chip sortsel" id="mSort" aria-label="${esc(t('sort'))}">${SORTS.map(k => `<option value="${k}" ${view.sort === k ? 'selected' : ''}>${esc(t('sort_' + k))}</option>`).join('')}</select>
    </div>
    ${cats.filter(c => !view.cat || c.id === view.cat).map(c => { const open = q || filtering() ? true : view.open.has(c.id); const rows = visibleOf(c).map(dishRow).join(''); if ((q || filtering()) && !rows) return ''; return `
      <section class="group">
        <button type="button" class="cat-h" data-cat="${esc(c.id)}" aria-expanded="${open}">${esc(c.name)}<span class="n">${(c.products || []).length}</span>${icon('chevron-down', 'chev')}</button>
        <div class="rows" ${open ? '' : 'hidden'}>${rows}</div>
      </section>`; }).join('')}
    ${!cats.length ? `<div class="empty">${icon('bowl-chopsticks')}<b data-t="loadFail"></b></div>` : ''}`;
  $('#mNew', host).onclick = openNewDish;
  $('#mCats', host).onclick = openCategories;
  $('#mSort', host).onchange = e => { view.sort = e.target.value; rerender(); };
  host.onclick = e => {
    const fc = e.target.closest('[data-fc]'); if (fc) { view.cat = fc.dataset.fc; return rerender(); }
    const fs = e.target.closest('[data-fs]'); if (fs) { view.state = view.state === fs.dataset.fs ? 'all' : fs.dataset.fs; return rerender(); }
    const fp = e.target.closest('[data-fp]'); if (fp) { view.photo = view.photo === 'none' ? 'all' : 'none'; return rerender(); }
    const h = e.target.closest('[data-cat]'); if (h) { const id = h.dataset.cat; view.open.has(id) ? view.open.delete(id) : view.open.add(id); const rows = h.nextElementSibling; rows.hidden = !view.open.has(id); h.setAttribute('aria-expanded', String(view.open.has(id))); return; }
    const r = e.target.closest('[data-p]'); if (r) openDish(r.dataset.p);
  };
  const mq = $('#mq', host); mq.oninput = () => { view.q = mq.value; rerender().then(() => $('#mq')?.focus()); };
  $('#mImport', host).onclick = openImport;
  $('#mRecipes', host).onclick = () => openBulk('recipes', async () => { await loadVenue(); rerender(); });
}

/// The plate's diameter, for the storefront's "see it on the table" view.
const SIZE_CM_MIN = 3, SIZE_CM_MAX = 120;
// Where a dish is made: the closed set the Worker accepts (`bell_route::Station`).
const STATIONS = ['kitchen', 'bar'];

/// CSV import: a dry run first, with what would change, then apply.
function openImport(){
  sheet(`<p class="eyebrow" data-t="tabMenu"></p><h2 data-t="importMenu"></h2><p class="muted small" data-t="importHint"></p>
    <input type="file" id="imFile" accept=".csv,text/csv">
    ${switchEl('imRetire', false, 'retireMissing', 'retireHint')}
    <div class="btn-row"><button class="btn ghost" id="imDry" disabled>${icon('eye')}<span data-t="dryRun"></span></button><button class="btn" id="imApply" disabled>${icon('check')}<span data-t="applyImport"></span></button></div>
    <div id="imOut"></div>`, { name: 'import' });
  let csv = null;
  $('#imFile').onchange = async e => { const f = e.target.files?.[0]; csv = f ? await f.text() : null; $('#imDry').disabled = $('#imApply').disabled = !csv; };
  const run = async (apply) => {
    const q = apply ? `?apply=true${$('#imRetire').checked ? '&retire=true' : ''}` : '';
    try {
      const r = await busy($(apply ? '#imApply' : '#imDry'), () => api(`/owner/menu/import${q}`, { method: 'POST', body: csv, headers: { 'content-type': 'text/csv' } }));
      $('#imOut').innerHTML = `<div class="stats mt-3"><div class="stat"><small data-t="dishes"></small><b>${r.products ?? 0}</b></div><div class="stat"><small data-t="categories"></small><b>${r.categories ?? 0}</b></div></div>
        ${(r.warnings || []).length ? `<details class="fold"><summary>${(r.warnings || []).length} · <span data-t="warnings"></span></summary>${r.warnings.map(w => `<p class="hint">${esc(w)}</p>`).join('')}</details>` : ''}
        ${(r.notInFile || []).length ? `<details class="fold"><summary>${r.notInFile.length} · <span data-t="notInFile"></span></summary>${r.notInFile.map(w => `<p class="hint">${esc(w)}</p>`).join('')}</details>` : ''}
        ${apply ? `<p class="ok">${esc(t('saved'))}${r.retired ? ` · ${r.retired} ${esc(t('retired'))}` : ''}</p>` : ''}`;
      retranslate($('#imOut'));
      if (apply) { await loadVenue(); rerender(); }
    } catch (err) { toast(String(err.message || err)); }
  };
  $('#imDry').onclick = () => run(false);
  $('#imApply').onclick = () => run(true);
}

/// The number as typed, or null when the field is empty.
const num = v => { const s = String(v ?? '').trim(); if (!s) return null; const n = Number(s.replace(',', '.')); return Number.isFinite(n) ? n : null; };

export function openDish(id){
  const p = S.products.find(x => x.id === id); if (!p) return;
  const n = p.nutrition || {};
  const tags = new Set(p.tags || []);
  const tr = p.translations || {};
  sheet(`
    <p class="eyebrow">${esc(p.categoryName || '')}</p>
    <h2>${esc(p.name)}</h2>
    <div class="rowc" id="photoRow">${p.imageUrl ? `<img class="thumb" src="${esc(p.imageUrl)}" alt="">` : `<span class="thumb"></span>`}
      <span class="t"><b data-t="photo"></b><small data-t="${p.imageUrl ? 'photo' : 'noPhoto'}"></small></span>
      <button type="button" class="act" id="photoPick">${icon('camera-plus')}<span data-t="uploadPhoto"></span></button>
      ${p.imageUrl ? `<button type="button" class="act danger" id="photoClear">${icon('trash')}</button>` : ''}
      <input type="file" id="photoFile" accept="image/*" hidden></div>
    ${switchEl('d-avail', p.available, 'onSale')}
    <div id="offBox" ${p.available ? 'hidden' : ''}><label for="d-note" data-t="unavailableNote"></label><input id="d-note" value="${esc(p.unavailableNote || '')}"></div>
    <div class="grid2">
      <div><label for="d-price" data-t="price"></label><input id="d-price" inputmode="numeric" value="${p.price ?? ''}"></div>
      <div><label for="d-cook" data-t="cookingMin"></label><input id="d-cook" inputmode="numeric" value="${p.cookingMin ?? ''}"></div>
    </div>
    <label for="d-station" data-t="station"></label><select id="d-station">${STATIONS.map(k => `<option value="${k}" data-t="station_${k}" ${(p.station || 'kitchen') === k ? 'selected' : ''}></option>`).join('')}</select><p class="hint" data-t="stationHint"></p>
    <label data-t="tags"></label>
    <div class="chips" id="tagPick">${TAGS.map(tg => `<button type="button" class="chip ${tags.has(tg) ? 'on' : ''}" data-tag="${tg}">${esc(tg)}</button>`).join('')}</div>
    ${recipeMarkup(p)}
    ${tasteMarkup(p)}
    <label for="d-ings" data-t="ingredients"></label><textarea id="d-ings" rows="2">${esc((p.ingredients || []).join(', '))}</textarea>
    <label data-t="nutrition"></label>${p.nutritionDerived ? `<p class="hint" data-t="nutritionFromRecipe"></p>` : ''}
    <div class="grid3">
      <div><label for="d-kcal" data-t="kcal"></label><input id="d-kcal" inputmode="numeric" value="${n.kcal ?? ''}"></div>
      <div><label for="d-prot" data-t="protein"></label><input id="d-prot" inputmode="numeric" value="${n.protein ?? ''}"></div>
      <div><label for="d-fat" data-t="fat"></label><input id="d-fat" inputmode="numeric" value="${n.fat ?? ''}"></div>
    </div>
    <div class="grid2">
      <div><label for="d-carb" data-t="carbs"></label><input id="d-carb" inputmode="numeric" value="${n.carbs ?? ''}"></div>
      <div><label for="d-weight" data-t="weight"></label><input id="d-weight" inputmode="numeric" value="${p.weightG ?? ''}"></div>
    </div>
    <label for="d-size" data-t="sizeCm"></label><input id="d-size" inputmode="numeric" min="${SIZE_CM_MIN}" max="${SIZE_CM_MAX}" value="${p.sizeCm ?? ''}"><p class="hint" data-t="sizeCmHint"></p>
    <label data-t="translations"></label>
    ${LANGS.filter(l => l !== (S.venue?.defaultLocale || 'sq')).map(l => `<div class="grid2">
      <div><label for="d-name-${l}">${l.toUpperCase()} · <span data-t="name"></span></label><input id="d-name-${l}" value="${esc(tr[l]?.name || '')}"></div>
      <div><label for="d-desc-${l}">${l.toUpperCase()} · <span data-t="description"></span></label><input id="d-desc-${l}" value="${esc(tr[l]?.description || '')}"></div>
    </div>`).join('')}
    <div class="btn-row"><button class="btn" id="dSave">${icon('check')}<span data-t="save"></span></button></div>
    <div class="btn-row"><button class="btn danger" id="dDel">${icon('trash')}<span data-t="deleteDish"></span></button></div>`, { name: 'dish' });
  $('#dDel').onclick = async () => {
    const ok = await confirm(t('deleteDish'), `${p.name} · ${t('deleteDishHint')}`, { danger: true });
    if (!ok) return openDish(id);
    try { await post(`/owner/products/${encodeURIComponent(id)}/delete`, withLoc()); toast(t('saved')); closeSheet(); await loadVenue(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
  $('#d-avail').onchange = e => { $('#offBox').hidden = e.target.checked; };
  bindRecipe(p); bindTaste(p);
  for (const b of $$('[data-tag]', $('#sheetIn'))) b.onclick = () => b.classList.toggle('on');
  $('#photoPick').onclick = () => $('#photoFile').click();
  $('#photoFile').onchange = async e => {
    const f = e.target.files?.[0]; if (!f) return;
    try {
      const { shrinkPair } = await import('/lib/shrink.js');
      // The sheet's photograph and the grid's card, both made here: the
      // browser has the pixels and a decoder, the Worker has neither.
      const { full, small } = await shrinkPair(f, { max: PHOTO_MAX_PX, quality: PHOTO_QUALITY })
        .catch(() => ({ full: f, small: null }));
      await busy($('#photoPick'), async () => {
        await api(`/owner/products/${encodeURIComponent(id)}/image`, { method: 'POST', body: full, headers: { 'content-type': 'application/octet-stream' } });
        if (small) await api(`/owner/products/${encodeURIComponent(id)}/image?variant=small`, { method: 'POST', body: small, headers: { 'content-type': 'application/octet-stream' } });
      });
      toast(t('saved')); await loadVenue(); openDish(id); rerender();
    } catch (err) { toast(String(err.message || err)); }
  };
  const clr = $('#photoClear'); if (clr) clr.onclick = async () => {
    try { await busy(clr, () => post(`/owner/products/${encodeURIComponent(id)}/image/clear`, withLoc())); await loadVenue(); openDish(id); rerender(); } catch (err) { toast(String(err.message || err)); }
  };
  $('#dSave').onclick = async () => {
    const avail = $('#d-avail').checked;
    const nutrition = {};
    for (const [k, el] of [['kcal', '#d-kcal'], ['protein', '#d-prot'], ['fat', '#d-fat'], ['carbs', '#d-carb']]) { const v = num($(el).value); if (v !== null) nutrition[k] = v; }
    const translations = {};
    for (const l of LANGS) { const nm = $(`#d-name-${l}`), ds = $(`#d-desc-${l}`); if (nm || ds) translations[l] = { ...(nm ? { name: nm.value.trim() } : {}), ...(ds ? { description: ds.value.trim() } : {}) }; }
    const body = withLoc({
      available: avail, unavailable_note: avail ? null : ($('#d-note').value.trim() || null),
      price: num($('#d-price').value) ?? p.price,
      cooking_min: num($('#d-cook').value),
      // Sent only when changed: a save never moves a dish between stations by accident.
      ...($('#d-station').value !== (p.station || 'kitchen') ? { station: $('#d-station').value } : {}),
      tags: $$('[data-tag].on', $('#sheetIn')).map(b => b.dataset.tag),
      ingredients: $('#d-ings').value.split(',').map(s => s.trim()).filter(Boolean),
      nutrition: Object.keys(nutrition).length ? nutrition : null,
      weight_g: num($('#d-weight').value),
      ...(num($('#d-size').value) != null ? { size_cm: num($('#d-size').value) } : {}),
      // Only a recipe READ from the owner route is sent back: an unread one is
      // not an empty one, and sending [] would clear it.
      ...(recipeKnown === id ? { bom: recipeDraft.map(l => ({ supply: l.supply, qty: l.qty })) } : {}),
      taste: tasteDraft,
      translations,
    });
    for (const k of Object.keys(body)) if (body[k] === null) delete body[k];
    try { await busy($('#dSave'), () => post(`/owner/products/${encodeURIComponent(id)}`, body)); toast(t('saved')); await loadVenue(); closeSheet(); rerender(); }
    catch (err) { toast(String(err.message || err)); }
  };
}


/// A new dish: the three things it cannot exist without; the editor opens on it.
/// Every category, empty ones too: the public menu hides those, the owner needs them.
async function allCategories(){ try { return (await api('/owner/categories')).categories || []; } catch { return (S.categories || []).map(c => ({ id: c.id, name: c.name, count: (c.products || []).length })); } }
async function openNewDish(){
  const cats = await allCategories();
  sheet(`<p class="eyebrow" data-t="tabMenu"></p><h2 data-t="addDish"></h2>
    <label for="nd-name" data-t="name"></label><input id="nd-name" autocomplete="off">
    <label for="nd-cat" data-t="category"></label><select id="nd-cat">${cats.map(c => `<option value="${esc(c.id)}" ${c.id === view.cat ? 'selected' : ''}>${esc(c.name)}</option>`).join('')}</select>
    <label for="nd-price" data-t="price"></label><input id="nd-price" inputmode="numeric">
    <label for="nd-desc" data-t="description"></label><textarea id="nd-desc" rows="2"></textarea>
    ${cats.length ? '' : `<p class="hint" data-t="needCategory"></p>`}
    <div class="btn-row"><button class="btn" id="ndGo" ${cats.length ? '' : 'disabled'}>${icon('plus')}<span data-t="addDish"></span></button></div>`, { name: 'newdish' });
  $('#ndGo').onclick = async () => {
    const name = $('#nd-name').value.trim(); const price = num($('#nd-price').value);
    if (!name || price == null) return toast(t('required'));
    try {
      const r = await busy($('#ndGo'), () => post('/owner/products', withLoc({ name, category_id: $('#nd-cat').value, price, description: $('#nd-desc').value.trim() })));
      toast(t('saved')); await loadVenue();
      // The public menu may lag a write by its cache; the editor opens on the
      // record the hub just returned, not on what the menu shows yet.
      if (!S.products.some(x => x.id === r.id)) { const cat = cats.find(c => c.id === r.categoryId); S.products.push({ ...r, categoryId: r.categoryId, categoryName: cat?.name || '', translations: {} }); }
      rerender(); view.open.add(r.categoryId); openDish(r.id);
    } catch (e) { toast(String(e.message || e)); }
  };
}

/// The categories: add one, rename one, remove an empty one.
async function openCategories(){
  const cats = await allCategories();
  sheet(`<p class="eyebrow" data-t="tabMenu"></p><h2 data-t="categories"></h2>
    <div class="rows">${cats.map(c => `<div class="rowc"><span class="t"><input class="inline" data-cn="${esc(c.id)}" value="${esc(c.name)}"><small class="mono">${c.count ?? 0} · <span data-t="dishes"></span></small></span>
      <button type="button" class="act" data-cs="${esc(c.id)}">${icon('check')}</button><button type="button" class="act danger" data-cd="${esc(c.id)}" ${c.count ? 'disabled' : ''}>${icon('trash')}</button></div>`).join('')}</div>
    <label for="nc-name" class="mt-3" data-t="addCategory"></label><div class="grid2"><input id="nc-name" autocomplete="off"><button type="button" class="btn" id="ncGo">${icon('plus')}<span data-t="add"></span></button></div>`, { name: 'cats' });
  const fail = e => toast(String(e.message || e));
  $('#ncGo').onclick = async () => { const name = $('#nc-name').value.trim(); if (!name) return toast(t('required')); try { await busy($('#ncGo'), () => post('/owner/categories', withLoc({ name }))); await loadVenue(); openCategories(); rerender(); } catch (e) { fail(e); } };
  for (const b of $$('[data-cs]', $('#sheetIn'))) b.onclick = async () => { const name = $(`[data-cn="${b.dataset.cs}"]`).value.trim(); if (!name) return; try { await busy(b, () => post('/owner/categories', withLoc({ id: b.dataset.cs, name }))); toast(t('saved')); await loadVenue(); rerender(); } catch (e) { fail(e); } };
  for (const b of $$('[data-cd]', $('#sheetIn'))) b.onclick = async () => { const ok = await confirm(t('remove'), $(`[data-cn="${b.dataset.cd}"]`).value, { danger: true }); if (!ok) return openCategories(); try { await post(`/owner/categories/${encodeURIComponent(b.dataset.cd)}/delete`, withLoc()); await loadVenue(); openCategories(); rerender(); } catch (e) { fail(e); } };
}


// ── the recipe: one portion's components, and what they add up to ────────────
/// The recipe being edited; lines carry the supply's snapshot for the sums.
let recipeDraft = [];
/// The dish whose stored recipe was read (`GET /owner/products?id=`); null until then.
let recipeKnown = null;
let supplyBook = null;
const TASTE_AXES = ['spicy', 'sweet', 'salty', 'sour', 'richness'];
const TASTE_ICONS = { spicy: 'pepper', sweet: 'candy', salty: 'salt', sour: 'lemon-2', richness: 'flame' };
const TASTE_LEVELS = [1, 2, 3];
let tasteDraft = {};
const KIND_ICON = { food_ingredient: 'meat', condiment: 'bottle', packaging: 'box', utensil: 'tool' };
const isFoodKind = k => k === 'food_ingredient' || k === 'condiment';
const basisOf = u => u === 'unit' ? 1 : 100;
/// The stepper moves by ten grams or millilitres, by one piece.
const STEP_MASS = 10, STEP_PIECE = 1;

async function supplies(){ if (!supplyBook) { try { supplyBook = (await api('/owner/stock')).supplies || []; } catch { supplyBook = []; } } return supplyBook; }
/// A line scaled from its supply, the way the hub will scale it on save.
function lineOf(sup, qty){
  const ratio = qty / basisOf(sup.unit), food = isFoodKind(sup.kind);
  const sc = v => food && v != null ? v * ratio : null;
  return { supply: sup.id, qty, name: sup.name, unit: sup.unit, kind: sup.kind, kcal: sc(sup.kcalPer100), protein: sc(sup.proteinPer100), fat: sc(sup.fatPer100), carbs: sc(sup.carbsPer100),
    cost: sup.costPerBasis != null ? Math.round(sup.costPerBasis * ratio) : null, weightG: sup.unit === 'unit' ? (sup.weightPerUnit != null ? sup.weightPerUnit * qty : null) : qty };
}
function derived(lines){
  const food = lines.filter(l => isFoodKind(l.kind));
  const sum = k => Math.round(food.reduce((a, l) => a + (l[k] ?? 0), 0));
  return { kcal: sum('kcal'), protein: sum('protein'), fat: sum('fat'), carbs: sum('carbs'), complete: food.length > 0 && food.every(l => l.kcal != null),
    cost: lines.length && lines.every(l => l.cost != null) ? lines.reduce((a, l) => a + l.cost, 0) : null,
    weightG: food.length && food.every(l => l.weightG != null) ? Math.round(food.reduce((a, l) => a + l.weightG, 0)) : null };
}
function recipeMarkup(p){
  recipeDraft = (p.bom || []).map(l => ({ ...l })); recipeKnown = null;
  return `<p class="eyebrow mt-3" data-t="recipe"></p><p class="muted small" data-t="recipeHint"></p>
    <div id="rcLines"></div>
    <details class="fold" id="rcPick"><summary>${icon('plus')} <span data-t="addSupplyToRecipe"></span></summary>
      <label class="srch mt-2">${icon('search')}<input id="rcQ" type="search" data-t-attr="placeholder:search"></label>
      <div class="chips filters" id="rcKinds"><button type="button" class="chip on" data-rk="all"><span data-t="all"></span></button>${Object.entries(KIND_ICON).map(([k, ic]) => `<button type="button" class="chip" data-rk="${k}">${icon(ic)}<span data-t="kind_${k}"></span></button>`).join('')}</div>
      <div id="rcBook" class="rows"><div class="skel skel-row"></div></div>
      <div class="btn-row"><button type="button" class="btn ghost" id="rcAdd">${icon('plus')}<span data-t="addSelected"></span></button></div>
    </details>
    <div id="rcSum" class="rc-sum"></div>`;
}
function drawRecipe(p){
  const host = $('#rcLines'); if (!host) return;
  host.innerHTML = recipeDraft.length ? recipeDraft.map((l, i) => `<div class="rc-line"><span class="rc-ic">${icon(KIND_ICON[l.kind] || 'meat')}</span>
      <span class="t"><b>${esc(l.name)}</b><small class="mono">${l.kcal != null ? `${Math.round(l.kcal)} kcal` : isFoodKind(l.kind) ? `<span class="pill warn" data-t="noData"></span>` : ''}${l.cost != null ? ` · ${money(l.cost)}` : ''}</small></span>
      <span class="rc-qty"><button type="button" class="act" data-rm="${i}">−</button><input class="mono" data-rq="${i}" inputmode="numeric" value="${l.qty}"><span class="mono">${esc(l.unit)}</span><button type="button" class="act" data-rp="${i}">+</button></span>
      <button type="button" class="act danger" data-rx="${i}">${icon('x')}</button></div>`).join('')
    : `<p class="hint" data-t="noRecipe"></p>`;
  const d = derived(recipeDraft);
  const cost = d.cost, price = num($('#d-price')?.value) ?? p.price ?? 0;
  $('#rcSum').innerHTML = recipeDraft.length ? `<div class="stats strip">
      <div class="stat"><b>${d.kcal}</b><small data-t="kcal"></small></div><div class="stat"><b>${d.protein}</b><small data-t="protein"></small></div><div class="stat"><b>${d.fat}</b><small data-t="fat"></small></div><div class="stat"><b>${d.carbs}</b><small data-t="carbs"></small></div></div>
    <p class="hint ${d.complete ? 'ok' : ''}" data-t="${d.complete ? 'nutritionPerServing' : 'nutritionIncomplete'}"></p>
    <p class="hint mono">${d.weightG != null ? `${t('weight')}: ${d.weightG} g · ` : ''}${cost != null ? `${t('foodCost')}: ${money(cost)}${price ? ` · ${Math.round(100 * cost / price)}%` : ''}` : t('costUnknown')}</p>` : '';
  retranslate($('#sheetIn'));
  const step = l => l.unit === 'unit' ? STEP_PIECE : STEP_MASS;
  const setQty = (i, q) => { if (q <= 0) { recipeDraft.splice(i, 1); } else { const sup = supplyBook?.find(s => s.id === recipeDraft[i].supply); recipeDraft[i] = sup ? lineOf(sup, q) : { ...recipeDraft[i], qty: q }; } drawRecipe(p); };
  for (const b of $$('[data-rm]', host)) b.onclick = () => setQty(+b.dataset.rm, recipeDraft[+b.dataset.rm].qty - step(recipeDraft[+b.dataset.rm]));
  for (const b of $$('[data-rp]', host)) b.onclick = () => setQty(+b.dataset.rp, recipeDraft[+b.dataset.rp].qty + step(recipeDraft[+b.dataset.rp]));
  for (const b of $$('[data-rx]', host)) b.onclick = () => setQty(+b.dataset.rx, 0);
  for (const inp of $$('[data-rq]', host)) inp.onchange = () => { const q = Math.round(num(inp.value) ?? 0); setQty(+inp.dataset.rq, q); };
}
/// The dish's recipe AS STORED: the public menu the console loads never carries it.
async function storedBom(id){
  const r = await api(`/owner/products?id=${encodeURIComponent(id)}`);
  const p = (r?.products || []).find(x => x.id === id);
  if (!p) throw new Error('unknown product');
  return p.bom || [];
}
async function bindRecipe(p){
  const book = await supplies();
  try { recipeDraft = (await storedBom(p.id)).map(l => ({ ...l })); recipeKnown = p.id; } catch { recipeKnown = null; }
  // Lines loaded from the dish are re-scaled from today's supply numbers, as the hub does on save.
  recipeDraft = recipeDraft.map(l => { const sup = book.find(s => s.id === l.supply); return sup ? lineOf(sup, l.qty) : l; });
  drawRecipe(p);
  let kind = 'all';
  const drawBook = () => {
    const q = String($('#rcQ')?.value || '').toLowerCase().trim();
    const list = book.filter(s => (kind === 'all' || s.kind === kind) && (!q || `${s.name} ${s.category}`.toLowerCase().includes(q)) && !recipeDraft.some(l => l.supply === s.id));
    $('#rcBook').innerHTML = list.length ? list.slice(0, 60).map(s => `<label class="rowc pick"><input type="checkbox" data-rs="${esc(s.id)}">${icon(KIND_ICON[s.kind] || 'meat')}<span class="t"><b>${esc(s.name)}</b><small class="mono">${esc(s.category || '')} · ${esc(s.unit)}${isFoodKind(s.kind) && s.kcalPer100 == null ? ` · <span data-t="noData"></span>` : ''}</small></span></label>`).join('') : `<p class="hint" data-t="${book.length ? 'none' : 'noStock'}"></p>`;
    retranslate($('#rcBook'));
  };
  $('#rcQ').oninput = drawBook;
  for (const b of $$('[data-rk]', $('#rcKinds'))) b.onclick = () => { kind = b.dataset.rk; for (const x of $$('[data-rk]', $('#rcKinds'))) x.classList.toggle('on', x === b); drawBook(); };
  $('#rcAdd').onclick = () => {
    for (const cb of $$('[data-rs]:checked', $('#rcBook'))) { const sup = book.find(s => s.id === cb.dataset.rs); if (sup && !recipeDraft.some(l => l.supply === sup.id)) recipeDraft.push(lineOf(sup, sup.unit === 'unit' ? 1 : 100)); }
    drawRecipe(p); drawBook();
  };
  $('#d-price').oninput = () => drawRecipe(p);
  drawBook();
}

// ── taste: five axes, three levels, authored by the kitchen ─────────────────
function tasteMarkup(p){
  tasteDraft = { ...(p.taste || {}) };
  return `<p class="eyebrow mt-3" data-t="taste"></p><p class="muted small" data-t="tasteHint"></p>
    <div class="taste" id="tasteBox">${TASTE_AXES.map(a => `<div class="taste-row"><span class="taste-ax">${icon(TASTE_ICONS[a])}<span data-t="taste_${a}"></span></span>
      <span class="taste-lv">${TASTE_LEVELS.map(l => `<button type="button" class="chip ${tasteDraft[a] === l ? 'on' : ''}" data-ta="${a}" data-tl="${l}"><span data-t="tlevel_${l}"></span></button>`).join('')}</span></div>`).join('')}</div>`;
}
function bindTaste(){
  for (const b of $$('[data-ta]', $('#tasteBox'))) b.onclick = () => {
    const a = b.dataset.ta, l = +b.dataset.tl;
    if (tasteDraft[a] === l) delete tasteDraft[a]; else tasteDraft[a] = l;   // re-tapping the level clears the axis, as the old console did
    for (const x of $$(`[data-ta="${a}"]`, $('#tasteBox'))) x.classList.toggle('on', tasteDraft[a] === +x.dataset.tl);
  };
}
