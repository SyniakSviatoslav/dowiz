// OFFLINE SALES (W-OFFSALE, row OF3): the cash sales a room tablet made with
// no network, as the hub recorded them when they synced -- how many, the
// oldest, which are past their 48 h fiscal deadline (RED), and what changed
// while the tablet was offline (a price, a dish taken off sale): the sale kept
// the price the guest paid, and the change is shown here.
//
// `GET /api/owner/offline_sales` (offline_sale::ledger::Pane). Sending to the
// tax authority is the platform's switch (`fiscal::SEND_ENABLED`, OFF); the
// pane says so from the answer, never from a sentence somebody keeps true.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { esc, icon, t, api, toast, sheet, day, clock, store, moneyEl } from '/admin/core.js';
import { retranslate } from '/admin/i18n.js';
import { rowDiv, pill, loading, empty } from '/admin/parts.js';
import '/admin/offline-i18n.js';

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const head = `<p class="eyebrow" data-t="settings"></p><h2 data-t="of_title"></h2><p class="sheet-hint" data-t="of_hint"></p>`;
const when = ms => (ms ? `${day(ms)} ${clock(ms)}` : '');
const word = (k, rest = '') => `<span data-t="${esc(k)}">${esc(t(k))}</span>${rest}`;

/// One sale's row. PURE given `t`/`day`/`clock`: an overdue sale is red
/// (`of-overdue`, the danger icon and pill), its conflicts named.
export function saleRow(s){
  const conflicts = (s.conflicts || []).map(c => word('of_c_' + c.kind, c.product_id ? ` ${esc(c.product_id)}` : '')).join(', ');
  const sub = [`${word('of_sold')} ${esc(when(s.sold_at_ms))}`, `${word('of_deadline')} ${esc(when(s.deadline_ms))}`,
    word('of_fiscal_' + s.fiscal), s.alerted ? word('of_alerted') : '', conflicts].filter(Boolean).join(' · ');
  return rowDiv({ cls: s.overdue ? 'of-overdue' : '', leading: icon(s.overdue ? 'alert-triangle' : 'receipt'),
    title: `${String(s.order_id).replace(/^offline:/, '').slice(0, 8)}`, sub, tour: 'offline.sale', data: { order: s.order_id },
    trailing: `${moneyEl(s.total)}${s.overdue ? ' ' + pill('bad', { key: 'of_overdue' }) : ''}` });
}

/// The whole pane from the answer. PURE given the same.
export function paneHtml(d){
  const items = d.items || [];
  const summary = `<div class="rows" data-tour="offline.summary">
    ${rowDiv({ leading: icon('receipt'), title: { t: 'of_count' }, trailing: `<b>${esc(d.count || 0)}</b>` })}
    ${rowDiv({ leading: icon('clock'), title: { t: 'of_oldest' }, sub: esc(when(d.oldest_sold_at_ms)) })}
    ${rowDiv({ cls: d.overdue ? 'of-overdue' : '', leading: icon('alert-triangle'), title: { t: 'of_overdue' },
      trailing: d.overdue ? pill('bad', { label: String(d.overdue) }) : pill('ok', { label: '0' }) })}
    ${rowDiv({ leading: icon('alert-circle'), title: { t: 'of_conflicts' }, trailing: `<b>${esc(d.conflicted || 0)}</b>` })}
    ${d.send_enabled ? '' : rowDiv({ leading: icon('plug-connected-x'), title: { t: 'of_sendOff' } })}
  </div>`;
  const list = items.length ? `<div class="rows" role="list" data-tour="offline.list">${items.map(saleRow).join('')}</div>` : empty('receipt', { key: 'of_none' });
  return `${summary}<section class="group mt-3"><p class="eyebrow" data-t="of_title"></p>${list}</section>`;
}

/// Open the pane (the W-WIRE tile `offlineSales`).
export async function openOfflineSales(){
  sheet(`${head}<div id="ofBody">${loading(3)}</div>`, { name: 'offline', keepScroll: true });
  let d;
  try { d = await api('/owner/offline_sales' + q()); } catch (e) { return toast(String(e.message || e)); }
  sheet(`${head}<div id="ofBody">${paneHtml(d)}</div>`, { name: 'offline', keepScroll: true });
  retranslate(document.getElementById('sheetIn') || document);
}
