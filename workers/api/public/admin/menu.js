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
import { publishedFields, FIELDS } from '/lib/dish-edit.js';
import { ui, k, btn, iconBtn, field, input, select, chips, press, pill, empty, loading, rowBtn, rowDiv, check } from '/admin/parts.js';
/// A filter chip whose data-* the screen's click handler reads.
const fchip = (on, data, label, tour) => ui.chip({ as: 'button', selected: on, label, attrs: { data: { ...data, tour } } });
const search = (id, value, tour) => `<div class="srch">${icon('search')}${ui.inputRow({ id, type: 'search', label: k('search'), placeholder: k('search'), attrs: { value, data: { tour } } })}</div>`;

/// The dish sheet's box for each tracked field (`lib/dish-edit.js`).
const BOX = { ings: '#d-ings', kcal: '#d-kcal', protein: '#d-prot', fat: '#d-fat', carbs: '#d-carb', weight: '#d-weight' };

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
  return rowBtn({ cls: p.available ? '' : 'off', data: { p: p.id }, tour: 'menu.dish', title: p.name, sub: esc(p.description || ''),
    leading: p.imageUrl ? `<img class="thumb" src="${esc(p.imageUrl)}" alt="" loading="lazy">` : `<span class="thumb"></span>`,
    trailing: `${ui.amount(String(money(p.price)))}${pill(p.available ? 'ok' : 'bad', { key: p.available ? 'onSale' : 'stopList' })}` });
}

