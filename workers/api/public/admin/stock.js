// The shelf -- what the kitchen has, as a fold over what happened to it.
//
// A level is never a number somebody edits: it is received, wasted, counted,
// and reserved or consumed by orders. The screen shows each supply with its
// available quantity and a bar against its low mark, and a tap opens the
// three honest movements. A supply the venue tracks is a supply an order is
// refused for when it runs out -- which is the whole point.

import { $, $$, esc, icon, t, api, post, withLoc, toast, sheet, closeSheet, busy, money, switchEl, confirm, retranslate, hydrate } from '/admin/core.js';
import { lang } from '/admin/i18n.js';
import { rerender } from '/admin/app.js';
import { openBulk } from '/admin/bulk.js';
import { ui, k as key, btn, iconBtn, field, select, chips, press, pill, empty, loading, rowBtn, rowDiv } from '/admin/parts.js';

/// The bar reads full at this many times the low mark.
const FULL_AT_LOW_MULTIPLE = 4;
/// The movements an owner can make by hand.
const MOVES = [['received', 'received', 'download'], ['wasted', 'wasted', 'trash'], ['stocktake', 'counted', 'check']];
/// The reasons a kitchen writes off stock, as the hub's vocabulary spells them.
const WASTE_REASONS = ['spoiled', 'dropped', 'unsold', 'returned', 'staff_meal'];

let stock = null;

