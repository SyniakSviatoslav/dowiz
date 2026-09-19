// The menu -- what the venue sells, category by category.
//
// A category folds; a dish is a row with its photograph, price and whether it
// is on sale; a tap opens the dish as a sheet where everything the storefront
// shows can be set: price, stop-list with a reason, photograph, ingredients,
// nutrition, cooking time, tags, and the name and description in the other
// two languages. The stop-list is one switch: a dish off sale is off for the
// customer within the menu's thirty-second cache.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, money, moneyEl, busy, switchEl, store, retranslate } from '/admin/core.js';
import { lang, LANGS } from '/admin/i18n.js';
import { loadVenue, rerender } from '/admin/app.js';

/// The tags the storefront knows how to filter and draw.
const TAGS = ['popular', 'salmon', 'tuna', 'shrimp', 'vegetarian', 'hot'];
/// A photograph is shrunk to this on the phone before upload.
const PHOTO_MAX_PX = 1600;
const PHOTO_QUALITY = 0.86;
const view = { open: new Set(), q: '' };

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
      <button type="button" class="act" id="mImport">${icon('download')}<span data-t="importMenu"></span></button></div>
    <label class="srch">${icon('search')}<input id="mq" type="search" value="${esc(view.q)}" data-t-attr="placeholder:search"></label>
    ${cats.map(c => { const open = q ? true : view.open.has(c.id); const rows = (c.products || []).map(dishRow).join(''); if (q && !rows) return ''; return `
      <section class="group">
        <button type="button" class="cat-h" data-cat="${esc(c.id)}" aria-expanded="${open}">${esc(c.name)}<span class="n">${(c.products || []).length}</span>${icon('chevron-down', 'chev')}</button>
        <div class="rows" ${open ? '' : 'hidden'}>${rows}</div>
      </section>`; }).join('')}
    ${!cats.length ? `<div class="empty">${icon('bowl-chopsticks')}<b data-t="loadFail"></b></div>` : ''}`;
  host.onclick = e => {
    const h = e.target.closest('[data-cat]'); if (h) { const id = h.dataset.cat; view.open.has(id) ? view.open.delete(id) : view.open.add(id); const rows = h.nextElementSibling; rows.hidden = !view.open.has(id); h.setAttribute('aria-expanded', String(view.open.has(id))); return; }
    const r = e.target.closest('[data-p]'); if (r) openDish(r.dataset.p);
  };
  const mq = $('#mq', host); mq.oninput = () => { view.q = mq.value; rerender().then(() => $('#mq')?.focus()); };
  $('#mImport', host).onclick = openImport;
}

/// The plate's diameter, for the storefront's "see it on the table" view.
const SIZE_CM_MIN = 3, SIZE_CM_MAX = 120;

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
    <label data-t="tags"></label>
    <div class="chips" id="tagPick">${TAGS.map(tg => `<button type="button" class="chip ${tags.has(tg) ? 'on' : ''}" data-tag="${tg}">${esc(tg)}</button>`).join('')}</div>
    <label for="d-ings" data-t="ingredients"></label><textarea id="d-ings" rows="2">${esc((p.ingredients || []).join(', '))}</textarea>
    <label data-t="nutrition"></label>
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
    <div class="btn-row"><button class="btn" id="dSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'dish' });
  $('#d-avail').onchange = e => { $('#offBox').hidden = e.target.checked; };
  for (const b of $$('[data-tag]', $('#sheetIn'))) b.onclick = () => b.classList.toggle('on');
  $('#photoPick').onclick = () => $('#photoFile').click();
  $('#photoFile').onchange = async e => {
    const f = e.target.files?.[0]; if (!f) return;
    try {
      const { shrinkImage } = await import('/lib/shrink.js');
      const blob = await shrinkImage(f, { max: PHOTO_MAX_PX, quality: PHOTO_QUALITY }).catch(() => f);
      await busy($('#photoPick'), () => api(`/owner/products/${encodeURIComponent(id)}/image`, { method: 'POST', body: blob, headers: { 'content-type': 'application/octet-stream' } }));
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
      tags: $$('[data-tag].on', $('#sheetIn')).map(b => b.dataset.tag),
      ingredients: $('#d-ings').value.split(',').map(s => s.trim()).filter(Boolean),
      nutrition: Object.keys(nutrition).length ? nutrition : null,
      weight_g: num($('#d-weight').value),
      ...(num($('#d-size').value) != null ? { size_cm: num($('#d-size').value) } : {}),
      translations,
    });
    for (const k of Object.keys(body)) if (body[k] === null) delete body[k];
    try { await busy($('#dSave'), () => post(`/owner/products/${encodeURIComponent(id)}`, body)); toast(t('saved')); await loadVenue(); closeSheet(); rerender(); }
    catch (err) { toast(String(err.message || err)); }
  };
}
