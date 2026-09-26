// THE THREE SHEETS THAT TOUCH MANY INGREDIENTS AT ONCE: a count of the whole
// shelf (one stocktake session, one decision), a delivery with every line of
// its invoice, and the write-offs with what they cost.
//
//   count     POST /api/owner/stock/count     {lines:[{item, observed}]}
//   delivery  POST /api/owner/stock/received  one per line, the paper on each
//   waste     GET  /api/owner/stock/waste     every binned thing, valued
//
// Whole grams throughout (1,5 kg is read as 1500 g); money in minor units.

import { $, $$, esc, t, api, post, toast, sheet, busy, money, day, clock, icon, retranslate, hydrate } from '/admin/core.js';
import { ui, k as key, btn, field, input, select, chips, press, empty, loading, rowDiv } from '/admin/parts.js';
import * as C from '/admin/ingredients-calc.js';
import { openMove, pickSupply } from '/admin/ingredients.js';

const fail = e => toast(String(e.message || e));
const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '-');
const nameOf = (ctx, id) => (ctx.data?.supplies || []).find(s => s.id === id)?.name || id;
const byShelf = list => list.slice().sort((a, b) => String(a.category || '').localeCompare(String(b.category || '')) || String(a.name).localeCompare(String(b.name)));

/// The whole shelf, counted: every line one decision, one session id.
export function openCount(ctx){
  const all = byShelf(ctx.data?.supplies || []);
  const sessions = (ctx.data?.sessions || []).map(s => `<tr><td>${esc(when(s.at))}</td><td>${s.lines}</td><td class="${s.value < 0 ? 'neg' : ''}">${money(s.value)}</td></tr>`).join('');
  sheet(`<p class="eyebrow" data-t="inv_count"></p><h2 data-t="inv_count"></h2><p class="hint" data-t="inv_countHint"></p>
    <div class="srch">${icon('search')}${ui.inputRow({ id: 'cQ', type: 'search', label: key('search'), placeholder: key('search'), attrs: {} })}</div>
    <div id="cList">${all.map(s => `<div class="inv-count" data-row="${esc(s.id)}"><span class="t"><b>${esc(s.name)}</b>
      <small><span data-t="inv_onHand"></span> ${s.onHand ?? 0} ${esc(s.unit)} · ${esc(s.category || '')} <span data-drift="${esc(s.id)}"></span></small></span>
      ${ui.inputRow({ label: s.name, attrs: { placeholder: String(s.onHand ?? 0), inputmode: 'decimal', autocomplete: 'off', data: { ci: s.id } } })}</div>`).join('')}</div>
    <div class="btn-row">${btn({ id: 'cGo', variant: 'primary', icon: 'check', key: 'save' })}</div>
    <div id="cOut"></div>
    ${sessions ? `<p class="eyebrow mt-3" data-t="inv_sessions"></p><div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_day"></th><th data-t="inv_lines"></th><th data-t="inv_value"></th></tr></thead><tbody>${sessions}</tbody></table></div>` : ''}`, { name: 'count' });
  const sup = id => all.find(s => s.id === id);
  $('#cQ').oninput = () => { const q = $('#cQ').value.toLowerCase().trim(); for (const r of $$('[data-row]', $('#cList'))) { const s = sup(r.dataset.row); r.hidden = !!q && !`${s.name} ${s.category}`.toLowerCase().includes(q); } };
  for (const inp of $$('[data-ci]', $('#cList'))) inp.oninput = () => {
    const s = sup(inp.dataset.ci), v = C.amount(inp.value, s.unit), el = $(`[data-drift="${CSS.escape(s.id)}"]`);
    el.textContent = v == null ? '' : `· ${t('inv_drift')} ${v - (s.onHand ?? 0) > 0 ? '+' : ''}${v - (s.onHand ?? 0)}`;
  };
  $('#cGo').onclick = async () => {
    const lines = [];
    for (const inp of $$('[data-ci]', $('#cList'))) {
      if (!inp.value.trim()) continue;
      const v = C.amount(inp.value, sup(inp.dataset.ci).unit);
      if (v == null || v < 0) { inp.focus(); return toast(t('required')); }
      lines.push({ item: inp.dataset.ci, observed: v });
    }
    if (!lines.length) return toast(t('required'));
    try {
      const r = await busy($('#cGo'), () => post('/owner/stock/count', { lines }));
      $('#cOut').innerHTML = `<p class="ok">${esc(t('inv_countSaved'))} · ${lines.length} ${esc(t('inv_lines'))} · ${money(r.value || 0)}</p>
        <div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_ingredient"></th><th data-t="inv_expected"></th><th data-t="inv_counted"></th><th data-t="inv_drift"></th><th data-t="inv_value"></th></tr></thead>
        <tbody>${(r.lines || []).map(l => `<tr><td>${esc(nameOf(ctx, l.item))}</td><td>${l.expected}</td><td>${l.observed}</td><td class="${l.drift < 0 ? 'neg' : l.drift > 0 ? 'pos' : ''}">${l.drift > 0 ? '+' : ''}${l.drift}</td><td>${l.value != null ? money(l.value) : '-'}</td></tr>`).join('')}</tbody></table></div>`;
      retranslate($('#cOut'));
      ctx.reload();
    } catch (e) { fail(e); }
  };
}

/// A delivery with every line of its paper: one supplier, one invoice number.
export function openDelivery(ctx){
  const all = byShelf(ctx.data?.supplies || []);
  const sups = ctx.data?.suppliers || [];
  const line = i => `<div class="inv-line" data-dl="${i}">${select({ key: 'ka_ingredient', value: '', options: [{ value: '', key: 'inv_pick' }, ...all.map(s => ({ value: s.id, label: `${s.name} (${s.unit})` }))], attrs: { data: { dls: i } } })}
      ${field({ key: 'inv_qty', inputmode: 'decimal', autocomplete: 'off', attrs: { data: { dlq: i } } })}
      ${field({ key: 'inv_price', inputmode: 'numeric', autocomplete: 'off', attrs: { data: { dlp: i } } })}
      ${field({ key: 'inv_lot', autocomplete: 'off', attrs: { data: { dll: i } } })}
      ${input({ type: 'date', key: 'inv_expiry', attrs: { data: { dle: i } } })}<span></span></div>`;
  sheet(`<p class="eyebrow" data-t="inv_delivery"></p><h2 data-t="inv_delivery"></h2><p class="hint" data-t="inv_deliveryHint"></p>
    <div class="grid2"><div>${field({ id: 'd-sup', key: 'inv_supplier', autocomplete: 'off', attrs: { list: 'dSupList' } })}<datalist id="dSupList">${sups.map(s => `<option value="${esc(s)}">`).join('')}</datalist></div>
      <div>${field({ id: 'd-doc', key: 'inv_doc', autocomplete: 'off' })}</div></div>
    <p class="ui-label" data-t="inv_price"></p>${chips({ id: 'dMode', values: [{ value: 'per', label: `${t('inv_perKg')} / ${t('inv_perL')} / ${t('inv_perPiece')}` }, { value: 'total', key: 'inv_total' }], value: 'per', attr: 'dmode' })}
    <div id="dLines">${line(0)}</div>
    <div class="btn-row">${btn({ id: 'dAdd', icon: 'plus', key: 'inv_addLine' })}${btn({ id: 'dGo', variant: 'primary', icon: 'check', key: 'inv_saveAll' })}</div>
    <div id="dOut"></div>`, { name: 'delivery' });
  let n = 1, mode = 'per';
  for (const b of $$('[data-dmode]', $('#sheetIn'))) b.onclick = () => { mode = b.dataset.dmode; press($$('[data-dmode]', $('#sheetIn')), b); };
  $('#dAdd').onclick = () => { $('#dLines').insertAdjacentHTML('beforeend', line(n++)); retranslate($('#dLines')); };
  $('#dGo').onclick = async () => {
    const paper = { supplier: $('#d-sup').value.trim(), doc: $('#d-doc').value.trim() };
    const rows = $$('[data-dl]', $('#dLines')).map(r => ({ r, id: $('[data-dls]', r).value, qty: $('[data-dlq]', r).value, price: $('[data-dlp]', r).value,
      lot: $('[data-dll]', r).value.trim(), expiry: $('[data-dle]', r).value.trim() })).filter(x => x.id);
    if (!rows.length) return toast(t('required'));
    const out = [];
    await busy($('#dGo'), async () => {
      for (const x of rows) {
        const sup = all.find(s => s.id === x.id), qty = C.amount(x.qty, sup.unit);
        if (!qty || qty <= 0) { out.push([sup.name, t('inv_lineFailed') + ': ' + t('required')]); continue; }
        const body = { item: x.id, qty, ...C.priceBody(sup.unit, C.amount(x.price, 'unit'), mode) };
        for (const [kk, v] of [['supplier', paper.supplier], ['doc', paper.doc], ['lot', x.lot], ['expiry', x.expiry]]) if (v) body[kk] = v;
        try { const r = await post('/owner/stock/received', body); const v = (r.lines || [])[0]?.value; out.push([sup.name, `${qty} ${sup.unit}${v != null ? ' · ' + money(v) : ''}`]); x.r.remove(); }
        catch (e) { out.push([sup.name, `${t('inv_lineFailed')}: ${e.message || e}`]); }
      }
    });
    $('#dOut').innerHTML = `<div class="rows">${out.map(([a, b]) => rowDiv({ title: a, sub: esc(b) })).join('')}</div>`;
    ctx.reload();
  };
}

/// Every write-off, valued, with its reason, signer and lot; and the button
/// that records a new one.
export async function openWaste(ctx){
  sheet(`<p class="eyebrow" data-t="inv_waste"></p><h2 data-t="inv_waste"></h2><p class="hint" data-t="inv_wasteHint"></p>
    <div class="btn-row">${btn({ id: 'wNew', variant: 'primary', icon: 'trash', key: 'inv_writeOff' })}</div><div id="wOut">${loading(3)}</div>`, { name: 'waste' });
  $('#wNew').onclick = () => pickSupply(ctx, 'inv_writeOff', s => openMove(s, 'wasted', ctx));
  let r;
  try { r = await api('/owner/stock/waste'); } catch (e) { $('#wOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const tot = r.totals || {}, byValue = tot.valueByReason || {}, stockQty = (tot.byReason || {}).stock || {};
  const reasons = [...new Set([...Object.keys(byValue), ...Object.keys(stockQty)])];
  const heights = C.bars(reasons.map(x => byValue[x] || 0));
  const rows = (r.rows || []).slice().reverse();
  $('#wOut').innerHTML = `<div class="stats strip"><div class="stat money"><b>${money(tot.value || 0)}</b><small data-t="inv_value"></small></div>
      <div class="stat"><b>${rows.length}</b><small data-t="inv_lines"></small></div><div class="stat"><b>${tot.unvalued || 0}</b><small data-t="inv_unvalued"></small></div></div>
    <p class="eyebrow" data-t="inv_byReason"></p>
    ${reasons.length ? `<div class="kt-wrap"><table class="kt"><tbody>${reasons.map((x, i) => `<tr><td>${esc(t(x))}<span class="kt-bar"><i data-w="${heights[i]}"></i></span></td><td>${money(byValue[x] || 0)}</td></tr>`).join('')}</tbody></table></div>` : `<p class="hint" data-t="none"></p>`}
    <p class="eyebrow" data-t="inv_moves"></p>
    ${rows.length ? `<div class="kt-wrap"><table class="kt"><thead><tr><th data-t="ka_day"></th><th data-t="ka_ingredient"></th><th data-t="inv_qty"></th><th data-t="inv_value"></th><th data-t="reason"></th><th data-t="inv_lot"></th></tr></thead>
      <tbody>${rows.map(w => `<tr><td>${esc(when(w.at))}</td><td>${esc(w.source === 'void' ? w.item : nameOf(ctx, w.item))}</td><td>${w.qty}</td><td>${w.value != null ? money(w.value) : '-'}</td><td>${esc(t(w.reason))}</td><td>${esc(w.lot || '')}</td></tr>`).join('')}</tbody></table></div>`
      : empty('trash', { key: 'none' })}`;
  retranslate($('#wOut')); hydrate($('#wOut'));
}
