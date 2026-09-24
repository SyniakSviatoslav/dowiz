// THE KITCHEN PRINTER (BLUEPRINT-LAST-MILE section 3.1): a CloudPRNT-style
// printer that PHONES US. The whole rail was built -- the outbox entry per
// order, /api/print/poll, the job, the DELETE ack, the queued/printing/
// printed/failed marks on the orders list -- and `print.kitchen`, the ONE
// setting that switches it on, had no control anywhere in the console, so no
// venue could ever get a ticket and the jobs view was always empty
// (2026-09-24). This sheet is that control, plus the two facts the printer's
// own setup screen asks for: the server URL, and a venue key as its password.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, esc, icon, t, api, post, toast, sheet, busy } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { btn, field, pill, rowDiv } from '/admin/parts.js';

const WORDS = {
  sq: { prHint: 'Kur ka nje emer, cdo porosi e re del edhe si flete per kuzhinen. Printeri e merr vete nga adresa me poshte.', prName: 'Emri i printerit', prNameHint: 'P.sh. kuzhina. Bosh = pa printer, pa flete.', prUrl: 'Adresa e serverit (CloudPRNT)', prKey: 'Fjalekalimi i printerit', prKeyHint: 'Nje celes API i lokalit, vetem per printerin. Shfaqet nje here.', prNewKey: 'Krijo celes per printerin', prJobs: 'Fletet ne radhe', prNoJobs: 'Asnje flete ne radhe.', prOn: 'ndezur', prOff: 'fikur' },
  en: { prHint: 'With a name set, every new order is also queued as a ticket for the kitchen. The printer fetches it itself from the address below.', prName: 'Printer name', prNameHint: 'E.g. kitchen. Empty = no printer, no tickets.', prUrl: 'Server URL (CloudPRNT)', prKey: 'Printer password', prKeyHint: 'A venue API key, for the printer only. Shown once.', prNewKey: 'Make a key for the printer', prJobs: 'Tickets in the queue', prNoJobs: 'No ticket in the queue.', prOn: 'on', prOff: 'off' },
  uk: { prHint: 'Коли ім\'я задано, кожне нове замовлення стає ще й чеком для кухні. Принтер забирає його сам за адресою нижче.', prName: 'Назва принтера', prNameHint: 'Напр. kitchen. Порожньо = без принтера, без чеків.', prUrl: 'Адреса сервера (CloudPRNT)', prKey: 'Пароль принтера', prKeyHint: 'API-ключ закладу, лише для принтера. Показується один раз.', prNewKey: 'Створити ключ для принтера', prJobs: 'Чеки в черзі', prNoJobs: 'У черзі немає чеків.', prOn: 'увімкнено', prOff: 'вимкнено' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// `print_rail::SETTING`.
const SETTING = 'print.kitchen';
const fail = e => toast(String(e.message || e));

export async function open(){
  let s = { values: {} }, j = { jobs: [] };
  try { [s, j] = await Promise.all([api('/owner/settings'), api('/owner/print/jobs')]); } catch (e) { return fail(e); }
  const name = (s.values || {})[SETTING] || '';
  const url = `${location.origin}/api/print/poll`;
  const jobs = j.jobs || [];
  sheet(`<p class="eyebrow" data-t="settings"></p><h2 data-t="printer"></h2><p class="muted small" data-t="prHint"></p>
    <div class="rows">${rowDiv({ leading: icon('receipt'), title: { t: 'printer' }, sub: `<span class="mono">${esc(name || '-')}</span>`, trailing: pill(name ? 'ok' : '', { key: name ? 'prOn' : 'prOff' }), tour: 'printer.state' })}</div>
    ${field({ id: 'pr-name', key: 'prName', maxlength: 40, autocomplete: 'off', value: name, hintKey: 'prNameHint', tour: 'printer.name' })}
    <div class="btn-row">${btn({ id: 'prSave', variant: 'primary', icon: 'check', key: 'save', tour: 'printer.save' })}</div>
    <p class="eyebrow mt-3" data-t="prUrl"></p><div class="code" id="prUrl" data-tour="printer.url">${esc(url)}</div>
    <p class="eyebrow mt-3" data-t="prKey"></p><p class="hint" data-t="prKeyHint"></p>
    <div class="btn-row">${btn({ id: 'prKey', icon: 'key', key: 'prNewKey', tour: 'printer.key' })}</div><div id="prKeyOut"></div>
    <p class="eyebrow mt-3" data-t="prJobs"></p>
    ${jobs.length ? `<div class="rows" data-tour="printer.jobs">${jobs.map(x => rowDiv({ leading: icon(x.state === 'failed' ? 'alert-triangle' : 'receipt'), title: '#' + String(x.orderId || '').slice(0, 8),
      sub: `<span class="mono">${esc(t('print_' + x.state))}${x.tries ? ' · ' + esc(x.tries) : ''}${x.code ? ' · ' + esc(x.code) : ''}</span>` })).join('')}</div>` : `<p class="muted small" data-t="prNoJobs"></p>`}`,
    { name: 'printer', keepScroll: true });
  $('#prSave').onclick = async () => {
    const v = $('#pr-name').value.trim();
    // An empty value CLEARS the setting on the hub: the printer is off.
    try { await busy($('#prSave'), () => post('/owner/settings', { key: SETTING, value: v })); toast(t('saved')); open(); } catch (e) { fail(e); }
  };
  $('#prKey').onclick = async () => {
    try {
      const d = await busy($('#prKey'), () => post('/owner/apikeys', { label: 'printer' }));
      $('#prKeyOut').innerHTML = `<div class="code">${esc(d.key || d.token || '')}</div><p class="hint" data-t="keyOnce"></p>`;
      $('#prKeyOut [data-t]').textContent = t('keyOnce');
    } catch (e) { fail(e); }
  };
}
