// Fiscal sending (card L70): orders that take money, sent to ebills.al as
// fiscal invoices -- ONLY once the owner arms it here, with the consequence
// typed out. One sheet: armed or why not, the sale unit and fee item, the
// queue with each order's fiscal stage and deadline, and the receipt.
//
// ARMING IS A LEGAL ACT. The switch is a phrase typed by the owner, checked
// here and again by the server; switching it OFF needs nothing.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, icon, t, api, post, toast, sheet, ago, day, clock, store, busy } from '/admin/core.js';

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="settings"></p><h2 data-t="fx_title"></h2><p class="muted small" data-t="fx_hint"></p>`;
const when = ms => ms ? `${day(ms)} ${clock(ms)}` : '';

export async function open(){
  sheet(`${head}<div id="fxBody"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>`, { name: 'fiscal', keepScroll: true });
  let d;
  try { d = await api('/owner/fiscal' + q()); } catch (e) { return fail(e); }
  const a = d.arming || {};
  const row = (ic, label, value, tone = '') => `<div class="rowc">${icon(ic)}<span class="t"><b data-t="${label}"></b><small class="mono">${value}</small></span>${tone ? `<span class="pill ${tone}"></span>` : ''}</div>`;
  const units = (d.floor || []).map(u => `<option value="${esc(u)}" ${u === a.sale_unit ? 'selected' : ''}>${esc(u)}</option>`).join('');
  const items = (d.items || []).map(i => `<option value="${esc(i.code)}" ${i.code === a.fee_item ? 'selected' : ''}>${esc(i.code)} - ${esc(i.name)}</option>`).join('');
  const waiting = (d.waiting || []).map(w => `<div class="rowc" data-order="${esc(w.order_id)}">${icon(w.overdue ? 'alert-triangle' : 'receipt')}
      <span class="t"><b class="mono">${esc(w.order_id)}</b><small><span data-t="fx_stage_${esc(w.stage)}"></span> · <span data-t="fx_deadline"></span> ${esc(when(w.deadline))}${w.sale_id ? ' · #' + esc(w.sale_id) : ''}${w.why ? ' · ' + esc(w.why) : ''}</small></span>
      <button class="btn ghost fx-receipt">${icon('receipt')}<span data-t="fx_receipt"></span></button></div>`).join('');
  const body = `
    <div class="rows">
      <div class="rowc">${icon('receipt')}<span class="t"><b data-t="fx_title"></b><small>${d.armed ? '' : (d.why || []).map(esc).join(' / ')}</small></span><span class="pill ${d.armed ? 'ok' : ''}" data-t="${d.armed ? 'fx_armed' : 'fx_off'}"></span></div>
      ${row('clock', 'fx_lastOk', d.last_ok_ms ? esc(ago(d.last_ok_ms)) : esc(t('eb_never')))}
      ${d.last_error ? row('alert-triangle', 'fx_lastError', `${esc(ago(d.last_error[0]))} · ${esc(d.last_error[1])}`) : ''}
      ${row('clock', 'fx_backlog', `${esc(d.backlog || 0)} · ${esc(d.overdue || 0)} ${esc(t('fx_overdue'))} · ${esc(d.sent || 0)} ${esc(t('fx_sent'))}`)}
    </div>
    <section class="group mt-3"><p class="eyebrow" data-t="fx_waiting"></p>
      <div class="rows">${waiting || `<p class="muted small" data-t="fx_none"></p>`}</div>
      <pre id="fxReceipt" class="mono small" hidden></pre></section>
    <section class="group mt-3"><p class="eyebrow" data-t="fx_setup"></p>
      <label for="fx-unit" data-t="fx_saleUnit"></label><select id="fx-unit"><option value="" data-t="fx_pick"></option>${units}${a.sale_unit && !(d.floor || []).includes(a.sale_unit) ? `<option selected value="${esc(a.sale_unit)}">${esc(a.sale_unit)}</option>` : ''}</select>
      <p class="muted small" data-t="fx_saleUnitHint"></p>
      <label for="fx-fee" data-t="fx_feeItem"></label><select id="fx-fee"><option value="" data-t="fx_noFee"></option>${items}</select>
      <div class="btn-row"><button class="btn" id="fxSave">${icon('check')}<span data-t="save"></span></button></div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="fx_arming"></p>
      <p class="small" data-t="fx_consequence"></p><p class="muted small"><span data-t="fx_marker"></span> <b class="mono">${esc(d.marker || '')}</b></p>
      ${d.arming && d.arming.armed
        ? `<div class="btn-row"><button class="btn ghost danger" id="fxDisarm">${icon('x')}<span data-t="fx_disarm"></span></button></div>`
        : `<label for="fx-phrase"><span data-t="fx_typePhrase"></span> <b class="mono">${esc(d.confirm)}</b></label><input id="fx-phrase" autocomplete="off">
           <div class="btn-row"><button class="btn danger" id="fxArm">${icon('alert-triangle')}<span data-t="fx_arm"></span></button></div>`}
      <p class="muted small"><span data-t="fx_cancelState"></span> <b data-t="${a.cancel_armed ? 'fx_armed' : 'fx_off'}"></b> · ${esc(d.cancel_note || '')} · ${esc((d.cancels || []).length)} <span data-t="fx_cancelsQueued"></span></p></section>`;
  sheet(head + `<div id="fxBody">${body}</div>`, { name: 'fiscal', keepScroll: true });
  wire(d);
}

function wire(d){
  const save = body => post('/owner/fiscal/ebills' + q(), body);
  $('#fxSave').onclick = async () => {
    try { await busy($('#fxSave'), () => save({ sale_unit: $('#fx-unit').value, fee_item: $('#fx-fee').value })); toast(t('saved')); open(); } catch (e) { fail(e); }
  };
  const arm = $('#fxArm');
  if (arm) arm.onclick = async () => {
    const phrase = $('#fx-phrase').value.trim();
    if (phrase !== d.confirm) return toast(t('fx_typePhrase') + ' ' + d.confirm);
    try { await busy(arm, () => save({ armed: true, confirm: phrase })); toast(t('fx_armedNow')); open(); } catch (e) { fail(e); }
  };
  const disarm = $('#fxDisarm');
  if (disarm) disarm.onclick = async () => {
    try { await busy(disarm, () => save({ armed: false })); toast(t('fx_disarmedNow')); open(); } catch (e) { fail(e); }
  };
  for (const b of $$('.fx-receipt', $('#fxBody'))) b.onclick = async () => {
    const id = b.closest('[data-order]').dataset.order;
    try {
      const r = await busy(b, () => api('/owner/orders/' + encodeURIComponent(id) + '/receipt' + q()));
      const pre = $('#fxReceipt'); pre.textContent = r.text; pre.hidden = false;
    } catch (e) { fail(e); }
  };
}
