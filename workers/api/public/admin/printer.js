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
    <div class="rows"><div class="rowc">${icon('receipt')}<span class="t"><b data-t="printer"></b><small class="mono">${esc(name || '-')}</small></span><span class="pill ${name ? 'ok' : ''}" data-t="${name ? 'prOn' : 'prOff'}"></span></div></div>
    <label for="pr-name" data-t="prName"></label><input id="pr-name" maxlength="40" autocomplete="off" value="${esc(name)}"><p class="hint" data-t="prNameHint"></p>
    <div class="btn-row"><button class="btn" id="prSave">${icon('check')}<span data-t="save"></span></button></div>
    <p class="eyebrow mt-3" data-t="prUrl"></p><div class="code" id="prUrl">${esc(url)}</div>
    <p class="eyebrow mt-3" data-t="prKey"></p><p class="hint" data-t="prKeyHint"></p>
    <div class="btn-row"><button class="btn ghost" id="prKey">${icon('key')}<span data-t="prNewKey"></span></button></div><div id="prKeyOut"></div>
    <p class="eyebrow mt-3" data-t="prJobs"></p>
    ${jobs.length ? `<div class="rows">${jobs.map(x => `<div class="rowc">${icon(x.state === 'failed' ? 'alert-triangle' : 'receipt')}<span class="t"><b>#${esc(String(x.orderId || '').slice(0, 8))}</b><small class="mono">${esc(t('print_' + x.state))}${x.tries ? ' · ' + esc(x.tries) : ''}${x.code ? ' · ' + esc(x.code) : ''}</small></span></div>`).join('')}</div>` : `<p class="muted small" data-t="prNoJobs"></p>`}`,
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
