// FOR THE WEEK (operator 2026-11-23): what to buy for the coming week from
// the average weekly use of the last four whole weeks. Every number is the
// hub's: `GET /api/owner/analytics/kitchen` `week` (`analytics/kitchen/
// report/week.rs`); `stock-week-logic.js` only groups and words it.
//
// Said, never hidden: a stock nobody counted (or one below zero) is not
// subtracted, so the row shows the week's need and says why; a row with no
// price says "no price", never 0; and the share of sales whose dish has no
// recipe -- they draw nothing -- is printed first, so the list reads as a
// floor. A member of the kitchen gets the same list without revenue (the hub
// strips it, `access::numbers_for_kitchen`).
//
// ASCII QUOTES ONLY in this file.

import '/admin/stock-week-i18n.js';
import { $, esc, t, api, sheet, busy, toast, money, S } from '/admin/core.js';
import { btn, pill, rowDiv, empty, loading } from '/admin/parts.js';
import { shareOrCopy, qtyText } from '/admin/order-list-logic.js';
import * as W from '/admin/stock-week-logic.js';

/// One ingredient: weekly average, free stock (or why it is not counted), to buy, est. cost.
function rowOf(r){
  const u = r.unit || '';
  const free = r.stockKnown ? `${t('wp_free')} ${qtyText(r.available, u)}`
    : pill('warn', { key: r.counted ? 'wp_belowZero' : 'wp_notCounted' });
  const facts = [`${qtyText(r.weekly ?? 0, u)} ${t('wp_weekly')}`, `${r.leadDays} ${t('wp_lead')}`, r.supplier ? esc(r.supplier) : ''].filter(Boolean).join(' · ');
  const qty = W.qtyOf(r);
  const buy = r.stockKnown ? `${t('wp_toBuy')} <b class="mono">${esc(qtyText(qty, u))}</b>` : `${t('wp_need')} <b class="mono">${esc(qtyText(qty, u))}</b>`;
  const cost = r.cost != null ? money(r.cost) : (qty > 0 ? pill('info', { key: 'wp_noPrice' }) : '');
  return rowDiv({ title: r.name, sub: `<span class="mono">${facts}</span> ${free}<br>${buy}${cost ? ` · ${cost}` : ''}`, data: { wp: r.id } });
}

/// The share of sales no recipe draws for, and the top dishes.
function coverageOf(cov){
  const share = W.shareText(cov);
  if (!share) return '';
  const top = (cov.top || []).map(d => `<li>${esc(d.name)} · ${d.sold}${d.revenue != null ? ` · ${money(d.revenue)}` : ''}</li>`).join('');
  return `<section class="group"><p class="warn small"><b>${esc(share)}</b> ${t('wp_floor')}</p>
    ${top ? `<p class="eyebrow">${t('wp_top')}</p><ul class="small">${top}</ul>` : ''}</section>`;
}

export async function openWeek(){
  sheet(`<p class="eyebrow" data-t="inv_title"></p><h2 data-t="wp_title"></h2><p class="sheet-hint" data-t="wp_hint"></p>
    <div id="wpOut">${loading(3)}</div>`, { name: 'weekPurchase' });
  let k;
  try { k = await api('/owner/analytics/kitchen?days=1&v=2'); } catch (e) { $('#wpOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(e.message || e), alert: true }); return; }
  const wk = k?.week;
  if (!wk || wk.error) { $('#wpOut').innerHTML = empty('alert-triangle', { key: 'loadFail', body: String(wk?.error || ''), alert: true }); return; }
  const period = `<p class="small">${t('wp_period')} ${esc(wk.from)} – ${esc(wk.to)}: ${esc(W.dataText(wk, t))}</p>`;
  const hist = wk.coverage?.historyError ? `<p class="warn small">${esc(wk.coverage.historyError)}</p>` : '';
  const rows = wk.rows || [];
  const tot = wk.totals || {};
  const total = tot.cost != null || tot.unpriced ? `<p class="small">${tot.cost != null ? `${t('wp_total')}: <b>${money(tot.cost)}</b>` : ''}${tot.unpriced ? ` · ${tot.unpriced} ${t('wp_unpriced')}` : ''}</p>` : '';
  const groups = W.groups(wk);
  const copies = groups.map((g, i) => btn({ variant: i ? 'ghost' : 'primary', icon: 'copy', label: `${t('wp_copy')}: ${g.supplier || t('wp_noSupplier')}`, data: { wpg: String(i) } })).join('');
  $('#wpOut').innerHTML = `${period}${hist}${coverageOf(wk.coverage)}
    ${rows.length ? `<div class="rows">${rows.map(rowOf).join('')}</div>` : empty('check', { key: wk.dataDays ? 'wp_empty' : 'wp_noData' })}
    ${total}${copies ? `<div class="btn-row">${copies}</div>` : ''}`;
  $('#wpOut').onclick = async e => {
    const b = e.target.closest('[data-wpg]'); if (!b) return;
    const text = W.supplierText(groups[Number(b.dataset.wpg)], S.venue?.name, t);
    try {
      await busy(b, async () => {
        const how = await shareOrCopy(text, navigator);
        if (how === 'failed') throw new Error(text);
        if (how === 'copied') toast(t('wp_copied'));
      });
    } catch (err) { toast(String(err.message || err)); }
  };
}
