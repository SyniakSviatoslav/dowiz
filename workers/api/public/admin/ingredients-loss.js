// LOSSES (W-STOCK P4): the sheet. The unexplained loss between two counts of
// each ingredient, in money, biggest first -- the kitchen numbers' `avt`
// block (`GET /api/owner/analytics/kitchen?days=`, `kitchen/avt.rs`), drawn by
// `ingredients-loss-view.js`.

import { $, t, api, sheet, money, retranslate, day, clock } from '/admin/core.js';
import { chips, empty, loading } from '/admin/parts.js';
import { lossesMarkup } from '/admin/ingredients-loss-view.js';

const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '-');

export async function open(days = 30){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="ls_title"></h2><p class="sheet-hint" data-t="ls_hint"></p>
    ${chips({ id: 'lsDays', values: [{ value: 7, key: 'ka_7' }, { value: 30, key: 'ka_30' }, { value: 62, label: '62' }], value: days, attr: 'lsd', labelKey: 'ls_days' })}
    <div id="lsOut">${loading(3)}</div>`, { name: 'losses' });
  $('#lsDays').onclick = e => { const b = e.target.closest('[data-lsd]'); if (b) open(Number(b.dataset.lsd)); };
  try {
    const k = await api(`/owner/analytics/kitchen?days=${days}`);
    $('#lsOut').innerHTML = lossesMarkup(k.avt, { money, t, when });
    retranslate($('#lsOut'));
  } catch (e) { $('#lsOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); }
}
