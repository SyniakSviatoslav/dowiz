// TAX, FISCAL START, AND THE TWO NOTICE SWITCHES (W-WIRE rows 5, 6, 11).
//
// Four `tax.*` keys had no screen at all; `fiscal.since_ms` was set only by a
// raw settings call; `notify.whatsapp.status` (paid WhatsApp notices) and
// `notify.order.late_min` (when an order counts as late for the Telegram
// groups) had no control. Each is a key of the closed settings list
// (`settings/known.rs`) written through `POST /api/owner/settings`; the hub
// refuses a bad value (`tax_cfg::validate`) and its words are shown as sent.
//
// A person types a percentage and a date; the hub stores parts per million
// and epoch milliseconds. Nothing here is a float.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, esc, t, api, post, busy, toast, switchEl } from '/admin/core.js';
import { btn, field, input, rowDiv, pill, loading } from '/admin/parts.js';
import { q, tz, fail, open, paint } from '/admin/wire-core.js';
import { pctToPpm, ppmToPct, dayToMs, msToDay, scheduleRows, scheduleJson } from '/admin/wire-logic.js';

export const KEYS = { rate: 'tax.default_ppm', incl: 'tax.prices_include', fee: 'tax.delivery_fee_ppm', sched: 'tax.schedule',
  since: 'fiscal.since_ms', wa: 'notify.whatsapp.status', late: 'notify.order.late_min' };

/// The writes a form asks for, or the first refusal (a key and its reason).
export function writes(f){
  const out = [];
  const rate = f.rate.trim() ? pctToPpm(f.rate) : '';
  if (rate === null) return { bad: 'w_badRate' };
  const fee = f.fee.trim() ? pctToPpm(f.fee) : '';
  if (fee === null) return { bad: 'w_badRate' };
  const sched = scheduleJson(f.rows, f.tz);
  if (sched.bad.length) return { bad: 'w_badSchedule', rows: sched.bad };
  const since = f.since ? dayToMs(f.since, f.tz) : '';
  if (since === null) return { bad: 'w_badDate' };
  const late = String(f.late).trim();
  if (!/^\d{1,4}$/.test(late)) return { bad: 'w_badMinutes' };
  out.push([KEYS.rate, String(rate)], [KEYS.incl, f.incl ? 'true' : 'false'], [KEYS.fee, String(fee)], [KEYS.sched, sched.json],
    [KEYS.since, String(since)], [KEYS.wa, f.wa ? 'on' : 'off'], [KEYS.late, late]);
  return { out };
}

const rowHtml = (r, i) => `<div class="grid2" data-row="${i}">${input({ type: 'date', key: 'w_from', value: r.day, data: { sd: i } })}
  ${field({ key: 'w_ratePct', inputmode: 'decimal', value: r.pct, data: { sp: i }, autocomplete: 'off' })}</div>`;

export async function mount(host){
  if (!host) return;
  host.innerHTML = loading(3);
  let s, fx = {};
  try { s = await api('/owner/settings'); } catch (e) { return fail(e); }
  try { fx = await api('/owner/fiscal' + q()); } catch { fx = {}; }
  const v = s.values || {};
  // A key never written reads as its declared default, so saving an untouched
  // form writes nothing.
  const def = Object.fromEntries((s.known || []).map(k => [k.key, k.default]));
  let rows = scheduleRows(v[KEYS.sched], tz());
  const since = Number(v[KEYS.since]) || 0;
  host.innerHTML = `<p class="eyebrow" data-t="w_tax"></p><p class="muted small" data-t="w_taxHint"></p>
    <div class="grid2">${field({ id: 'w-rate', key: 'w_ratePct', inputmode: 'decimal', value: v[KEYS.rate] ? ppmToPct(v[KEYS.rate]) : '', hintKey: 'w_rateHint', autocomplete: 'off' })}
      ${field({ id: 'w-fee', key: 'w_feePct', inputmode: 'decimal', value: v[KEYS.fee] ? ppmToPct(v[KEYS.fee]) : '', hintKey: 'w_feeHint', autocomplete: 'off' })}</div>
    ${switchEl('w-incl', (v[KEYS.incl] || 'true') === 'true', 'w_inclusive', 'w_inclusiveHint')}
    <p class="eyebrow mt-3" data-t="w_schedule"></p><p class="muted small" data-t="w_scheduleHint"></p>
    <div id="wRows"></div><div class="btn-row">${btn({ id: 'wAddRow', icon: 'plus', key: 'w_addChange' })}</div>
    <p class="eyebrow mt-3" data-t="w_fiscal"></p>
    ${rowDiv({ title: { t: fx.sendEnabled ? 'w_sendOn' : 'w_sendOff' }, sub: esc(t('w_sendOffHint')), trailing: pill(fx.sendEnabled ? 'ok' : 'warn', { key: fx.sendEnabled ? 'on' : 'off' }) })}
    ${input({ type: 'date', id: 'w-since', key: 'w_fiscalSince', value: since ? msToDay(since, tz()) : '' })}<p class="muted small" data-t="w_fiscalSinceHint"></p>
    <p class="eyebrow mt-3" data-t="w_notices"></p>
    ${switchEl('w-wa', v[KEYS.wa] === 'on', 'w_waStatus', 'w_waStatusHint')}
    ${field({ id: 'w-late', key: 'w_lateMin', inputmode: 'numeric', value: v[KEYS.late] || '20', hintKey: 'w_lateMinHint', autocomplete: 'off' })}
    <div class="btn-row">${btn({ id: 'wTaxSave', variant: 'primary', icon: 'check', key: 'save' })}</div>`;
  const drawRows = () => { $('#wRows').innerHTML = rows.map(rowHtml).join(''); paint($('#wRows')); };
  const readRows = () => rows.map((_, i) => ({ day: $(`[data-sd="${i}"]`, host).value, pct: $(`[data-sp="${i}"]`, host).value }));
  drawRows();
  paint(host);
  $('#wAddRow').onclick = () => { rows = [...readRows(), { day: '', pct: '' }]; drawRows(); };
  $('#wTaxSave').onclick = async () => {
    const w = writes({ rate: $('#w-rate').value, fee: $('#w-fee').value, incl: $('#w-incl').checked, rows: readRows(), since: $('#w-since').value,
      wa: $('#w-wa').checked, late: $('#w-late').value, tz: tz() });
    if (w.bad) return fail(`${t(w.bad)}${w.rows ? ' ' + w.rows.join(', ') : ''}`);
    try {
      await busy($('#wTaxSave'), async () => { for (const [key, value] of w.out) if ((v[key] ?? def[key] ?? '') !== value) await post('/owner/settings', { key, value }); });
      toast(t('saved')); mount(host);
    } catch (e) { fail(e); }
  };
}

export const openTax = () => mount(open('w_taxTitle', null, 'tax'));
