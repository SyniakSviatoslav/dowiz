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
import { $, $$, esc, icon, t, api, post, withLoc, toast, retranslate, hydrate, money, busy } from '/admin/core.js';
import { lang } from '/admin/i18n.js';
import { rerender } from '/admin/app.js';
import { openBulk } from '/admin/bulk.js';
import { ui, k as key, btn, iconBtn, select, chips, empty, loading, rowBtn, rowDiv } from '/admin/parts.js';
import * as C from '/admin/ingredients-calc.js';
import { openCard, openSupply, openMove, openPrep } from '/admin/ingredients.js';
import { KINDS, matches, alertsMarkup, rowMarkup, noRecipeMarkup } from '/admin/ingredients-view.js';
import { openCount, openWaste, openDelivery } from '/admin/ingredients-count.js';
import { openKitchen, ensureCss } from '/admin/kitchen-analytics.js';

export { KINDS };
export const UNITS = ['g', 'ml', 'unit'];
export const basisOf = C.basisOf;

/// The five things a kitchen does with its shelf, as tiles.
const TILES = [['delivery', 'package', 'inv_delivery'], ['count', 'check', 'inv_count'], ['prep', 'tools-kitchen-2', 'inv_prep'],
  ['waste', 'trash', 'inv_waste'], ['numbers', 'chart-bar', 'inv_numbers']];

let stock = null;
const view = { kind: 'all', q: '', sort: 'name', flag: '' };
const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

/// What every sheet opened from here needs: the answer, and how to refresh it.
const ctx = {
  get data(){ return stock; },
  async reload(){ rerender(); },
};

export async function render(host){
  ensureCss();
  host.innerHTML = `<div class="screen-h"><div><h1 data-t="inv_title"></h1></div>
    <div class="screen-acts">${iconBtn({ id: 'importSupplies', icon: 'download', ariaKey: 'importSupplies', tour: 'stock.import' })}${btn({ id: 'addSupply', variant: 'primary', icon: 'plus', key: 'addSupply', tour: 'stock.addSupply' })}</div></div>
    <p class="screen-hint" data-t="inv_hint"></p>
    <div class="tiles">${TILES.map(([id, ic, word]) => rowBtn({ cls: 'tile', leading: `<span class="tile-ic">${icon(ic)}</span>`, title: key(word), data: { tile: id } })).join('')}</div>
    <div id="invAlerts" class="inv-alerts"></div>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'sq', type: 'search', label: key('search'), placeholder: key('search'), attrs: { value: view.q, data: { tour: 'stock.search' } } })}</div>
    ${chips({ values: [{ value: 'all', key: 'all' }, ...KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic }))], value: view.kind, attr: 'k', labelKey: 'kind', tour: 'stock.filter' }).replace('class="chips"', 'class="chips filters"')}
    ${select({ id: 'sSort', ariaLabel: t('sort'), controlCls: 'sortsel', value: view.sort, options: ['name', 'category', 'low'].map(k => ({ value: k, key: 'ssort_' + k })), tour: 'stock.sort' })}
    <div id="stockList">${loading(2)}</div>`;
  retranslate(host);
  $('#addSupply', host).onclick = () => openSupply(null, ctx);
  $('#importSupplies', host).onclick = () => openBulk('supplies', rerender);
  $('#sSort', host).onchange = e => { view.sort = e.target.value; rerender(); };
  $('#sq', host).oninput = e => { view.q = e.target.value; drawList(host); };
  try { stock = await api('/owner/stock'); } catch (e) { $('#stockList', host).innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  drawList(host);
  host.onclick = e => {
    const tile = e.target.closest('[data-tile]'); if (tile) return openTile(tile.dataset.tile);
    const f = e.target.closest('[data-flag]'); if (f) { view.flag = view.flag === f.dataset.flag ? '' : f.dataset.flag; return drawList(host); }
    const k = e.target.closest('[data-k]'); if (k) { view.kind = k.dataset.k; return rerender(); }
    const a = e.target.closest('[data-act]'); if (a) return act(a.dataset.act, a.dataset.s);
    const r = e.target.closest('[data-s]'); if (r) return openCard(find(r.dataset.s), ctx);
    const one = e.target.closest('[data-asis]'); if (one) return asIs([one.dataset.asis], one);
    const all = e.target.closest('[data-asiscat]'); if (all) return asIs(noRecipe().filter(d => (d.categoryId || '') === all.dataset.asiscat).map(d => d.id), all);
  };
}

const find = id => (stock?.supplies || []).find(s => s.id === id);
const noRecipe = () => stock?.noRecipe || [];

function openTile(id){
  if (id === 'delivery') return openDelivery(ctx);
  if (id === 'count') return openCount(ctx);
  if (id === 'prep') return openPrep(null, ctx);
  if (id === 'waste') return openWaste(ctx);
  if (id === 'numbers') return openKitchen();
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
    const r = await busy(el, () => post('/owner/stock/as-is', withLoc({ products: ids })));
    toast(`${(r.linked || []).length} ${t('inv_linked')}`);
    ctx.reload();
  } catch (e) { toast(String(e.message || e)); }
}

function drawList(host){
  const all = stock?.supplies || [];
  $('#invAlerts', host).innerHTML = alertsMarkup(all, noRecipe(), view.flag, t);
  const row = sup => rowMarkup(sup, { money, t, warnDays: stock.expiryWarnDays });
  const q = norm(view.q).trim();
  let list = all.filter(s => (view.kind === 'all' || s.kind === view.kind) && (!q || norm(`${s.name} ${s.category} ${s.id}`).includes(q)) && matches(s, view.flag));
  if (view.sort === 'name') list = [...list].sort((a, b) => String(a.name).localeCompare(String(b.name), lang));
  else if (view.sort === 'category') list = [...list].sort((a, b) => String(a.category).localeCompare(String(b.category), lang) || String(a.name).localeCompare(String(b.name), lang));
  else list = [...list].sort((a, b) => (a.available || 0) - (b.available || 0));
  const counts = KINDS.map(([k]) => all.filter(s => s.kind === k).length);
  let html = '';
  if (view.sort === 'category') {
    const groups = [...new Set(list.map(s => s.category || ''))];
    html = groups.map(g => `<section class="group"><p class="eyebrow">${esc(g || t('none'))}</p><div class="rows">${list.filter(s => (s.category || '') === g).map(row).join('')}</div></section>`).join('');
  } else html = `<div class="rows">${list.map(row).join('')}</div>`;
  $('#stockList', host).innerHTML = (all.length ? `<p class="hint mono">${KINDS.map(([k], i) => counts[i] ? `${counts[i]} ${t('kind_' + k).toLowerCase()}` : '').filter(Boolean).join(' · ')}</p>` : '') +
    (view.flag === 'noRecipe' ? '' : list.length ? html : empty('bento', { key: all.length ? 'none' : 'noStock', bodyKey: 'stockHint' })) +
    noRecipeMarkup(noRecipe(), t) +
    (stock.stranded?.length ? `<section class="group mt-3"><p class="eyebrow" data-t="stranded"></p><p class="muted small" data-t="strandedHint"></p><div class="rows">${stock.stranded.map(s => rowDiv({ cls: 'off', leading: icon('alert-triangle'), title: `${s.item} × ${s.qty}`, sub: `<span class="mono">#${esc(String(s.order).slice(0, 8))}</span>` })).join('')}</div></section>` : '');
  retranslate($('#stockList', host)); hydrate($('#stockList', host));
  if (view.flag === 'noRecipe') $('#invNoRecipe', host)?.scrollIntoView({ block: 'start' });
}
