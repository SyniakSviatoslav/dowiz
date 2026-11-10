// The payroll tab ("Зарплата"): the owner enters a person's hours by date, pay
// follows from that person's hourly rate, and the table shows the month.
//
// THE HUB DECIDES THE MONEY (`services/operations/payroll/rules.rs`): this
// file only draws what the hub answers and sends what the owner typed. Rates
// and entries live in the venue's own object, so one venue's owner never asks
// about another's. Rows are the existing staff (`staff.js`), not a second list.
//
// ONE SHEET: the month, the entry form and the employee's page replace one
// another, so a close always lands back on the Venue screen.
//
// EVERY REQUEST NAMES THE VENUE ON SCREEN (`at`): an owner of two venues reads
// and writes the one the console shows, not the one the token was issued for.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { $, esc, icon, t, S, store, api, post, toast, sheet, busy, confirm, money, moneyEl } from '/admin/core.js';
import { retranslate, intlLocale } from '/admin/i18n.js';
import { btn, iconBtn, field, input, select, empty, loading, rowBtn } from '/admin/parts.js';
import { parseMinor } from '/admin/wire-logic.js';
import { decimalsOf } from '/lib/money.js';
import { parseHours, hoursLabel, entryPay, shiftMonth, monthTitle, cellOf, monthBounds, dateIn } from '/admin/payroll-logic.js';
import '/admin/payroll-i18n.js';

const view = { month: null, data: null };
/// A payroll path with the venue on screen, and whatever else the read needs.
const at = (path, more = {}) => `${path}?` + Object.entries({ location_id: store.loc || '', ...more }).map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join('&');

function ensureCss(){
  if (document.getElementById('payCss')) return;
  const l = document.createElement('link');
  l.id = 'payCss'; l.rel = 'stylesheet'; l.href = '/admin/payroll.css';
  document.head.append(l);
}

/// The sheet's one body, translated and wired to one click handler.
function draw(html, onclick){
  sheet(`<div id="payBody">${html}</div>`, { name: 'payroll', keepScroll: true });
  const host = $('#payBody');
  retranslate($('#sheetIn'));
  host.onclick = onclick || null;
  return host;
}

/// A fresh open starts on the present month with nothing remembered.
export async function open(){
  ensureCss();
  view.month = null; view.data = null;
  draw(`<p class="eyebrow" data-t="analytics"></p><h2 data-t="payroll"></h2>${loading(3)}`);
  await showMonth();
}

/// The card on `staff.js` opens the employee straight away.
export async function openEmployee(id){
  ensureCss();
  view.month = null; view.data = null;
  draw(loading(3));
  await load();
  if (view.data) showEmployee(id);
}

/// Read `month` (null: the present one). A month that cannot be read leaves the
/// one already read where it was and says why; with nothing read yet the sheet
/// says so and offers to ask again.
async function load(month = view.month){
  try {
    const d = await api(at('/owner/payroll', month ? { month } : {}));
    view.data = d;
    view.month = d.month;
  } catch (e) {
    if (view.data) return toast(String(e.message || e));
    const host = draw(`<p class="eyebrow" data-t="analytics"></p><h2 data-t="payroll"></h2>${empty('alert-triangle', { key: 'loadFail', alert: true })}
      <div class="btn-row">${btn({ id: 'payRetry', variant: 'ghost', icon: 'refresh', key: 'retry' })}</div>`);
    $('#payRetry', host).onclick = () => showMonth(month);
  }
}

const staffOf = id => (view.data?.staff || []).find(s => s.id === id);
const nameOf = (id, name) => name || staffOf(id)?.name || id.slice(0, 8);

// ── the month ───────────────────────────────────────────────────────────────
async function showMonth(month = view.month){
  await load(month);
  const d = view.data;
  if (!d) return;
  const host = draw(monthMarkup(d), monthClick);
  $('#payPrev', host).onclick = () => stepMonth(-1);
  $('#payNext', host).onclick = () => stepMonth(1);
  if ($('#payAdd', host)) $('#payAdd', host).onclick = () => showForm({});
}

async function stepMonth(delta){
  await showMonth(shiftMonth(view.month, delta));
}

function monthMarkup(d){
  const noStaff = !(d.staff || []).length;
  const nav = `<div class="pay-month" data-tour="payroll.month">${iconBtn({ id: 'payPrev', icon: 'chevron-left', ariaKey: 'pay_prev' })}
    <b class="pay-title">${esc(monthTitle(d.month, intlLocale()))}</b>${iconBtn({ id: 'payNext', icon: 'chevron-right', ariaKey: 'pay_next', disabled: d.month >= d.today.slice(0, 7) })}</div>`;
  const body = noStaff ? empty('user', { key: 'pay_noStaff', bodyKey: 'pay_hint' })
    : `<div class="btn-row">${btn({ id: 'payAdd', variant: 'primary', icon: 'plus', key: 'pay_add', tour: 'payroll.add' })}</div>
       ${grid(d)}${(d.dates || []).length ? '' : '<p class="muted small" data-t="pay_noEntries"></p>'}`;
  return `<p class="eyebrow" data-t="analytics"></p><h2 data-t="payroll"></h2><p class="muted small" data-t="pay_hint"></p>${nav}${body}`;
}