export async function render(host){
  const cats = S.categories;
  const q = norm(view.q).trim();
  host.innerHTML = `
    <div class="screen-h"><div><p class="eyebrow" data-t="tabMenu"></p><h1>${esc(S.venue?.name || '')}</h1></div>
      <div class="btn-row compact">${btn({ id: 'mCats', icon: 'adjustments', key: 'categories', tour: 'menu.categories' })}${iconBtn({ id: 'mImport', icon: 'download', ariaKey: 'importMenu', tour: 'menu.import' })}${iconBtn({ id: 'mRecipes', icon: 'tools-kitchen-2', ariaKey: 'importRecipes', tour: 'menu.importRecipes' })}${btn({ id: 'mNew', variant: 'primary', icon: 'plus', key: 'addDish', tour: 'menu.addDish' })}</div></div>
    <p class="screen-hint" data-t="menuHint"></p>
    ${search('mq', view.q, 'menu.search')}
    <div class="chips filters" role="group">
      ${fchip(!view.cat, { fc: '' }, k('allDishes'), 'menu.filterCategory')}
      ${cats.map(c => fchip(view.cat === c.id, { fc: c.id }, c.name, 'menu.filterCategory')).join('')}
    </div>
    <div class="chips filters" role="group">
      ${fchip(view.state === 'on', { fs: 'on' }, k('onSale'), 'menu.filterOnSale')}
      ${fchip(view.state === 'off', { fs: 'off' }, k('stopList'), 'menu.filterStopList')}
      ${fchip(view.photo === 'none', { fp: 'none' }, k('withoutPhoto'), 'menu.filterNoPhoto')}
    </div>
    ${select({ id: 'mSort', ariaLabel: t('sort'), controlCls: 'sortsel', value: view.sort, options: SORTS.map(s => ({ value: s, key: 'sort_' + s })), tour: 'menu.sort' })}
    ${cats.filter(c => !view.cat || c.id === view.cat).map(c => { const open = q || filtering() ? true : view.open.has(c.id); const rows = visibleOf(c).map(dishRow).join(''); if ((q || filtering()) && !rows) return ''; return `
      <section class="group">
        ${rowBtn({ cls: 'cat-h', title: c.name, data: { cat: c.id }, attrs: { 'aria-expanded': String(open) }, tour: 'menu.category', trailing: `<span class="n">${(c.products || []).length}</span>${icon('chevron-down', 'chev')}` })}
        <div class="rows" ${open ? '' : 'hidden'}>${rows}</div>
      </section>`; }).join('')}
    ${!cats.length ? empty('bowl-chopsticks', { key: 'loadFail', alert: true }) : ''}`;
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
    ${input({ id: 'imFile', type: 'file', accept: '.csv,text/csv', tour: 'import.file' })}
    ${switchEl('imRetire', false, 'retireMissing', 'retireHint', 'import.retire')}
    <div class="btn-row">${btn({ id: 'imDry', icon: 'eye', key: 'dryRun', disabled: true, tour: 'import.dryRun' })}${btn({ id: 'imApply', variant: 'primary', icon: 'check', key: 'applyImport', disabled: true, tour: 'import.apply' })}</div>
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
    ${rowDiv({ id: 'photoRow', title: k('photo'), sub: `<span data-t="${p.imageUrl ? 'photo' : 'noPhoto'}"></span>`, leading: p.imageUrl ? `<img class="thumb" src="${esc(p.imageUrl)}" alt="">` : `<span class="thumb"></span>`,
      trailing: `${btn({ id: 'photoPick', icon: 'camera-plus', key: 'uploadPhoto', tour: 'dish.photo' })}${p.imageUrl ? iconBtn({ id: 'photoClear', icon: 'trash', ariaKey: 'remove', tour: 'dish.photoClear' }) : ''}
      ${input({ id: 'photoFile', type: 'file', accept: 'image/*', hidden: true, tour: 'dish.photoFile' })}` })}
    ${switchEl('d-avail', p.available, 'onSale', null, 'dish.onSale')}
    <div id="offBox" ${p.available ? 'hidden' : ''}>${field({ id: 'd-note', key: 'unavailableNote', value: p.unavailableNote || '', tour: 'dish.offNote' })}</div>
    <div class="grid2">
      <div>${field({ id: 'd-price', key: 'price', inputmode: 'numeric', value: p.price ?? '', tour: 'dish.price' })}</div>
      <div>${field({ id: 'd-cook', key: 'cookingMin', inputmode: 'numeric', value: p.cookingMin ?? '' })}</div>
    </div>
    ${select({ id: 'd-station', key: 'station', value: p.station || 'kitchen', options: STATIONS.map(s => ({ value: s, key: 'station_' + s })), tour: 'dish.station' })}<p class="hint" data-t="stationHint"></p>
    <p class="ui-label" data-t="tags"></p>
    <div class="chips" id="tagPick" role="group">${TAGS.map(tg => ui.chip({ as: 'button', selected: tags.has(tg), label: tg, attrs: { data: { tag: tg, tour: 'dish.tag' } } })).join('')}</div>
    ${recipeMarkup(p)}
    ${tasteMarkup(p)}
    ${field({ id: 'd-ings', key: 'ingredients', rows: 2, value: (p.ingredients || []).join(', '), tour: 'dish.ingredients' })}
    <p class="ui-label" data-t="nutrition"></p>${p.nutritionDerived ? `<p class="hint" data-t="nutritionFromRecipe"></p>` : ''}
    <div class="grid3">
      <div>${field({ id: 'd-kcal', key: 'kcal', inputmode: 'numeric', value: n.kcal ?? '' })}</div>
      <div>${field({ id: 'd-prot', key: 'protein', inputmode: 'numeric', value: n.protein ?? '' })}</div>
      <div>${field({ id: 'd-fat', key: 'fat', inputmode: 'numeric', value: n.fat ?? '' })}</div>
    </div>
    <div class="grid2">
      <div>${field({ id: 'd-carb', key: 'carbs', inputmode: 'numeric', value: n.carbs ?? '' })}</div>
      <div>${field({ id: 'd-weight', key: 'weight', inputmode: 'numeric', value: p.weightG ?? '' })}</div>
    </div>
    ${field({ id: 'd-size', key: 'sizeCm', inputmode: 'numeric', hintKey: 'sizeCmHint', value: p.sizeCm ?? '', attrs: { min: SIZE_CM_MIN, max: SIZE_CM_MAX } })}
    <p class="ui-label" data-t="translations"></p>
    ${LANGS.filter(l => l !== (S.venue?.defaultLocale || 'sq')).map(l => `<div class="grid2">
      <div>${field({ id: `d-name-${l}`, label: `${l.toUpperCase()} · ${t('name')}`, value: tr[l]?.name || '', tour: 'dish.translation' })}</div>
      <div>${field({ id: `d-desc-${l}`, label: `${l.toUpperCase()} · ${t('description')}`, value: tr[l]?.description || '' })}</div>
    </div>`).join('')}
    <div class="btn-row">${btn({ id: 'dSave', variant: 'primary', icon: 'check', key: 'save', tour: 'dish.save' })}</div>
    <div class="btn-row">${btn({ id: 'dDel', variant: 'danger', icon: 'trash', key: 'deleteDish', tour: 'dish.delete' })}</div>`, { name: 'dish' });
  $('#dDel').onclick = async () => {
    const ok = await confirm(t('deleteDish'), `${p.name} · ${t('deleteDishHint')}`, { danger: true });
    if (!ok) return openDish(id);
    try { await post(`/owner/products/${encodeURIComponent(id)}/delete`, withLoc()); toast(t('saved')); closeSheet(); await loadVenue(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
  $('#d-avail').onchange = e => { $('#offBox').hidden = e.target.checked; };
  // What the owner changes in THIS sheet; the rest follows the recipe (audit D19).
  const edited = new Set();
  for (const f of FIELDS) $(BOX[f]).addEventListener('input', () => edited.add(f));
  bindRecipe(p); bindTaste(p);
  for (const b of $$('[data-tag]', $('#sheetIn'))) b.onclick = () => b.setAttribute('aria-pressed', String(b.getAttribute('aria-pressed') !== 'true'));
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
    const values = Object.fromEntries(FIELDS.map(f => [f, $(BOX[f]).value]));
    const translations = {};
    for (const l of LANGS) { const nm = $(`#d-name-${l}`), ds = $(`#d-desc-${l}`); if (nm || ds) translations[l] = { ...(nm ? { name: nm.value.trim() } : {}), ...(ds ? { description: ds.value.trim() } : {}) }; }
    const body = withLoc({
      available: avail, unavailable_note: avail ? null : ($('#d-note').value.trim() || null),
      price: num($('#d-price').value) ?? p.price,
      cooking_min: num($('#d-cook').value),
      // Sent only when changed: a save never moves a dish between stations by accident.
      ...($('#d-station').value !== (p.station || 'kitchen') ? { station: $('#d-station').value } : {}),
      tags: $$('[data-tag][aria-pressed="true"]', $('#sheetIn')).map(b => b.dataset.tag),
      // Ingredients, nutrition and weight only when edited here: a prefilled
      // value sent back was stored as typed and hid every recipe change.
      ...publishedFields(values, edited),
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
    ${field({ id: 'nd-name', key: 'name', autocomplete: 'off', tour: 'newDish.name' })}
    ${select({ id: 'nd-cat', key: 'category', value: view.cat, options: cats.map(c => ({ value: c.id, label: c.name })), tour: 'newDish.category' })}
    ${field({ id: 'nd-price', key: 'price', inputmode: 'numeric', tour: 'newDish.price' })}
    ${field({ id: 'nd-desc', key: 'description', rows: 2 })}
    ${cats.length ? '' : `<p class="hint" data-t="needCategory"></p>`}
    <div class="btn-row">${btn({ id: 'ndGo', variant: 'primary', icon: 'plus', key: 'addDish', disabled: !cats.length, tour: 'newDish.save' })}</div>`, { name: 'newdish' });
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
    <div class="rows">${cats.map(c => rowDiv({ title: '', sub: `${ui.inputRow({ label: k('name'), cls: 'inline', attrs: { value: c.name, data: { cn: c.id, tour: 'category.name' } } })}<span class="mono">${c.count ?? 0} · <span data-t="dishes"></span></span>`,
      trailing: `${iconBtn({ icon: 'check', ariaKey: 'save', data: { cs: c.id }, tour: 'category.save' })}${iconBtn({ icon: 'trash', ariaKey: 'remove', disabled: !!c.count, data: { cd: c.id }, tour: 'category.remove' })}` })).join('')}</div>
    <div class="grid2 mt-3">${field({ id: 'nc-name', key: 'addCategory', autocomplete: 'off', tour: 'menu.newCategory' })}${btn({ id: 'ncGo', variant: 'primary', icon: 'plus', key: 'add', tour: 'menu.addCategory' })}</div>`, { name: 'cats' });
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
      ${search('rcQ', '', 'recipe.search')}
      ${chips({ id: 'rcKinds', values: [{ value: 'all', key: 'all' }, ...Object.entries(KIND_ICON).map(([kd, ic]) => ({ value: kd, key: 'kind_' + kd, icon: ic }))], value: 'all', attr: 'rk', labelKey: 'kind', tour: 'recipe.kind' }).replace('class="chips"', 'class="chips filters"')}
      <div id="rcBook" class="rows">${loading()}</div>
      <div class="btn-row">${btn({ id: 'rcAdd', icon: 'plus', key: 'addSelected', tour: 'recipe.addLine' })}</div>
    </details>
    <div id="rcSum" class="rc-sum"></div>`;
}
const step = l => l.unit === 'unit' ? STEP_PIECE : STEP_MASS;
function drawRecipe(p){
  const host = $('#rcLines'); if (!host) return;
  host.innerHTML = recipeDraft.length ? recipeDraft.map((l, i) => `<div class="rc-line"><span class="rc-ic">${icon(KIND_ICON[l.kind] || 'meat')}</span>
      <span class="t"><b>${esc(l.name)}</b><small class="mono">${l.kcal != null ? `${Math.round(l.kcal)} kcal` : isFoodKind(l.kind) ? pill('warn', { key: 'noData' }) : ''}${l.cost != null ? ` · ${money(l.cost)}` : ''}</small></span>
      <span class="rc-qty">${iconBtn({ icon: 'minus', ariaLabel: `- ${step(l)}`, data: { rm: i }, tour: 'recipe.less' })}${ui.inputRow({ label: l.name, cls: 'mono', attrs: { value: String(l.qty), inputmode: 'numeric', data: { rq: i, tour: 'recipe.qty' } } })}<span class="mono">${esc(l.unit)}</span>${iconBtn({ icon: 'plus', ariaLabel: `+ ${step(l)}`, data: { rp: i }, tour: 'recipe.more' })}</span>
      ${iconBtn({ icon: 'x', ariaKey: 'remove', data: { rx: i }, tour: 'recipe.removeLine' })}</div>`).join('')
    : `<p class="hint" data-t="noRecipe"></p>`;
  const d = derived(recipeDraft);
  const cost = d.cost, price = num($('#d-price')?.value) ?? p.price ?? 0;
  $('#rcSum').innerHTML = recipeDraft.length ? `<div class="stats strip">
      <div class="stat"><b>${d.kcal}</b><small data-t="kcal"></small></div><div class="stat"><b>${d.protein}</b><small data-t="protein"></small></div><div class="stat"><b>${d.fat}</b><small data-t="fat"></small></div><div class="stat"><b>${d.carbs}</b><small data-t="carbs"></small></div></div>
    <p class="hint ${d.complete ? 'ok' : ''}" data-t="${d.complete ? 'nutritionPerServing' : 'nutritionIncomplete'}"></p>
    <p class="hint mono">${d.weightG != null ? `${t('weight')}: ${d.weightG} g · ` : ''}${cost != null ? `${t('foodCost')}: ${money(cost)}${price ? ` · ${Math.round(100 * cost / price)}%` : ''}` : t('costUnknown')}</p>` : '';
  retranslate($('#sheetIn'));
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
    $('#rcBook').innerHTML = list.length ? list.slice(0, 60).map(s => check({ cls: 'pick', data: { rs: s.id }, tour: 'recipe.pick',
      label: `${s.name} · ${s.category || ''} · ${s.unit}${isFoodKind(s.kind) && s.kcalPer100 == null ? ` · ${t('noData')}` : ''}` })).join('') : `<p class="hint" data-t="${book.length ? 'none' : 'noStock'}"></p>`;
    retranslate($('#rcBook'));
  };
  $('#rcQ').oninput = drawBook;
  for (const b of $$('[data-rk]', $('#rcKinds'))) b.onclick = () => { kind = b.dataset.rk; press($$('[data-rk]', $('#rcKinds')), b); drawBook(); };
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
      <span class="taste-lv">${TASTE_LEVELS.map(l => ui.chip({ as: 'button', selected: tasteDraft[a] === l, label: k('tlevel_' + l), attrs: { data: { ta: a, tl: l, tour: 'taste.' + a } } })).join('')}</span></div>`).join('')}</div>`;
}
function bindTaste(){
  for (const b of $$('[data-ta]', $('#tasteBox'))) b.onclick = () => {
    const a = b.dataset.ta, l = +b.dataset.tl;
    if (tasteDraft[a] === l) delete tasteDraft[a]; else tasteDraft[a] = l;   // re-tapping the level clears the axis, as the old console did
    for (const x of $$(`[data-ta="${a}"]`, $('#tasteBox'))) x.setAttribute('aria-pressed', String(tasteDraft[a] === +x.dataset.tl));
  };
}
