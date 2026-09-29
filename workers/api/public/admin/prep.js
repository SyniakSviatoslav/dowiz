// SEMI-FINISHED PRODUCTS ON SCREEN (lane W-PF, 2026-09-29): the card, the
// editor, and "one sale takes off the shelf". Every call is one of:
//   GET  /api/owner/preps                the list, hydrated, with where used
//   POST /api/owner/preps                add or edit a card
//   GET  /api/owner/products/:id/takes   one sale's raw leaves
//   POST /api/owner/supplies/:id/retire  off the list, reversible
// (deleting, and the where-used question before it, is nom.js's).
//
// "максимально просто": one picker over raw items AND other semi-finished
// products, a quantity per line in the item's own unit, one yield field, and
// K / batch cost / cost per kg drawn live as the fields change.
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import './prep-i18n.js';
import { $, $$, esc, icon, t, api, post, withLoc, toast, sheet, closeSheet, busy, money, retranslate } from '/admin/core.js';
import { ui, k as key, btn, field, input, chips, press } from '/admin/parts.js';
import { me } from '/admin/app.js';
import * as P from '/admin/prep-logic.js';
import { cardMarkup, editorLine, liveMarkup, takesMarkup, PREP_ICON } from '/admin/prep-view.js';
import { deleteSupplies } from '/admin/nom.js';
import { cookForm, cookLive, cookResult } from '/admin/pf2-view.js';
import './pf2-i18n.js';

const fail = e => toast(String(e.message || e));

/// The styles of the editor's lines, once.
export function ensurePrepCss(){
  if (document.getElementById('prepCss')) return;
  const l = document.createElement('link');
  l.id = 'prepCss'; l.rel = 'stylesheet'; l.href = '/admin/prep.css';
  document.head.append(l);
}

/// Every semi-finished product, hydrated.
export async function loadPreps(){
  try { return (await api('/owner/preps')).preps || []; } catch (e) { fail(e); return []; }
}

/// The card of one ПФ.
export function openPrepCard(p, ctx){
  if (!p) return;
  sheet(cardMarkup(p, { money, t, owner: !me().staff }), { name: 'prepCard' });
  $('#pfEdit').onclick = () => openPrepEditor(p, ctx);
  $('#pfCook').onclick = () => openCook(p, ctx);
  const del = $('#pfDelete'); if (del) del.onclick = async () => { const r = await deleteSupplies([p.id], del, { names: [p.name || p.id] }); if (r) { closeSheet(); ctx.reload(); } };
  $('#pfRetire').onclick = async () => { try { await post(`/owner/supplies/${encodeURIComponent(p.id)}/retire`, withLoc()); toast(t('saved')); closeSheet(); ctx.reload(); } catch (e) { fail(e); } };
  for (const b of $$('[data-takes]', $('#sheetIn'))) b.onclick = () => openTakes(b.dataset.takes, b.dataset.tname, () => openPrepCard(p, ctx));
}

/// THE PRODUCTION ACT (W-PF2 R2): a batch of `p` cooked ahead. The raw items
/// leave the shelf by the card; the batch lands weighed; the loss is shown.
export function openCook(p, ctx){
  sheet(cookForm(p, { field, input }), { name: 'prepCook' });
  const live = () => {
    const l = cookLive($('#ck-qty').value, $('#ck-out').value, p.grossG, p.yield);
    $('#ckLive').textContent = l ? `${t('pf_cookLoss')}: ${l.loss} g · ${(l.pm / 10).toFixed(1)}%` : '';
  };
  $('#ck-qty').oninput = live; $('#ck-out').oninput = live; live();
  $('#ckSave').onclick = async () => {
    const qty = Math.round(Number($('#ck-qty').value)), out = $('#ck-out').value.trim();
    if (!(qty > 0)) return toast(t('pf_yieldBad'));
    const b = { item: p.id, qty, ...(out ? { out: Math.round(Number(out)) } : {}), ...($('#ck-exp').value ? { expiry: $('#ck-exp').value } : {}) };
    try {
      const r = await busy($('#ckSave'), () => post('/owner/stock/cooked', withLoc(b)));
      $('#ckOut').innerHTML = cookResult(r, { money, t }); $('#ckSave').disabled = true; ctx.reload();
    } catch (e) { fail(e); }
  };
}

