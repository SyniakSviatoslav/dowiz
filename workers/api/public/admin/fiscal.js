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
import { btn, field, select, pill, loading, rowDiv } from '/admin/parts.js';

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="settings"></p><h2 data-t="fx_title"></h2><p class="muted small" data-t="fx_hint"></p>`;
const when = ms => ms ? `${day(ms)} ${clock(ms)}` : '';

export async function open(){
  sheet(`${head}<div id="fxBody">${loading(2)}</div>`, { name: 'fiscal', keepScroll: true });
  let d;
  try { d = await api('/owner/fiscal' + q()); } catch (e) { return fail(e); }
  const a = d.arming || {};
  const row = (ic, label, value) => rowDiv({ leading: icon(ic), title: { t: label }, sub: `<span class="mono">${value}</span>` });
  const units = (d.floor || []).map(u => ({ value: u, label: u }));
  // A sale unit the till no longer lists is still shown, chosen, as before.
  const lost = a.sale_unit && !(d.floor || []).includes(a.sale_unit) ? [{ value: a.sale_unit, label: a.sale_unit }] : [];
  const items = (d.items || []).map(i => ({ value: i.code, label: `${i.code} - ${i.name}` }));
  const words = (k, rest = '') => `<span data-t="${k}">${esc(t(k))}</span>${rest}`;
  const waiting = (d.waiting || []).map(w => rowDiv({ leading: icon(w.overdue ? 'alert-triangle' : 'receipt'), title: w.order_id, data: { order: w.order_id }, tour: 'fiscal.stage',
      sub: `${words('fx_stage_' + w.stage)} · ${words('fx_deadline')} ${esc(when(w.deadline))}${w.sale_id ? ' · #' + esc(w.sale_id) : ''}${w.why ? ' · ' + esc(w.why) : ''}`,
      trailing: btn({ cls: 'fx-receipt', variant: 'ghost', icon: 'receipt', key: 'fx_receipt', tour: 'fiscal.receipt' }) })).join('');
  const body = `
    <div class="rows">
      ${rowDiv({ leading: icon('receipt'), title: { t: 'fx_title' }, sub: d.armed ? '' : (d.why || []).map(esc).join(' / '), trailing: pill(d.armed ? 'ok' : '', { key: d.armed ? 'fx_armed' : 'fx_off' }), tour: 'fiscal.state' })}
      ${row('clock', 'fx_lastOk', d.last_ok_ms ? esc(ago(d.last_ok_ms)) : esc(t('eb_never')))}
      ${d.last_error ? row('alert-triangle', 'fx_lastError', `${esc(ago(d.last_error[0]))} · ${esc(d.last_error[1])}`) : ''}
      ${row('clock', 'fx_backlog', `${esc(d.backlog || 0)} · ${esc(d.overdue || 0)} ${esc(t('fx_overdue'))} · ${esc(d.sent || 0)} ${esc(t('fx_sent'))}`)}
    </div>
    <section class="group mt-3"><p class="eyebrow" data-t="fx_waiting"></p>
      <div class="rows" data-tour="fiscal.waiting">${waiting || `<p class="muted small" data-t="fx_none"></p>`}</div>
      <pre id="fxReceipt" class="mono small" hidden></pre></section>
    <section class="group mt-3"><p class="eyebrow" data-t="fx_setup"></p>
      ${select({ id: 'fx-unit', key: 'fx_saleUnit', value: a.sale_unit || '', options: [{ value: '', key: 'fx_pick' }, ...units, ...lost], tour: 'fiscal.saleUnit' })}
      <p class="muted small" data-t="fx_saleUnitHint"></p>
      ${select({ id: 'fx-fee', key: 'fx_feeItem', value: a.fee_item || '', options: [{ value: '', key: 'fx_noFee' }, ...items], tour: 'fiscal.feeItem' })}
      <div class="btn-row">${btn({ id: 'fxSave', variant: 'primary', icon: 'check', key: 'save', tour: 'fiscal.save' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="fx_arming"></p>
      <p class="small" data-t="fx_consequence"></p><p class="muted small"><span data-t="fx_marker"></span> <b class="mono">${esc(d.marker || '')}</b></p>
      ${d.arming && d.arming.armed
        ? `<div class="btn-row">${btn({ id: 'fxDisarm', variant: 'ghost', icon: 'x', key: 'fx_disarm', tour: 'fiscal.disarm' })}</div>`
        : `${field({ id: 'fx-phrase', label: `${t('fx_typePhrase')} ${d.confirm || ''}`, autocomplete: 'off', spellcheck: false, tour: 'fiscal.phrase' })}
           <div class="btn-row">${btn({ id: 'fxArm', variant: 'danger', icon: 'alert-triangle', key: 'fx_arm', tour: 'fiscal.arm' })}</div>`}
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