/// Name, then the month's hours and pay, then the days: on a phone the sums are
/// what is read first, and a full month of days scrolls away to the right.
function grid(d){
  const dates = d.dates || [];
  const cell = (r, date) => {
    const v = r.days?.[date];
    return `<td class="pay-num">${v ? btn({ label: hoursLabel(v.minutes), variant: 'ghost', cls: 'pay-cell', data: { edit: `${r.staffId}|${date}` } }) : ''}</td>`;
  };
  const row = r => `<tr class="${r.active === false ? 'off' : ''}"><th scope="row">${btn({ label: nameOf(r.staffId, r.name), variant: 'ghost', cls: 'pay-name', data: { emp: r.staffId } })}</th>
    <td class="pay-num pay-sum">${esc(hoursLabel(r.minutes))}</td><td class="pay-num pay-sum">${moneyEl(r.payMinor)}</td>${dates.map(date => cell(r, date)).join('')}</tr>`;
  return `<div class="pay-scroll" data-tour="payroll.grid"><table class="pay-grid">
    <thead><tr><th scope="col" data-t="pay_employee"></th><th scope="col" class="pay-num pay-sum" data-t="pay_sumHours"></th><th scope="col" class="pay-num pay-sum" data-t="pay_sumPay"></th>
      ${dates.map(x => `<th scope="col" class="pay-num">${esc(x.slice(8))}</th>`).join('')}</tr></thead>
    <tbody>${d.rows.map(row).join('')}</tbody>
    <tfoot><tr><th scope="row" data-t="pay_total"></th><td class="pay-num pay-sum">${esc(hoursLabel(d.totalMinutes))}</td><td class="pay-num pay-sum">${moneyEl(d.totalMinor)}</td>${dates.map(() => '<td></td>').join('')}</tr></tfoot>
  </table></div>`;
}

function monthClick(e){
  const edit = e.target.closest('[data-edit]');
  if (edit) { const [staffId, date] = edit.dataset.edit.split('|'); return showForm({ staffId, date }); }
  const emp = e.target.closest('[data-emp]');
  if (emp) return showEmployee(emp.dataset.emp);
}

// ── one entry ───────────────────────────────────────────────────────────────
const back = id => btn({ id, variant: 'ghost', icon: 'arrow-left', key: 'pay_back', cls: 'mt-3' });

function showForm({ staffId, date }){
  const d = view.data;
  const staff = (d.staff || []).filter(s => s.active !== false || s.id === staffId).map(s => ({ value: s.id, label: s.name || s.id.slice(0, 8) }));
  // A row whose person is no longer on the staff list is still that person's row.
  if (staffId && !staff.some(s => s.value === staffId)) staff.push({ value: staffId, label: nameOf(staffId) });
  const sid = staffId || staff[0]?.value || '';
  const bounds = monthBounds(d.month, d.today);
  const day = date || (d.month === d.today.slice(0, 7) ? d.today : bounds.min);
  const host = draw(`<p class="eyebrow" data-t="payroll"></p><h2 data-t="pay_add"></h2>
    ${select({ id: 'pe-staff', key: 'pay_employee', value: sid, options: staff })}
    ${input({ id: 'pe-date', type: 'date', key: 'pay_date', value: day, min: bounds.min, max: bounds.max })}
    ${field({ id: 'pe-hours', key: 'pay_hours', inputmode: 'decimal', autocomplete: 'off', hintKey: 'pay_hoursHint' })}
    ${field({ id: 'pe-note', key: 'pay_note', autocomplete: 'off' })}
    <p class="muted small pay-preview" id="pe-pay" aria-live="polite"></p>
    <div class="btn-row">${btn({ id: 'peSave', variant: 'primary', icon: 'check', key: 'save' })}${btn({ id: 'peDel', variant: 'ghost', icon: 'trash', key: 'pay_delete' })}</div>
    ${back('peBack')}`);
  const el = id => $(id, host);
  const picked = () => el('#pe-staff').value || sid;
  let shown = false; // the fields hold a stored entry, not what the owner is typing
  const follow = () => {
    const cur = cellOf(d, picked(), el('#pe-date').value);
    if (cur) { el('#pe-hours').value = hoursLabel(cur.minutes); el('#pe-note').value = cur.note || ''; }
    else if (shown) { el('#pe-hours').value = ''; el('#pe-note').value = ''; }
    shown = !!cur;
    el('#peDel').hidden = !cur;
    preview(host, cur, picked());
  };
  el('#pe-staff').onchange = el('#pe-date').onchange = follow;
  el('#pe-hours').oninput = () => { shown = false; preview(host, cellOf(d, picked(), el('#pe-date').value), picked()); };
  follow();
  el('#peBack').onclick = () => showMonth();
  el('#peSave').onclick = () => saveEntry(host, bounds, picked());
  el('#peDel').onclick = () => removeEntry(host, picked());
}