/// What one sale of a dish takes off the shelf. `back` reopens what was under it.
export async function openTakes(dishId, name, back = null){
  sheet(`<p class="eyebrow" data-t="pf_takes"></p><h2>${esc(name || dishId)}</h2><div id="tkOut"><p class="hint" data-t="loading"></p></div>
    ${back ? `<div class="btn-row">${btn({ id: 'tkBack', variant: 'ghost', icon: 'arrow-left', key: 'back' })}</div>` : ''}`, { name: 'takes' });
  const b = $('#tkBack'); if (b) b.onclick = back;
  try {
    const tk = await api(`/owner/products/${encodeURIComponent(dishId)}/takes`);
    $('#tkOut').innerHTML = takesMarkup(tk, { money, t });
  } catch (e) { $('#tkOut').innerHTML = `<p class="hint">${esc(String(e.message || e))}</p>`; }
  retranslate($('#tkOut'));
}

/// The editor's state: what is typed, and the lines with their supplies.
function stateOf(p, all){
  const byId = id => all.find(s => s.id === id) || { id, name: id, unit: 'g' };
  return { fresh: !p, id: p?.id || '', name: p?.name || '', category: p?.category || '', unit: p?.unit || 'g',
    yield: p?.yield ?? '', weightPerUnit: p?.weightPerUnit ?? '',
    lines: (p?.lines || []).map(l => ({ item: l.item, qty: l.qty, sup: { ...byId(l.item), name: l.name || byId(l.item).name, unit: l.unit || byId(l.item).unit, kind: l.kind || byId(l.item).kind } })) };
}

