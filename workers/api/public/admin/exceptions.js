// The exception report (P1-5, BLUEPRINT-OPERATIONAL-BLIND-SPOTS section 2.5):
// voids after the kitchen, comps, late amendments, refunds, cash outside a
// till, pay-outs and over/short -- one row per signed event, with the name of
// who signed it. Read-only: GET /api/owner/exceptions.
//
// NOTHING HERE ADDS UP A PERSON. The groups are by kind and reason; a name is
// only ever on its own row (DECISIONS OD-8: trust is a signed capability,
// never a score).
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console. No apostrophes inside the words below, for the same reason.

import { esc, icon, t, api, toast, sheet, money, store, clock, day } from '/admin/core.js';
import { T, LANGS, intlLocale } from '/admin/i18n.js';
import * as Money from '/lib/money.js';

const WORDS = {
  sq: { exceptions: 'Përjashtimet', exHint: 'Çdo anulim pas kuzhinës, dhuratë, rimbursim, pagesë nga arka dhe diferencë arke, me emrin e kujt e bëri. Vetëm lexim.', exDay: '24 orët e fundit', exTill: 'Periudha e arkës', exNone: 'Asnjë përjashtim në këtë periudhë.', exGroups: 'Sipas llojit dhe arsyes', exRows: 'Ngjarjet', exBy: 'nga', exThreshold: 'Njoftim në Telegram pas', exLate: 'Ndryshim i vonë pas (min)',
        ex_void_after_kitchen: 'Anulim pas kuzhinës', ex_comp: 'Dhuratë', ex_late_amendment: 'Ndryshim i vonë', ex_refund: 'Rimbursim', ex_cash_outside_till: 'Para jashtë arkës', ex_pay_out: 'Pagesë nga arka', ex_over_short: 'Diferencë arke' },
  en: { exceptions: 'Exceptions', exHint: 'Every void after the kitchen, comp, refund, pay-out and till difference, with the name of who did it. Read-only.', exDay: 'Last 24 hours', exTill: 'Till period', exNone: 'No exceptions in this period.', exGroups: 'By kind and reason', exRows: 'Events', exBy: 'by', exThreshold: 'Telegram alert after', exLate: 'Late amendment after (min)',
        ex_void_after_kitchen: 'Void after kitchen', ex_comp: 'Comp', ex_late_amendment: 'Late amendment', ex_refund: 'Refund', ex_cash_outside_till: 'Cash outside a till', ex_pay_out: 'Pay-out', ex_over_short: 'Till over/short' },
  uk: { exceptions: 'Винятки', exHint: 'Кожне скасування після кухні, комплімент, повернення, виплата з каси та розбіжність каси, з іменем того, хто це зробив. Лише читання.', exDay: 'Останні 24 години', exTill: 'Період каси', exNone: 'У цьому періоді винятків немає.', exGroups: 'За видом і причиною', exRows: 'Події', exBy: 'від', exThreshold: 'Сповіщення в Telegram після', exLate: 'Пізня зміна після (хв)',
        ex_void_after_kitchen: 'Скасування після кухні', ex_comp: 'Комплімент', ex_late_amendment: 'Пізня зміна', ex_refund: 'Повернення', ex_cash_outside_till: 'Готівка поза касою', ex_pay_out: 'Виплата з каси', ex_over_short: 'Розбіжність каси' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="analytics"></p><h2 data-t="exceptions"></h2><p class="muted small" data-t="exHint"></p>`;
/// An amount IN THE CURRENCY IT WAS TAKEN IN, never converted: a till pile of
/// euro is shown in euro. No currency on the row = the venue formatter.
const amount = (n, cur) => esc(cur ? Money.formatter({ base: cur, display: cur, rates: null, locale: intlLocale() })(n) : money(n));
const when = ms => `${esc(day(ms))} ${esc(clock(ms))}`;

export async function open(period = ''){
  sheet(`${head}<div id="exBody"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>`, { name: 'exceptions', keepScroll: true });
  const q = '?location_id=' + encodeURIComponent(store.loc || '') + (period ? '&period=' + period : '');
  let d;
  try { d = await api('/owner/exceptions' + q); } catch (e) { return fail(e); }
  const rows = d.rows || [], groups = d.groups || [];
  const tabs = `<div class="chips"><button class="chip ${period ? '' : 'on'}" data-period="" data-t="exDay"></button><button class="chip ${period ? 'on' : ''}" data-period="till" data-t="exTill"></button></div>`;
  const facts = `<p class="muted small"><span data-t="exThreshold"></span> ${esc(d.threshold)} · <span data-t="exLate"></span> ${esc(d.lateMinutes)}</p>`;
  const groupRows = groups.map(g => `<div class="rowc">${icon('alert-triangle')}<span class="t"><b>${esc(t('ex_' + g.kind))}</b><small>${esc(g.reason || '-')}</small></span>
    <span class="mono">${esc(g.count)} · ${Object.entries(g.amount || {}).map(([c, n]) => amount(n, c)).join(' / ')}</span></div>`).join('');
  const eventRows = rows.slice().reverse().map(r => `<div class="rowc">${icon('receipt')}<span class="t"><b>${esc(t('ex_' + r.kind))}</b>
    <small class="mono">${when(r.at)} · ${esc(r.order_id || r.till_id || '-')} · ${esc(r.reason || '-')} · <span data-t="exBy"></span> ${esc(r.by || '-')}</small></span>
    <span class="mono">${amount(r.amount, r.currency)}</span></div>`).join('');
  const body = rows.length
    ? `<section class="group mt-3"><p class="eyebrow" data-t="exGroups"></p><div class="rows">${groupRows}</div></section>
       <section class="group mt-3"><p class="eyebrow" data-t="exRows"></p><div class="rows">${eventRows}</div></section>`
    : `<p class="muted small mt-3" data-t="exNone"></p>`;
  sheet(`${head}<div id="exBody">${tabs}${facts}${body}</div>`, { name: 'exceptions', keepScroll: true });
  for (const b of document.querySelectorAll('#exBody [data-period]')) b.onclick = () => open(b.dataset.period);
}