/// The four kinds a supply can be, with the old console's icons.
export const KINDS = [['food_ingredient', 'meat'], ['condiment', 'bottle'], ['packaging', 'box'], ['utensil', 'tool']];
export const UNITS = ['g', 'ml', 'unit'];
const isFood = k => k === 'food_ingredient' || k === 'condiment';
/// Nutrition and cost are per 100 g/ml, per ONE piece.
export const basisOf = u => u === 'unit' ? 1 : 100;
const view = { kind: 'all', q: '', sort: 'name' };
const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabStock"></p><h1 data-t="supplies"></h1></div>
    <div class="btn-row compact">${iconBtn({ id: 'importSupplies', icon: 'download', ariaKey: 'importSupplies', tour: 'stock.import' })}${btn({ id: 'addSupply', variant: 'primary', icon: 'plus', key: 'addSupply', tour: 'stock.addSupply' })}</div></div>
    <p class="screen-hint" data-t="stockScreenHint"></p>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'sq', type: 'search', label: key('search'), placeholder: key('search'), attrs: { value: view.q, data: { tour: 'stock.search' } } })}</div>
    ${chips({ values: [{ value: 'all', key: 'all' }, ...KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic }))], value: view.kind, attr: 'k', labelKey: 'kind', tour: 'stock.filter' }).replace('class="chips"', 'class="chips filters"')}
    ${select({ id: 'sSort', ariaLabel: t('sort'), controlCls: 'sortsel', value: view.sort, options: ['name', 'category', 'low'].map(k => ({ value: k, key: 'ssort_' + k })), tour: 'stock.sort' })}
    <div id="stockList">${loading(2)}</div>`;
  $('#addSupply', host).onclick = () => openSupply(null);
  $('#importSupplies', host).onclick = () => openBulk('supplies', rerender);
  $('#sSort', host).onchange = e => { view.sort = e.target.value; rerender(); };
  $('#sq', host).oninput = e => { view.q = e.target.value; drawList(host); };
  try { stock = await api('/owner/stock'); } catch (e) { $('#stockList', host).innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  drawList(host);
  host.onclick = e => {
    const k = e.target.closest('[data-k]'); if (k) { view.kind = k.dataset.k; return rerender(); }
    const r = e.target.closest('[data-s]'); if (r) openMove(r.dataset.s);
  };
}

function drawList(host){
  const all = stock?.supplies || [];
  const lowN = all.filter(s => s.low || (s.available || 0) <= 0).length;
  const h1 = $('.screen-h', host); if (h1 && lowN && !$('.lowN', h1)) h1.insertAdjacentHTML('beforeend', pill('warn', { label: `${lowN} ${t('low')}`, cls: 'lowN' }));
  const q = norm(view.q).trim();
  let list = all.filter(s => (view.kind === 'all' || s.kind === view.kind) && (!q || norm(`${s.name} ${s.category} ${s.id}`).includes(q)));
  if (view.sort === 'name') list = [...list].sort((a, b) => String(a.name).localeCompare(String(b.name), lang));
  else if (view.sort === 'category') list = [...list].sort((a, b) => String(a.category).localeCompare(String(b.category), lang) || String(a.name).localeCompare(String(b.name), lang));
  else list = [...list].sort((a, b) => Number(!!b.low) - Number(!!a.low) || (a.available || 0) - (b.available || 0));
  const counts = KINDS.map(([k]) => all.filter(s => s.kind === k).length);
  const row = sup => {
    const low = sup.lowAt || 0;
    const pct = low ? Math.min(100, Math.round(100 * (sup.available || 0) / (low * FULL_AT_LOW_MULTIPLE))) : 100;
    const state = (sup.available || 0) <= 0 ? 'bad' : sup.low ? 'warn' : 'ok';
    const ic = (KINDS.find(([k]) => k === sup.kind) || KINDS[0])[1];
    const per = `/${basisOf(sup.unit) === 1 ? '' : '100'}${sup.unit}`;
    const facts = [sup.category, isFood(sup.kind) && sup.kcalPer100 != null ? `${sup.kcalPer100} kcal${per}` : '', sup.costPerBasis != null ? `${money(sup.costPerBasis)}${per}` : ''].filter(Boolean).join(' · ');
    return rowBtn({ leading: icon(ic), title: sup.name || sup.id, data: { s: sup.id }, tour: 'stock.supply',
      sub: `<span class="mono">${esc(facts)}</span><span class="mono">${sup.available ?? 0} ${esc(sup.unit || '')}${sup.reserved ? ` · ${esc(t('reserved'))} ${sup.reserved}` : ''}${low ? ` · ${esc(t('minLevel'))} ${low}` : ''}</span>
        <span class="gauge"><i class="${state === 'ok' ? '' : state}" data-w="${pct}"></i></span>`,
      trailing: `${isFood(sup.kind) && !sup.nutritionConfirmed ? pill('warn', { key: 'unconfirmed' }) : ''}${pill(state, { key: state === 'bad' ? 'out' : state === 'warn' ? 'low' : 'onSale' })}` }); };
  let html = '';
  if (view.sort === 'category') {
    const groups = [...new Set(list.map(s => s.category || ''))];
    html = groups.map(g => `<section class="group"><p class="eyebrow">${esc(g || t('none'))}</p><div class="rows">${list.filter(s => (s.category || '') === g).map(row).join('')}</div></section>`).join('');
  } else html = `<div class="rows">${list.map(row).join('')}</div>`;
  $('#stockList', host).innerHTML = (all.length ? `<p class="hint mono">${KINDS.map(([k], i) => `${counts[i]} ${t('kind_' + k).toLowerCase()}`).join(' · ')}</p>` : '') +
    (list.length ? html : empty('bento', { key: all.length ? 'none' : 'noStock', bodyKey: 'stockHint' })) +
    (stock.stranded?.length ? `<section class="group mt-3"><p class="eyebrow" data-t="stranded"></p><p class="muted small" data-t="strandedHint"></p><div class="rows">${stock.stranded.map(s => rowDiv({ cls: 'off', leading: icon('alert-triangle'), title: `${s.item} × ${s.qty}`, sub: `<span class="mono">#${esc(String(s.order).slice(0, 8))}</span>` })).join('')}</div></section>` : '');
  retranslate($('#stockList', host)); hydrate($('#stockList', host));
}

/// The supply as the old service knew it, plus a cost and a weight per piece.
function openSupply(sup){
  const kind = sup?.kind || KINDS[0][0], unit = sup?.unit || 'g';
  const cats = [...new Set((stock?.supplies || []).map(s => s.category).filter(Boolean))];
  sheet(`<p class="eyebrow" data-t="supplies"></p><h2 data-t="${sup ? 'edit' : 'addSupply'}"></h2>
    ${field({ id: 's-name', key: 'name', value: sup?.name || '', autocomplete: 'off', tour: 'supply.name' })}
    ${field({ id: 's-id', key: 'ingredient', value: sup?.id || '', placeholder: 'salmon', hintKey: 'supplyIdHint', attrs: { readonly: !!sup }, tour: 'supply.id' })}
    <p class="ui-label" data-t="kind"></p>${chips({ id: 'sKind', values: KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic })), value: kind, attr: 'kind', labelKey: 'kind', tour: 'supply.kind' })}
    <div class="grid2"><div>${field({ id: 's-cat', key: 'category', value: sup?.category || '', autocomplete: 'off', attrs: { list: 'catList' }, tour: 'supply.category' })}<datalist id="catList">${cats.map(c => `<option value="${esc(c)}">`).join('')}</datalist></div>
      <div>${select({ id: 's-unit', key: 'unit', value: unit, options: UNITS.map(u => ({ value: u, label: u })), tour: 'supply.unit' })}</div></div>
    <div id="sFood" ${isFood(kind) ? '' : 'hidden'}>
      <p class="eyebrow mt-3"><span data-t="nutritionPer"></span> <span id="sBasis">${basisOf(unit) === 1 ? '1' : '100'} ${esc(unit)}</span></p>
      <div class="grid2"><div>${field({ id: 's-kcal', key: 'kcal', inputmode: 'decimal', value: sup?.kcalPer100 ?? '', tour: 'supply.kcal' })}</div><div>${field({ id: 's-prot', key: 'protein', inputmode: 'decimal', value: sup?.proteinPer100 ?? '' })}</div></div>
      <div class="grid2"><div>${field({ id: 's-fat', key: 'fat', inputmode: 'decimal', value: sup?.fatPer100 ?? '' })}</div><div>${field({ id: 's-carb', key: 'carbs', inputmode: 'decimal', value: sup?.carbsPer100 ?? '' })}</div></div>
      ${switchEl('s-conf', !!sup?.nutritionConfirmed, 'nutritionConfirmed', 'nutritionConfirmedHint', 'supply.confirmed')}
    </div>
    <div class="grid2"><div>${field({ id: 's-cost', key: 'costPer', hint: `${basisOf(unit) === 1 ? '1' : '100'} ${unit}`, inputmode: 'numeric', value: sup?.costPerBasis ?? '', tour: 'supply.cost' })}</div>
      <div id="sWeight" ${unit === 'unit' ? '' : 'hidden'}>${field({ id: 's-wpu', key: 'weightPerUnit', inputmode: 'decimal', value: sup?.weightPerUnit ?? '' })}</div></div>
    ${field({ id: 's-low', key: 'minLevel', inputmode: 'numeric', value: sup?.lowAt ?? '', tour: 'supply.low' })}
    <div class="btn-row">${btn({ id: 'sSave', variant: 'primary', icon: 'check', key: 'save', tour: 'supply.save' })}</div>
    ${sup ? `<div class="btn-row">${btn({ id: 'sRetire', variant: 'danger', icon: 'trash', key: 'retireSupply', tour: 'supply.retire' })}</div>` : ''}`, { name: 'supply' });
  let k = kind;
  for (const b of $$('[data-kind]', $('#sheetIn'))) b.onclick = () => { k = b.dataset.kind; press($$('[data-kind]', $('#sheetIn')), b); $('#sFood').hidden = !isFood(k); };
  $('#s-unit').onchange = e => { const u = e.target.value; $('#sWeight').hidden = u !== 'unit'; for (const el of $$('#sBasis, #s-cost-hint')) el.textContent = `${basisOf(u) === 1 ? '1' : '100'} ${u}`; };
  if (!sup) $('#s-name').oninput = e => { $('#s-id').value = e.target.value.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '').replace(/[^a-z0-9а-яіїєґ]+/gi, '-').replace(/^-|-$/g, ''); };
  const num = v => { const s = String(v ?? '').trim(); if (!s) return null; const n = Number(s.replace(',', '.')); return Number.isFinite(n) ? n : null; };
  $('#sSave').onclick = async () => {
    const id = $('#s-id').value.trim().toLowerCase().replace(/\s+/g, '-'); if (!id) return toast(t('required'));
    const body = { id, name: $('#s-name').value.trim() || id, unit: $('#s-unit').value, kind: k, category: $('#s-cat').value.trim() };
    const low = num($('#s-low').value); if (low != null) body.lowAt = Math.round(low);
    if (isFood(k)) { for (const [f, key] of [['s-kcal', 'kcalPer100'], ['s-prot', 'proteinPer100'], ['s-fat', 'fatPer100'], ['s-carb', 'carbsPer100']]) { const v = num($('#' + f).value); if (v != null) body[key] = v; } body.nutritionConfirmed = $('#s-conf').checked; }
    const cost = num($('#s-cost').value); if (cost != null) body.costPerBasis = Math.round(cost);
    const wpu = num($('#s-wpu').value); if (wpu != null) body.weightPerUnit = wpu;
    try { await busy($('#sSave'), () => post('/owner/supplies', body)); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
  const ret = $('#sRetire'); if (ret) ret.onclick = async () => { const ok = await confirm(t('retireSupply'), t('retireSupplyHint'), { danger: true }); if (!ok) return openSupply(sup); try { await post(`/owner/supplies/${encodeURIComponent(sup.id)}/retire`, withLoc()); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); } };
}

function openMove(id){
  const sup = (stock?.supplies || []).find(s => s.id === id); if (!sup) return;
  sheet(`<p class="eyebrow" data-t="move"></p><h2>${esc(sup.name || sup.id)}</h2>
    <p class="mono muted">${sup.available ?? 0} ${esc(sup.unit || '')}</p>
    <div class="chips" id="kind" role="group">${MOVES.map(([k, word, ic], i) => ui.chip({ as: 'button', selected: i === 0, icon: ic, label: key(word), attrs: { data: { k, tour: 'stock.' + k } } })).join('')}</div>
    ${field({ id: 'm-qty', key: 'level', inputmode: 'decimal', tour: 'move.qty' })}
    <div id="mReasons" hidden><p class="ui-label" data-t="reason"></p>${chips({ values: WASTE_REASONS.map(r => ({ value: r, key: r })), attr: 'r', labelKey: 'reason', tour: 'move.wasteReason' })}</div>
    ${field({ id: 'm-reason', key: 'reason', tour: 'move.reason' })}
    <div class="btn-row">${btn({ id: 'mEdit', variant: 'ghost', icon: 'tools-kitchen-2', key: 'edit', tour: 'move.edit' })}${btn({ id: 'mGo', variant: 'primary', icon: 'check', key: 'save', tour: 'move.save' })}</div>`, { name: 'move' });
  let kind = MOVES[0][0];
  for (const b of $$('[data-k]', $('#sheetIn'))) b.onclick = () => { kind = b.dataset.k; press($$('[data-k]', $('#sheetIn')), b); $('#mReasons').hidden = kind !== 'wasted'; };
  for (const b of $$('[data-r]', $('#sheetIn'))) b.onclick = () => { $('#m-reason').value = b.dataset.r; press($$('[data-r]', $('#sheetIn')), b); };
  $('#mEdit').onclick = () => openSupply(sup);
  $('#mGo').onclick = async () => {
    const qty = Number($('#m-qty').value); if (!Number.isFinite(qty)) return toast(t('required'));
    const body = kind === 'stocktake' ? { item: id, observed: qty } : { item: id, qty };
    const reason = $('#m-reason').value.trim(); if (reason) body.reason = reason;
    try { await busy($('#mGo'), () => post(`/owner/stock/${kind}`, body)); toast(t('saved')); closeSheet(); rerender(); } catch (e) { toast(String(e.message || e)); }
  };
}
