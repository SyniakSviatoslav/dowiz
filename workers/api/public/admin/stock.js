// INGREDIENTS & STOCK, one screen (operator, 2026-09-26): every ingredient
// with what is on the shelf, and the buttons that record what happened.
//
// A level is never a number somebody edits: it is received, written off,
// counted, prepped, and reserved or consumed by orders -- a fold over the
// stock log, which the hub answers in ONE pass (`GET /api/owner/stock`). Each
// ingredient shows its on-hand / reserved / free, its average price, its
// nearest expiry and its state (an uncounted minus is "needs a count", never
// an error), with its four actions under it; a tap opens its card.
//
// "усе має бути відразу видно": the five things a kitchen does with its shelf
// are big tiles at the top, the alarms are chips that filter, and the dishes
// that take nothing off the shelf yet are listed at the bottom with the one
// tap that fixes them.

import '/admin/ingredients-i18n.js';
import '/admin/stock-health-i18n.js';
import { $, $$, esc, icon, t, api, post, withLoc, toast, retranslate, hydrate, money, busy } from '/admin/core.js';
import { lang, st } from '/admin/i18n.js';
import { rerender, me } from '/admin/app.js';
import { openBulk } from '/admin/bulk.js';
import { ui, k as key, btn, iconBtn, select, chips, empty, loading, rowBtn } from '/admin/parts.js';
import * as C from '/admin/ingredients-calc.js';
import { openCard, openSupply, openMove, openPrep } from '/admin/ingredients.js';
import { KINDS, matches, alertsMarkup, rowMarkup, noRecipeMarkup } from '/admin/ingredients-view.js';
import { openCount, openWaste, openDelivery } from '/admin/ingredients-count.js';
import * as N from '/admin/nom-logic.js';
import { openKitchen, ensureCss } from '/admin/kitchen-analytics.js';
import { openPrepList } from '/admin/prep-list.js';
import { drawHealth } from '/admin/stock-health.js';
import { selection, pickRow, barMarkup, bindBar, tapDelete, deleteSupplies, openQuickAdd, ensureNomCss } from '/admin/nom.js';
import { show } from '/admin/app.js';
import * as P from '/admin/prep-logic.js';
import { prepRowMarkup, PREP_ICON } from '/admin/prep-view.js';
import { loadPreps, openPrepCard, openPrepEditor, ensurePrepCss } from '/admin/prep.js';

export { KINDS };
export const UNITS = ['g', 'ml', 'unit'];
export const basisOf = C.basisOf;

/// The five things a kitchen does with its shelf, as tiles.
const TILES = [['delivery', 'package', 'inv_delivery'], ['count', 'check', 'inv_count'], ['prep', 'tools-kitchen-2', 'inv_prep'],
  ['waste', 'trash', 'inv_waste'], ['numbers', 'chart-bar', 'inv_numbers'], ['prepList', 'note', 'pl_tile', 'prepList.open']];

let stock = null;
/// The semi-finished products, hydrated (`GET /api/owner/preps`), by id.
let preps = new Map();
const view = { kind: 'all', q: '', sort: 'name', flag: '' };
/// The three kinds of item (SPEC-SEMI-FINISHED §a): the raw kinds, the
/// semi-finished, and the dishes -- which live on the Menu tab.
const KIND_CHIPS = () => [{ value: 'all', key: 'all' }, ...KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic })),
  { value: P.KIND, key: 'kind_prep', icon: PREP_ICON }, { value: 'dish', key: 'kind_dish', icon: 'bowl-chopsticks' }];
/// W-NOM: the ingredients ticked for a bulk delete (the owner's).
const sel = selection();
const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

/// What every sheet opened from here needs: the answer, and how to refresh it.
const ctx = {
  get data(){ return stock; },
  async reload(){ rerender(); },
};

