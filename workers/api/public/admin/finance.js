// THE MONEY SHEET: taxes and other spending entered by hand, and the month's
// plus, minus and net -- one row per source, so every number says where it
// came from. Every sum is the hub's (`services/finance`); this sheet draws,
// and sends what the owner typed.
//
// ONE SAVE AT A TIME, AND A SAVE HAS A NAME: a new form carries its own id, so
// a second tap or a retry after a lost answer is the same entry at the hub.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, icon, t, api, post, toast, sheet, busy, money, confirm, store, retranslate, hydrate } from '/admin/core.js';
import { intlLocale } from '/admin/i18n.js';
import { ui, btn, iconBtn, field, input, chips, loading, rowBtn, rowDiv, empty } from '/admin/parts.js';
import '/admin/finance-i18n.js';
import * as L from '/admin/finance-logic.js';
import { decimalsOf } from '/lib/money.js';

const view = { d: null, month: null, mode: 'profit', editing: null, formId: '', saving: false };
const q = month => '?location_id=' + encodeURIComponent(store.loc || '') + (month ? '&month=' + encodeURIComponent(month) : '');
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="analytics"></p><h2 data-t="fn_title"></h2><p class="sheet-hint" data-t="fn_hint"></p>`;
const stat = (key, value, cls = '') => `<div class="stat ${cls}"><small data-t="${key}"></small><b>${value}</b></div>`;
const amount = n => `<b class="mono">${esc(money(n || 0))}</b>`;
const decimals = () => decimalsOf(view.d?.currency || 'ALL');
const newFormId = () => L.formId(Date.now(), Math.random().toString(36).slice(2));

function ensureCss(){
  if (document.getElementById('fnCss')) return;
  const l = document.createElement('link');
  l.id = 'fnCss'; l.rel = 'stylesheet'; l.href = '/admin/finance.css';
  document.head.append(l);
}

/// A fresh open starts on the present month with nothing remembered.
export async function open(){
  ensureCss();
  Object.assign(view, { d: null, month: null, mode: 'profit', editing: null, formId: newFormId(), saving: false });
  show();
  await reload(null);
}

/// The sheet, drawn from what is already known: no read.
function show(){
  sheet(`${head}<div id="fnBody">${loading(3)}</div>`, { name: 'finance', keepScroll: true });
  if (view.d) paint();
}

/// Read `month` (null: the present one). A month that cannot be read leaves the
/// one on screen where it was; with nothing on screen the sheet says so.
async function reload(month = view.month, { quiet = false } = {}){
  let d;
  try { d = await api('/owner/finance' + q(month)); } catch (e) {
    if (view.d) { if (!quiet) fail(e); return paint(); }
    const b = $('#fnBody');
    if (!b) return;
    b.innerHTML = `${empty('alert-triangle', { key: 'loadFail', alert: true })}<div class="btn-row">${btn({ id: 'fnRetry', variant: 'ghost', icon: 'refresh', key: 'retry' })}</div>`;
    retranslate(b);
    $('#fnRetry', b).onclick = () => { b.innerHTML = loading(3); return reload(month); };
    return;
  }
  view.d = d;
  view.month = d.month;
  paint();
}

function paint(){
  const d = view.d || {};
  const modes = [{ value: 'profit', key: 'fn_profit' }, { value: 'tax', key: 'fn_tax' }, { value: 'other', key: 'fn_other' }];
  const atNow = String(d.today || '').slice(0, 7) === view.month;
  const root = $('#fnBody');
  if (!root) return;
  root.innerHTML = `
    <div class="fn-month">${iconBtn({ id: 'fnPrev', icon: 'chevron-left', ariaKey: 'fn_prev' })}<b class="mono" id="fnMonth" data-tour="finance.month">${esc(L.monthName(view.month, intlLocale()))}</b>${iconBtn({ id: 'fnNext', icon: 'chevron-right', ariaKey: 'fn_next', disabled: atNow })}</div>
    ${chips({ values: modes, value: view.mode, attr: 'mode' })}
    <div id="fnPane" class="mt-3">${view.mode === 'profit' ? profit(d) : expenses(d, view.mode)}</div>`;
  retranslate(root); hydrate(root);
  for (const b of $$('[data-mode]', root)) b.onclick = () => { view.mode = b.dataset.mode; view.editing = null; paint(); };
  for (const b of $$('[data-month]', root)) b.onclick = () => go(b.dataset.month);
  for (const b of $$('[data-exp]', root)) b.onclick = () => { view.editing = (d.expenses || []).find(e => e.id === b.dataset.exp) || null; paint(); };
  $('#fnPrev', root).onclick = () => go(L.shiftMonth(view.month, -1));
  $('#fnNext', root).onclick = () => { if (!atNow) go(L.shiftMonth(view.month, 1)); };
  const save = $('#feSave', root);
  if (save) save.onclick = () => saveForm(root);
  const del = $('#feDel', root);
  if (del) del.onclick = () => removeOne();
  const off = $('#feCancel', root);
  if (off) off.onclick = () => { view.editing = null; paint(); };
}

async function go(month){ view.editing = null; const b = $('#fnBody'); if (b) b.innerHTML = loading(3); await reload(month); }

function profit(d){
  const p = d.profit || { plus: { rows: [] }, minus: { rows: [] } };
  const src = r => { const hint = L.purchasesHint(r), undated = L.undatedOf(r);
    return rowDiv({ leading: icon(r.key === 'till' ? 'receipt' : r.key === 'orders' ? 'bowl-chopsticks' : r.key === 'purchases' ? 'shopping-bag' : r.key === 'payroll' ? 'user' : 'coin-hole'), title: { t: L.srcKey(r.key) },
      sub: (hint ? `<span data-t="${hint}"></span>${r.key === 'purchases' && r.uncosted ? ` <span class="mono">${esc(r.uncosted)}</span>` : ''}` : '')
        + (undated ? `${hint ? '<br>' : ''}<span data-t="fn_undated"></span> <span class="mono">${esc(undated)}</span>` : ''), trailing: amount(r.amount) }); };
  const history = d.history || [];
  return `<div class="stats">${stat('fn_plus', esc(money(p.plus.total || 0)))}${stat('fn_minus', esc(money(p.minus.total || 0)))}${stat('fn_net', esc(money(p.net || 0)), (p.net || 0) < 0 ? 'warn' : '')}</div>
    <section class="group mt-3"><p class="eyebrow" data-t="fn_plus"></p><div class="rows">${(p.plus.rows || []).map(src).join('')}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="fn_minus"></p><div class="rows">${(p.minus.rows || []).map(src).join('')}</div></section>
    ${d.archive?.error ? `<p class="muted small" data-t="fn_archiveError"></p>` : ''}
    <section class="group mt-3"><p class="eyebrow" data-t="fn_history"></p><div class="rows">${history.slice().reverse().map(h => rowBtn({ leading: icon('clock'), title: L.monthName(h.month, intlLocale()), sub: `${esc(money(h.plus))} − ${esc(money(h.minus))}`, trailing: amount(h.net), data: { month: h.month } })).join('')}</div></section>`;
}

function expenses(d, kind){
  const all = (d.expenses || []).filter(e => e.kind === kind);
  const total = kind === 'tax' ? d.tax?.total : d.other?.total;
  const e = view.editing;
  const today = d.today || '';
  return `<div class="stats">${stat(kind === 'tax' ? 'fn_taxTotal' : 'fn_otherTotal', esc(money(total || 0)))}</div>
    <section class="group mt-3" data-nodraft><p class="eyebrow" data-t="${e ? 'fn_edit' : 'fn_add'}"></p>
      ${input({ type: 'date', id: 'feDate', key: 'fn_date', value: e ? e.date : L.startDate(view.month, today), max: today })}
      ${field({ id: 'feName', key: 'fn_name', phKey: kind === 'tax' ? 'fn_exTax' : 'fn_exOther', value: e ? e.name : '', maxlength: 120, enterkeyhint: 'next' })}
      ${field({ id: 'feAmount', key: 'fn_amount', value: e ? (e.amount / 10 ** decimals()).toFixed(decimals()) : '', inputmode: 'decimal', enterkeyhint: 'done' })}
      <div class="btn-row">${e ? btn({ id: 'feDel', variant: 'danger', icon: 'trash', key: 'remove' }) + btn({ id: 'feCancel', variant: 'ghost', key: 'cancel' }) : ''}${btn({ id: 'feSave', variant: 'primary', icon: 'check', key: 'save' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="${kind === 'tax' ? 'fn_tax' : 'fn_other'}"></p><div class="rows">${all.map(x => rowBtn({ leading: icon('receipt'), title: x.name, sub: esc(L.shortDate(x.date)), trailing: amount(x.amount), data: { exp: x.id } })).join('') || empty('receipt', { key: 'fn_none' })}</div></section>`;
}

async function saveForm(root){
  if (view.saving) return;
  const form = { date: $('#feDate', root).value, name: $('#feName', root).value, amount: $('#feAmount', root).value };
  const editing = view.editing;
  const v = L.checkForm(form, view.mode, decimals(), { today: view.d?.today || '', id: editing ? '' : view.formId });
  if (v.err) return toast(t(v.err));
  view.saving = true;
  try {
    await busy($('#feSave', root), () => post('/owner/finance/expenses' + (editing ? '/' + encodeURIComponent(editing.id) : '') + q(view.month), v.body));
  } catch (e) {
    fail(e);
    // The entry is gone at the hub: no form for it, and the list says what is left.
    if (editing && e.status === 404) { view.editing = null; await reload(view.month, { quiet: true }); }
    return;
  } finally { view.saving = false; }
  view.editing = null;
  view.formId = newFormId();
  toast(t('saved'));
  await reload(v.body.date.slice(0, 7));
}

/// The confirmation takes the sheet's place and closes it, so every way out of
/// it draws the sheet again: a cancel from what is known, the rest from the hub.
async function removeOne(){
  const e = view.editing;
  if (!e) return;
  const c = await confirm(t('remove'), `${e.name} · ${money(e.amount)}`, { danger: true });
  if (!c) return show();
  view.editing = null;
  let refused = false;
  try { await api('/owner/finance/expenses/' + encodeURIComponent(e.id) + q(view.month), { method: 'DELETE' }); } catch (err) { refused = true; fail(err); }
  show();
  // One sentence per failure: a refused delete has said its own.
  await reload(view.month, { quiet: refused });
}