/// The hours and the money the entry will carry; a person with no rate is told
/// so here, because the hub refuses the entry until one is set.
function preview(host, cur, sid){
  const out = $('#pe-pay', host), save = $('#peSave', host);
  // Hours are entered for the staff the venue has: a person who left keeps
  // the days already entered, which can be read and deleted, not changed.
  if (!staffOf(sid)) { save.disabled = true; out.textContent = t('pay_gone'); return; }
  const rate = cur?.rateMinor ?? staffOf(sid)?.rateMinor;
  if (rate == null) {
    save.disabled = true;
    out.innerHTML = `${esc(t('pay_rateFirst'))} ${btn({ id: 'peRate', variant: 'ghost', key: 'pay_setRate' })}`;
    retranslate(out);
    $('#peRate', host).onclick = () => showEmployee(sid);
    return;
  }
  save.disabled = false;
  const minutes = parseHours($('#pe-hours', host).value);
  out.textContent = minutes == null ? `${t('pay_rate')}: ${money(rate)}`
    : `${hoursLabel(minutes)} x ${money(rate)} = ${money(entryPay(minutes, rate))}${cur ? ' · ' + t('pay_replaces') : ''}`;
}

async function saveEntry(host, bounds, staff_id){
  const date = $('#pe-date', host).value;
  if (!dateIn(date, bounds)) return toast(t('pay_dateOutside'));
  const minutes = parseHours($('#pe-hours', host).value);
  if (minutes == null) return toast(t('pay_badHours'));
  const body = { staff_id, date, minutes, note: $('#pe-note', host).value.trim() };
  try {
    await busy($('#peSave', host), () => post(at('/owner/payroll/entry'), body));
    toast(t('saved'));
    await showMonth(date.slice(0, 7));
  } catch (err) { toast(String(err.message || err)); }
}

async function removeEntry(host, staff_id){
  const body = { staff_id, date: $('#pe-date', host).value };
  // The confirmation takes the sheet's place and closes it: a cancel draws the entry again.
  if (!(await confirm(t('pay_delete'), t('pay_deleteAsk'), { danger: true }))) return showForm({ staffId: body.staff_id, date: body.date });
  try {
    await post(at('/owner/payroll/entry/delete'), body);
    toast(t('saved'));
  } catch (err) { toast(String(err.message || err)); }
  await showMonth();
}

// ── one employee: the rate, and pay by month ────────────────────────────────
async function showEmployee(id){
  const s = staffOf(id) || { id };
  const decimals = decimalsOf(S.venue?.currencyCode || 'ALL');
  const host = draw(`<p class="eyebrow" data-t="payroll"></p><h2>${esc(s.name || id.slice(0, 8))}</h2>
    ${staffOf(id) ? `${field({ id: 'pay-rate', key: 'pay_rate', inputmode: decimals ? 'decimal' : 'numeric', autocomplete: 'off', value: s.rateMinor == null ? '' : rateText(s.rateMinor, decimals), hintKey: 'pay_rateHint' })}
    <div class="btn-row">${btn({ id: 'payRateGo', variant: 'primary', icon: 'check', key: 'save' })}</div>` : '<p class="muted small" data-t="pay_gone"></p>'}
    <p class="eyebrow mt-3" data-t="pay_history"></p><div id="payHist">${loading(2)}</div>
    ${back('payEmpBack')}`);
  $('#payEmpBack', host).onclick = () => showMonth();
  if ($('#payRateGo', host)) $('#payRateGo', host).onclick = async () => {
    const rate_minor = parseMinor($('#pay-rate', host).value, decimals);
    if (rate_minor == null || rate_minor <= 0) return toast(t('pay_badRate'));
    try {
      await busy($('#payRateGo', host), () => post(at('/owner/payroll/rate'), { staff_id: id, rate_minor }));
      toast(t('saved'));
      await load();
      if (view.data) showEmployee(id);
    } catch (err) { toast(String(err.message || err)); }
  };
  try {
    const h = await api(at('/owner/payroll/history', { staff_id: id }));
    const months = h.months || [];
    const box = $('#payHist', host);
    box.innerHTML = months.length ? `<div class="rows">${months.map(m => rowBtn({ data: { month: m.month }, leading: icon('history'), title: monthTitle(m.month, intlLocale()),
      sub: `${esc(hoursLabel(m.minutes))} ${esc(t('pay_hoursShort'))}`, trailing: moneyEl(m.payMinor) })).join('')}</div>` : empty('history', { key: 'pay_noHistory' });
    retranslate(box);
    box.onclick = e => { const r = e.target.closest('[data-month]'); if (r) showMonth(r.dataset.month); };
  } catch (e) { $('#payHist', host).innerHTML = empty('alert-triangle', { key: 'loadFail', alert: true }); }
}

/// A stored rate back in the field: the minor units as the owner would type them.
function rateText(minor, decimals){
  if (!decimals) return String(minor);
  const p = String(minor).padStart(decimals + 1, '0');
  return `${p.slice(0, -decimals)}.${p.slice(-decimals)}`;
}