export async function render(host){
  ensureCss(); ensureNomCss(); ensurePrepCss();
  const owner = !me().staff;
  host.innerHTML = `<div class="screen-h"><div><h1 data-t="inv_title"></h1></div>
    <div class="screen-acts">${me().staff ? '' : iconBtn({ id: 'resetIngredients', icon: 'trash', ariaKey: 'inv_reset' })}${iconBtn({ id: 'importSupplies', icon: 'download', ariaKey: 'importSupplies', tour: 'stock.import' })}${owner ? btn({ id: 'nomSel', variant: 'ghost', icon: 'check', key: sel.on ? 'nom_selectDone' : 'nom_select' }) : ''}${btn({ id: 'addMany', icon: 'note', key: 'nom_addMany' })}${btn({ id: 'addPrep', icon: PREP_ICON, key: 'pf_add', tour: 'pf.add' })}${btn({ id: 'addSupply', variant: 'primary', icon: 'plus', key: 'addSupply', tour: 'stock.addSupply' })}</div></div>
    <p class="screen-hint" data-t="inv_hint"></p>
    <div class="tiles">${TILES.map(([id, ic, word, tour]) => rowBtn({ cls: 'tile', leading: `<span class="tile-ic">${icon(ic)}</span>`, title: key(word), data: { tile: id }, tour })).join('')}</div>
    <div id="invAlerts" class="inv-alerts"></div>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'sq', type: 'search', label: key('search'), placeholder: key('search'), attrs: { value: view.q, data: { tour: 'stock.search' } } })}</div>
    ${chips({ values: KIND_CHIPS(), value: view.kind, attr: 'k', labelKey: 'kind', tour: 'stock.filter' }).replace('class="chips"', 'class="chips filters"')}
    ${select({ id: 'sSort', ariaLabel: t('sort'), controlCls: 'sortsel', value: view.sort, options: ['name', 'category', 'low'].map(k => ({ value: k, key: 'ssort_' + k })), tour: 'stock.sort' })}
    <div id="stockList">${loading(2)}</div>
    <section id="invHealth" class="group mt-3"></section>`;
  retranslate(host);
  $('#addSupply', host).onclick = () => openSupply(null, ctx);
  $('#addMany', host).onclick = () => openQuickAdd(ctx);
  $('#addPrep', host).onclick = () => openPrepEditor(null, ctx);
  const ns = $('#nomSel', host); if (ns) ns.onclick = () => { sel.on = !sel.on; sel.ids = new Set(); rerender(); };
  $('#importSupplies', host).onclick = () => openBulk('supplies', rerender);
  const reset = $('#resetIngredients', host);
  if (reset) reset.onclick = () => resetAll(reset);
  $('#sSort', host).onchange = e => { view.sort = e.target.value; rerender(); };
  $('#sq', host).oninput = e => { view.q = e.target.value; drawList(host); };
  try { stock = await api('/owner/stock'); } catch (e) { $('#stockList', host).innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  preps = new Map((await loadPreps()).map(p => [p.id, p]));
  drawList(host);
  // THE STOCK CHECK (owner only: the rebuild report is on the owner's health route).
  if (!me().staff) drawHealth($('#invHealth', host), { api, stock: () => stock, t, st });
  host.onclick = e => {
    const pk = e.target.closest('[data-pick]'); if (pk) { sel.ids = N.toggle(sel.ids, pk.dataset.pick); return drawList(host); }
    const del = e.target.closest('[data-del]'); if (del) return tapDelete(del, del.dataset.del, () => removeOne(del.dataset.del, del));
    const tile = e.target.closest('[data-tile]'); if (tile) return openTile(tile.dataset.tile);
    const f = e.target.closest('[data-flag]'); if (f) { view.flag = view.flag === f.dataset.flag ? '' : f.dataset.flag; return drawList(host); }
    const k = e.target.closest('[data-k]'); if (k) { if (k.dataset.k === 'dish') return show('menu'); view.kind = k.dataset.k; return rerender(); }
    const pe = e.target.closest('[data-pedit]'); if (pe) return openPrepEditor(preps.get(pe.dataset.pedit), ctx);
    const a = e.target.closest('[data-act]'); if (a) return act(a.dataset.act, a.dataset.s);
    const r = e.target.closest('[data-s]'); if (r) return r.dataset.prep ? openPrepCard(withShelf(r.dataset.s), ctx) : openCard(find(r.dataset.s), ctx);
    const one = e.target.closest('[data-asis]'); if (one) return asIs([one.dataset.asis], one);
    const all = e.target.closest('[data-asiscat]'); if (all) return asIs(noRecipe().filter(d => (d.categoryId || '') === all.dataset.asiscat).map(d => d.id), all);
  };
}

const find = id => (stock?.supplies || []).find(s => s.id === id);

/// The row's red Delete, second tap: one ingredient, for good (W-NOM).
async function removeOne(id, el){
  const r = await deleteSupplies([id], el, { ask: false });
  if (r) ctx.reload();
}
const noRecipe = () => stock?.noRecipe || [];

function openTile(id){
  if (id === 'delivery') return openDelivery(ctx);
  if (id === 'count') return openCount(ctx);
  if (id === 'prep') return openPrep(null, ctx);
  if (id === 'waste') return openWaste(ctx);
  if (id === 'numbers') return openKitchen();
  if (id === 'prepList') return openPrepList();
}

/// The four actions under an ingredient.
function act(what, id){
  const sup = find(id); if (!sup) return;
  if (what === 'prep') return openPrep(sup, ctx);
  return openMove(sup, what, ctx);
}

/// Link dishes to their own "sold as is" item (I0c): one tap, or a category.
async function asIs(ids, el){
  if (!ids.length) return;
  try {
    const r = await busy(el, () => post('/owner/stock/as-is', { products: ids }));
    toast(`${(r.linked || []).length} ${t('inv_linked')}`);
    ctx.reload();
  } catch (e) { toast(String(e.message || e)); }
}

/// A semi-finished product with its shelf (W-PF2 R2): the card's numbers
/// from `/owner/preps`, the level from `/owner/stock` (a batch cooked ahead).
function withShelf(id){
  const s = (stock?.supplies || []).find(x => x.id === id) || {};
  const p = preps.get(id) || s;
  return { ...p, onHand: s.onHand, available: s.available, counted: s.counted };
}

function drawList(host){
  const all = stock?.supplies || [];
  $('#invAlerts', host).innerHTML = alertsMarkup(all, noRecipe(), view.flag, t);
  const owner = !me().staff;
  // A semi-finished product is not stocked: its row is its card's numbers, not a shelf.
  const row = sel.on ? sup => pickRow(sup.id, sup.name || sup.id, esc(`${sup.category || ''} · ${P.isPrep(sup) ? t('kind_prep') : `${sup.onHand ?? 0} ${sup.unit || ''}`}`), sel.ids.has(sup.id))
    : sup => (P.isPrep(sup) ? prepRowMarkup(withShelf(sup.id), { money, t, del: owner }) : rowMarkup(sup, { money, t, warnDays: stock.expiryWarnDays, del: owner }));
  const q = norm(view.q).trim();
  let list = all.filter(s => (view.kind === 'all' || s.kind === view.kind) && (!q || norm(`${s.name} ${s.category} ${s.id} ${s.code || ''} ${s.barcode || ''}`).includes(q)) && matches(s, view.flag));
  if (view.sort === 'name') list = [...list].sort((a, b) => String(a.name).localeCompare(String(b.name), lang));
  else if (view.sort === 'category') list = [...list].sort((a, b) => String(a.category).localeCompare(String(b.category), lang) || String(a.name).localeCompare(String(b.name), lang));
  else list = [...list].sort((a, b) => (a.available || 0) - (b.available || 0));
  const counts = KINDS.map(([k]) => all.filter(s => s.kind === k).length);
  const nPrep = all.filter(P.isPrep).length;
  let html = '';
  if (view.sort === 'category') {
    const groups = [...new Set(list.map(s => s.category || ''))];
    html = groups.map(g => `<section class="group"><p class="eyebrow">${esc(g || t('none'))}</p><div class="rows">${list.filter(s => (s.category || '') === g).map(row).join('')}</div></section>`).join('');
  } else html = `<div class="rows">${list.map(row).join('')}</div>`;
  $('#stockList', host).innerHTML = (all.length ? `<p class="hint mono">${[...KINDS.map(([k], i) => counts[i] ? `${counts[i]} ${t('kind_' + k).toLowerCase()}` : ''), nPrep ? `${nPrep} ${t('pf_count')}` : ''].filter(Boolean).join(' · ')}</p>` : '') +
    (view.flag === 'noRecipe' ? '' : list.length ? html : empty('bento', { key: all.length ? 'none' : 'noStock', bodyKey: 'stockHint' })) +
    (sel.on ? barMarkup(sel) : '') +
    noRecipeMarkup(noRecipe(), t);
  retranslate($('#stockList', host)); hydrate($('#stockList', host));
  if (sel.on) bindBar($('#stockList', host), sel, { shown: () => list.map(s => s.id), redraw: () => (sel.on ? drawList(host) : rerender()),
    del: async (ids, el) => { const r = await deleteSupplies(ids, el, { names: ids.map(id => find(id)?.name || id) }); if (r) { sel.ids = new Set(); sel.on = false; ctx.reload(); } } });
  if (view.flag === 'noRecipe') $('#invNoRecipe', host)?.scrollIntoView({ block: 'start' });
}

/// THE OWNER'S WIPE (`POST /api/owner/ingredients/reset`): every ingredient,
/// recipe, allergen list and stock record goes, so the venue can fill them in
/// again. The owner types the venue's id to confirm; the server checks it too.
async function resetAll(el){
  const loc = withLoc().location_id;
  const typed = window.prompt(`${t('inv_resetAsk')}\n\n${loc}`);
  if (typed === null) return;
  if (typed.trim() !== loc) { toast(t('inv_resetMismatch')); return; }
  try {
    const r = await busy(el, () => post('/owner/ingredients/reset', withLoc({ confirm: loc })));
    toast(`${t('inv_resetDone')}: ${r.supplies} / ${r.recipes} / ${r.stockRecords}`);
    rerender();
  } catch (e) { toast(String(e.message || e)); }
}
