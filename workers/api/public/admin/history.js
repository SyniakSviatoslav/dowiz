// OLDER ORDERS (W-WIRE row 4). The nightly rotation moves finished orders
// older than the hot window out of the live log into an archive; the Orders
// screen folds only the live log, so without this screen an owner could not
// see last month. `GET /api/owner/history` lists the archives; one is loaded
// with `?archive=`, and a date range narrows it. Read-only.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, icon, t, api, moneyEl, day, clock } from '/admin/core.js';
import { input, rowDiv, empty, loading, chips } from '/admin/parts.js';
import { q, tz, fail, open, paint } from '/admin/wire-core.js';
import { inRange, total, archivesNewestFirst } from '/admin/wire-logic.js';

const DAY_MS = 86_400_000;

export async function mount(host){
  if (!host) return;
  host.innerHTML = loading(2);
  let d;
  try { d = await api('/owner/history' + q()); } catch (e) { host.innerHTML = empty('alert-triangle', { title: String(e.message || e), alert: true }); return; }
  const ids = archivesNewestFirst(d.archives);
  if (!ids.length) {
    host.innerHTML = empty('history', { key: 'w_noArchives', body: `${t('w_noArchivesHint')} ${Math.round((d.keepMs || 0) / DAY_MS)} ${t('w_days')}.` });
    return paint(host);
  }
  host.innerHTML = `<p class="muted small">${esc(t('w_keepHint'))} ${Math.round((d.keepMs || 0) / DAY_MS)} ${esc(t('w_days'))}.</p>
    ${chips({ id: 'wArch', values: ids.map((id, i) => ({ value: id, label: `${t('w_archive')} ${ids.length - i}` })), value: ids[0], attr: 'arch', labelKey: 'w_olderOrders' })}
    <div class="grid2">${input({ type: 'date', id: 'w-from', key: 'w_from' })}${input({ type: 'date', id: 'w-to', key: 'w_to' })}</div>
    <div id="wOrders"></div>`;
  paint(host);
  let orders = [], shown = ids[0];
  const draw = () => {
    const list = inRange(orders, $('#w-from').value, $('#w-to').value, tz());
    $('#wOrders').innerHTML = `${rowDiv({ leading: icon('receipt'), title: `${list.length} ${t('w_ordersN')}`, trailing: moneyEl(total(list)) })}
      <div class="rows" role="list">${list.slice(0, 200).map(o => rowDiv({ leading: icon('receipt'), title: '#' + String(o.id || '').slice(0, 8),
        sub: `${esc(day(o.created_at_ms))} ${esc(clock(o.created_at_ms))} · ${esc(o.status || '')}`, trailing: moneyEl(o.total || 0) })).join('')}</div>`;
    paint($('#wOrders'));
  };
  const load = async id => {
    shown = id;
    $('#wOrders').innerHTML = loading(3);
    try { orders = (await api(`/owner/history${q()}&archive=${encodeURIComponent(id)}`)).orders || []; } catch (e) { orders = []; fail(e); }
    if (shown === id) draw();
  };
  for (const c of $$('[data-arch]', host)) c.onclick = () => { for (const x of $$('[data-arch]', host)) x.setAttribute('aria-pressed', String(x === c)); load(c.dataset.arch); };
  $('#w-from').onchange = draw; $('#w-to').onchange = draw;
  load(ids[0]);
}

export const openHistory = () => mount(open('w_olderOrders', 'w_olderOrdersHint', 'history'));