/// The editor: a new card, or `p`'s. `st` carries the typed state back
/// from the picker, which is a sheet of its own.
export function openPrepEditor(p, ctx, st = null){
  ensurePrepCss();
  const all = ctx.data?.supplies || [];
  st = st || stateOf(p, all);
  const cats = [...new Set(all.map(s => s.category).filter(Boolean))];
  sheet(`<p class="eyebrow" data-t="pf_title"></p><h2 data-t="${st.fresh ? 'pf_add' : 'pf_edit'}"></h2><p class="sheet-hint" data-t="pf_hint"></p>
    ${field({ id: 'pf-name', key: 'name', value: st.name, autocomplete: 'off', tour: 'pf.name' })}
    ${field({ id: 'pf-id', key: 'ingredient', value: st.id, placeholder: 'rice-seasoned', hintKey: 'supplyIdHint', attrs: { readonly: !st.fresh } })}
    <div class="grid2"><div>${field({ id: 'pf-cat', key: 'category', value: st.category, autocomplete: 'off', attrs: { list: 'pfCats' } })}<datalist id="pfCats">${cats.map(c => `<option value="${esc(c)}">`).join('')}</datalist></div>
      <div><p class="ui-label" data-t="unit"></p>${chips({ id: 'pfUnit', values: ['g', 'ml', 'unit'].map(u => (u === 'unit' ? { value: u, key: 'nom_pcs' } : { value: u, label: u })), value: st.unit, attr: 'pu' })}</div></div>
    <p class="eyebrow mt-3" data-t="pf_lines"></p><div id="pfLines"></div>
    <div class="btn-row">${btn({ id: 'pfAddLine', icon: 'plus', key: 'pf_addLine', tour: 'pf.addLine' })}</div>
    <div class="inv-big">${field({ id: 'pf-yield', key: 'pf_yield', hintKey: 'pf_yieldHint', inputmode: 'decimal', autocomplete: 'off', value: st.yield, tour: 'pf.yield' })}</div>
    <div id="pfWpu" ${st.unit === 'unit' ? '' : 'hidden'}>${field({ id: 'pf-wpu', key: 'weightPerUnit', inputmode: 'decimal', value: st.weightPerUnit })}</div>
    <p class="mono" id="pfLive"></p><p class="hint" data-t="pf_kHint"></p>
    <div class="btn-row">${btn({ id: 'pfSave', variant: 'primary', icon: 'check', key: 'save', tour: 'pf.save' })}</div>`, { name: 'prepEdit' });
  const typed = () => { st.name = $('#pf-name').value; st.id = $('#pf-id').value; st.category = $('#pf-cat').value; st.yield = $('#pf-yield').value; st.weightPerUnit = $('#pf-wpu').value; return st; };
  const live = () => {
    const y = P.readYield($('#pf-yield').value, st.unit), wpu = Number($('#pf-wpu').value) || null;
    const batch = P.batchCost(st.lines), k = P.kPm(st.lines, y, st.unit, wpu);
    $('#pfLive').innerHTML = liveMarkup({ k, batch, per: P.costPer(batch, y, st.unit), unit: st.unit }, { money, t });
    st.lines.forEach((l, i) => { const c = P.lineCost(l.sup, l.qty); const el = $(`[data-plc="${i}"]`); if (el) el.textContent = c != null && !l.sup?.untracked ? `${t('pf_lineCost')} ${money(c)}` : ''; });
  };
  const draw = () => {
    $('#pfLines').innerHTML = st.lines.length ? st.lines.map(editorLine).join('') : `<p class="hint" data-t="pf_noLines"></p>`;
    retranslate($('#pfLines'));
    for (const inp of $$('[data-plq]', $('#pfLines'))) inp.oninput = () => { const l = st.lines[+inp.dataset.plq]; const r = P.readLines([{ item: l.item, qty: inp.value, unit: l.sup?.unit || 'g' }]); if (r.lines) l.qty = r.lines[0].qty; live(); };
    for (const b of $$('[data-plx]', $('#pfLines'))) b.onclick = () => { st.lines.splice(+b.dataset.plx, 1); draw(); };
    live();
  };
  for (const b of $$('[data-pu]', $('#sheetIn'))) b.onclick = () => { st.unit = b.dataset.pu; press($$('[data-pu]', $('#sheetIn')), b); $('#pfWpu').hidden = st.unit !== 'unit'; live(); };
  if (st.fresh) $('#pf-name').oninput = e => { $('#pf-id').value = P.idOf(e.target.value); };
  $('#pf-yield').oninput = live; $('#pf-wpu').oninput = live;
  $('#pfAddLine').onclick = () => openPicker(all, typed(), s => { st.lines.push({ item: s.id, qty: s.unit === 'unit' ? 1 : 100, sup: s }); openPrepEditor(p, ctx, st); });
  $('#pfSave').onclick = async () => {
    typed();
    const id = st.id.trim().toLowerCase().replace(/\s+/g, '-'); if (!id) return toast(t('required'));
    const read = P.readLines(st.lines.map(l => ({ item: l.item, qty: String(l.qty), unit: l.sup?.unit || 'g' })));
    if (read.error) return toast(t(read.error));
    const y = P.readYield(st.yield, st.unit); if (y == null) return toast(t('pf_yieldBad'));
    const b = P.body({ id, name: st.name.trim() || id, unit: st.unit, category: st.category.trim(), lines: read.lines, yield: y, weightPerUnit: Number(st.weightPerUnit) || null });
    try {
      const r = await busy($('#pfSave'), () => post('/owner/preps', withLoc(b)));
      toast((r.dishes || []).length ? `${t('pf_saved')}: ${r.dishes.length}` : t('saved'));
      closeSheet(); ctx.reload();
    } catch (e) { fail(e); }
  };
  draw();
}

/// Choose a raw item or another semi-finished product for a line.
function openPicker(all, st, then){
  const on = new Set(st.lines.map(l => l.item));
  sheet(`<p class="eyebrow" data-t="pf_lines"></p><h2 data-t="pf_pick"></h2>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'pkQ', type: 'search', label: key('search'), placeholder: key('search'), attrs: {} })}</div>
    <div class="rows" id="pkList"></div>`, { name: 'prepPick' });
  const draw = () => {
    const list = P.pickable(all, $('#pkQ').value, on, st.id.trim()).slice(0, 80);
    $('#pkList').innerHTML = list.map(s => `<button type="button" class="ui-row ui-row--action" data-pk="${esc(s.id)}"><span class="ui-row-lead">${icon(P.isPrep(s) ? PREP_ICON : 'meat')}</span><span class="ui-row-body"><span class="ui-row-title ui-row-title--strong">${esc(s.name)}</span><span class="ui-row-sub">${esc(`${s.category || ''} · ${s.unit}`)}</span></span></button>`).join('');
  };
  draw();
  $('#pkQ').oninput = draw;
  $('#pkList').onclick = e => { const r = e.target.closest('[data-pk]'); if (r) then(all.find(s => s.id === r.dataset.pk)); };
}
