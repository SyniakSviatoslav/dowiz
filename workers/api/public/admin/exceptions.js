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

import { $, esc, icon, t, api, post, toast, sheet, money, store, clock, day, busy } from '/admin/core.js';
import { T, LANGS, intlLocale } from '/admin/i18n.js';
import * as Money from '/lib/money.js';

const WORDS = {
  sq: { exceptions: 'Përjashtimet', exHint: 'Çdo anulim pas kuzhinës, dhuratë, rimbursim, pagesë nga arka dhe diferencë arke, me emrin e kujt e bëri. Vetëm lexim.', exDay: '24 orët e fundit', exTill: 'Periudha e arkës', exNone: 'Asnjë përjashtim në këtë periudhë.', exGroups: 'Sipas llojit dhe arsyes', exRows: 'Ngjarjet', exBy: 'nga', exThreshold: 'Njoftim në Telegram pas', exLate: 'Ndryshim i vonë pas (min)', exNoChat: 'Asnjë bisedë Telegram: njoftimet nuk shkojnë askund. Vendoseni te Njoftimet.', exChatOk: 'Njoftimet shkojnë në bisedën tuaj Telegram.', exOff: '0 = pa njoftime', legs: 'Portofoli kundrejt pagesave', legsOk: 'Çdo pagesë me portofol ka zbritjen e saj.', legsMissing: 'Zbritje që mungojnë', legsCheck: 'Kontrollo riparimin', legsApply: 'Shkruaj zbritjet që mungojnë', legsWould: 'Do të shkruheshin', legsRefused: 'Të refuzuara',
        ex_void_after_kitchen: 'Anulim pas kuzhinës', ex_comp: 'Dhuratë', ex_late_amendment: 'Ndryshim i vonë', ex_refund: 'Rimbursim', ex_cash_outside_till: 'Para jashtë arkës', ex_pay_out: 'Pagesë nga arka', ex_over_short: 'Diferencë arke' },
  en: { exceptions: 'Exceptions', exHint: 'Every void after the kitchen, comp, refund, pay-out and till difference, with the name of who did it. Read-only.', exDay: 'Last 24 hours', exTill: 'Till period', exNone: 'No exceptions in this period.', exGroups: 'By kind and reason', exRows: 'Events', exBy: 'by', exThreshold: 'Telegram alert after', exLate: 'Late amendment after (min)', exNoChat: 'No Telegram chat: alerts go nowhere. Set one under Notifications.', exChatOk: 'Alerts go to your Telegram chat.', exOff: '0 = no alerts', legs: 'Wallet against payments', legsOk: 'Every wallet payment has its debit.', legsMissing: 'Missing debits', legsCheck: 'Check the repair', legsApply: 'Write the missing debits', legsWould: 'Would write', legsRefused: 'Refused',
        ex_void_after_kitchen: 'Void after kitchen', ex_comp: 'Comp', ex_late_amendment: 'Late amendment', ex_refund: 'Refund', ex_cash_outside_till: 'Cash outside a till', ex_pay_out: 'Pay-out', ex_over_short: 'Till over/short' },
  uk: { exceptions: 'Винятки', exHint: 'Кожне скасування після кухні, комплімент, повернення, виплата з каси та розбіжність каси, з іменем того, хто це зробив. Лише читання.', exDay: 'Останні 24 години', exTill: 'Період каси', exNone: 'У цьому періоді винятків немає.', exGroups: 'За видом і причиною', exRows: 'Події', exBy: 'від', exThreshold: 'Сповіщення в Telegram після', exLate: 'Пізня зміна після (хв)', exNoChat: 'Немає чату Telegram: сповіщення нікуди не йдуть. Вкажіть його в Сповіщеннях.', exChatOk: 'Сповіщення йдуть у ваш чат Telegram.', exOff: '0 = без сповіщень', legs: 'Гаманець проти оплат', legsOk: 'Кожна оплата гаманцем має своє списання.', legsMissing: 'Відсутні списання', legsCheck: 'Перевірити ремонт', legsApply: 'Записати відсутні списання', legsWould: 'Буде записано', legsRefused: 'Відмовлено',
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
  // THE OWNER'S TWO NUMBERS, editable where they are read (they were only
  // printed: `alerts.exceptions.*` had no control anywhere in the console).
  const facts = `<div class="grid2"><div><label for="ex-thr" data-t="exThreshold"></label><input id="ex-thr" inputmode="numeric" value="${esc(d.threshold)}"><small class="muted" data-t="exOff"></small></div>
    <div><label for="ex-late" data-t="exLate"></label><input id="ex-late" inputmode="numeric" value="${esc(d.lateMinutes)}"></div></div>
    <div class="btn-row"><button class="btn ghost" id="exSave">${icon('check')}<span data-t="save"></span></button></div>
    <p class="${d.alertChat ? 'muted' : 'warn'} small" data-t="${d.alertChat ? 'exChatOk' : 'exNoChat'}"></p>`;
  const who = id => (d.names || {})[id] || id || '-';
  const groupRows = groups.map(g => `<div class="rowc">${icon('alert-triangle')}<span class="t"><b>${esc(t('ex_' + g.kind))}</b><small>${esc(g.reason || '-')}</small></span>
    <span class="mono">${esc(g.count)} · ${Object.entries(g.amount || {}).map(([c, n]) => amount(n, c)).join(' / ')}</span></div>`).join('');
  const eventRows = rows.slice().reverse().map(r => `<div class="rowc">${icon('receipt')}<span class="t"><b>${esc(t('ex_' + r.kind))}</b>
    <small class="mono">${when(r.at)} · ${esc(r.order_id || r.till_id || '-')} · ${esc(r.reason || '-')} · <span data-t="exBy"></span> ${esc(who(r.by))}</small></span>
    <span class="mono">${amount(r.amount, r.currency)}</span></div>`).join('');
  const body = rows.length
    ? `<section class="group mt-3"><p class="eyebrow" data-t="exGroups"></p><div class="rows">${groupRows}</div></section>
       <section class="group mt-3"><p class="eyebrow" data-t="exRows"></p><div class="rows">${eventRows}</div></section>`
    : `<p class="muted small mt-3" data-t="exNone"></p>`;
  sheet(`${head}<div id="exBody">${tabs}${facts}${body}<div id="exLegs"></div></div>`, { name: 'exceptions', keepScroll: true });
  for (const b of document.querySelectorAll('#exBody [data-period]')) b.onclick = () => open(b.dataset.period);
  $('#exSave').onclick = () => saveNumbers(period);
  legs();
}

/// `alerts.exceptions.threshold` / `.late_min`: whole numbers, or refused here.
async function saveNumbers(period){
  const thr = $('#ex-thr').value.trim(), late = $('#ex-late').value.trim();
  if (!/^[0-9]{1,4}$/.test(thr) || !/^[0-9]{1,4}$/.test(late) || Number(late) < 1) return toast(t('exThreshold') + ' / ' + t('exLate') + ': 0-9999');
  try {
    await busy($('#exSave'), async () => {
      await post('/owner/settings', { key: 'alerts.exceptions.threshold', value: thr });
      await post('/owner/settings', { key: 'alerts.exceptions.late_min', value: late });
    });
    toast(t('saved')); open(period);
  } catch (e) { fail(e); }
}

/// LAW 12 ON SCREEN: `GET /owner/wallet/legs` and its repair, which had a
/// route and no button. The repair is a DRY RUN first; only the second tap
/// writes, and the hub re-decides it against the ledger as it stands.
async function legs(){
  let a; try { a = await api('/owner/wallet/legs'); } catch (e) { $('#exLegs').innerHTML = `<p class="warn small">${esc(e.message || e)}</p>`; return; }
  const au = a.audit || {}, missing = au.missing || [];
  const bad = missing.length + (au.mismatched || []).length + (au.orphans || []).length;
  $('#exLegs').innerHTML = `<section class="group mt-3"><p class="eyebrow">${esc(t('legs'))}</p>
    ${a.holds ? `<p class="muted small">${esc(t('legsOk'))} (${esc((a.spends || []).length)})</p>` : `<div class="rows">${missing.map(l => `<div class="rowc">${icon('wallet')}<span class="t"><b>${esc(t('legsMissing'))}</b><small class="mono">${esc(l.order_id)} · ${esc(l.wallet)}</small></span><span class="mono">${amount(l.amount, l.currency)}</span></div>`).join('')}
      ${[...(au.mismatched || []), ...(au.orphans || [])].map(x => `<div class="rowc">${icon('alert-triangle')}<span class="t"><small class="mono">${esc(x)}</small></span></div>`).join('')}</div>
      <p class="muted small" id="exLegsOut"></p>
      <div class="btn-row"><button class="btn ghost" id="exLegsCheck">${icon('check')}<span>${esc(t('legsCheck'))}</span></button>${missing.length ? `<button class="btn" id="exLegsApply">${icon('wallet')}<span>${esc(t('legsApply'))}</span></button>` : ''}</div>`}</section>`;
  if (!bad) return;
  const say = r => { $('#exLegsOut').textContent = `${t('legsWould')}: ${(r.wouldWrite || r.written || []).length} · ${t('legsRefused')}: ${(r.refused || []).length}`; };
  $('#exLegsCheck').onclick = async () => { try { say(await busy($('#exLegsCheck'), () => post('/owner/wallet/legs/repair', {}))); } catch (e) { fail(e); } };
  const ap = $('#exLegsApply');
  if (ap) ap.onclick = async () => { try { say(await busy(ap, () => post('/owner/wallet/legs/repair', { apply: true }))); toast(t('saved')); legs(); } catch (e) { fail(e); } };
}
