// SUPPLIERS (W-STOCK P5): who the kitchen buys from, as cards -- name, phone,
// Telegram, delivery days, lead time, order cutoff, the language an order is
// written in -- and which ingredients come from each. The cards are notes on
// the stock log, written through the stock door that already exists:
//   POST /api/owner/stock/supplier  {card: {...}}       (gone: true takes one off)
// An ingredient's supplier is its own `supplier` field (POST /api/owner/supplies).

import { $, $$, esc, t, api, post, sheet, closeSheet, busy, toast, confirm, LANGS } from '/admin/core.js';
import { ui, btn, field, select, check, rowBtn, empty, loading } from '/admin/parts.js';

const fail = e => toast(String(e.message || e));
const DAYS = [1, 2, 3, 4, 5, 6, 7];
const norm = s => String(s ?? '').trim().toLowerCase();

export async function open(){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="su_title"></h2><p class="sheet-hint" data-t="su_hint"></p>
    <div id="suList">${loading(2)}</div>
    <div class="btn-row">${btn({ id: 'suAdd', variant: 'primary', icon: 'plus', key: 'su_add' })}</div>`, { name: 'suppliers' });
  let stock;
  try { stock = await api('/owner/stock'); } catch (e) { $('#suList').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const cards = stock.supplierCards || [];
  const of = c => (stock.supplies || []).filter(s => norm(s.supplier) === norm(c.name) || norm(s.supplier) === c.id);
  $('#suList').innerHTML = cards.length ? `<div class="rows">${cards.map(c => rowBtn({ title: c.name, data: { su: c.id },
    sub: `<span class="mono">${esc([c.phone, c.days.map(d => t('su_d' + d)).join(' '), `${c.leadDays} d`, `${of(c).length} ×`].filter(Boolean).join(' · '))}</span>` })).join('')}</div>`
    : empty('building', { key: 'su_none', bodyKey: 'su_link' });
  $('#suList').onclick = e => { const r = e.target.closest('[data-su]'); if (r) edit(cards.find(c => c.id === r.dataset.su), stock); };
  $('#suAdd').onclick = () => edit(null, stock);
}

/// One card, and the ingredients that come from it.
function edit(card, stock){
  const c = card || { name: '', phone: '', telegram: '', days: [], leadDays: 1, cutoff: '', lang: '' };
  const mine = new Set((stock.supplies || []).filter(s => card && (norm(s.supplier) === norm(card.name) || norm(s.supplier) === card.id)).map(s => s.id));
  sheet(`<p class="eyebrow" data-t="su_title"></p><h2>${esc(c.name || t('su_add'))}</h2>
    ${field({ id: 'su-name', key: 'su_name', value: c.name, autocomplete: 'off' })}
    <div class="grid2"><div>${field({ id: 'su-phone', key: 'su_phone', value: c.phone, inputmode: 'tel', autocomplete: 'off' })}</div>
      <div>${field({ id: 'su-tg', key: 'su_telegram', value: c.telegram, autocomplete: 'off' })}</div></div>
    <p class="ui-label" data-t="su_days"></p>
    <div class="chips" id="suDays" role="group">${DAYS.map(d => ui.chip({ as: 'button', selected: c.days.includes(d), label: { t: 'su_d' + d }, attrs: { data: { sd: String(d) } } })).join('')}</div>
    <div class="grid2"><div>${field({ id: 'su-lead', key: 'su_lead', value: c.leadDays, inputmode: 'numeric' })}</div>
      <div>${field({ id: 'su-cut', key: 'su_cutoff', value: c.cutoff, inputmode: 'numeric', placeholder: '18:00' })}</div></div>
    ${select({ id: 'su-lang', key: 'su_lang', value: c.lang, options: [{ value: '', key: 'su_langOwn' }, ...LANGS.map(l => ({ value: l, label: l.toUpperCase() }))] })}
    <p class="ui-label" data-t="inv_title"></p><p class="hint" data-t="su_link"></p>
    <div id="suSup">${(stock.supplies || []).slice().sort((a, b) => String(a.name).localeCompare(String(b.name)))
      .map(s => check({ id: `su-s-${s.id}`, label: s.supplier ? `${s.name} · ${s.supplier}` : s.name, checked: mine.has(s.id), data: { sup: s.id } })).join('')}</div>
    <div class="btn-row">${btn({ id: 'suSave', variant: 'primary', icon: 'check', key: 'save' })}</div>
    ${card ? `<div class="btn-row">${btn({ id: 'suGone', variant: 'danger', icon: 'trash', key: 'su_remove' })}</div>` : ''}`, { name: 'supplier' });
  for (const b of $$('[data-sd]', $('#suDays'))) b.onclick = () => b.setAttribute('aria-pressed', String(b.getAttribute('aria-pressed') !== 'true'));
  $('#suSave').onclick = async () => {
    const name = $('#su-name').value.trim();
    if (!name) return toast(t('required'));
    const lead = Number($('#su-lead').value.trim() || 0);
    const body = { card: { ...(card ? { id: card.id } : {}), name, phone: $('#su-phone').value.trim(), telegram: $('#su-tg').value.trim(),
      days: $$('[data-sd]', $('#suDays')).filter(b => b.getAttribute('aria-pressed') === 'true').map(b => Number(b.dataset.sd)),
      leadDays: Number.isInteger(lead) ? lead : 0, cutoff: $('#su-cut').value.trim(), lang: $('#su-lang').value } };
    try {
      await busy($('#suSave'), async () => {
        const r = await post('/owner/stock/supplier', body);
        // An ingredient ticked or unticked here: its own `supplier` field.
        for (const i of $$('[data-sup]', $('#suSup'))) {
          const was = mine.has(i.dataset.sup);
          if (i.checked !== was) await post('/owner/supplies', { id: i.dataset.sup, supplier: i.checked ? r.card.name : '' });
        }
      });
      toast(t('saved')); closeSheet(); open();
    } catch (e) { fail(e); }
  };
  const gone = $('#suGone');
  if (gone) gone.onclick = async () => {
    if (!(await confirm(t('su_remove'), t('su_removeQ'), { danger: true }))) return edit(card, stock);
    try { await post('/owner/stock/supplier', { card: { id: card.id, name: card.name, gone: true } }); toast(t('saved')); closeSheet(); open(); } catch (e) { fail(e); }
  };
}
