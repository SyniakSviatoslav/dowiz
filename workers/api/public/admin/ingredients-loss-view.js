// LOSSES, DRAWN (W-STOCK P4): the `avt` block of the kitchen numbers in,
// markup out. PURE: `money`, `t` and `when` are handed in, so it renders in
// node (`ingredients-loss-view.test.mjs`).
//
// Each loss opens to its records: the two counts and every delivery, sale,
// write-off and prep between them, so "where did it go" is a list, not a
// number. An ingredient counted once says "count it again"; one no recipe uses
// is listed as not tracked -- never as a loss.
//
// ASCII QUOTES ONLY in this file.

import { ui, pill } from './parts.js';

const esc = ui.esc;

/// Per mille as a percent with one decimal: 42 -> "4.2%".
export const pct = pm => (pm == null ? '' : `${(pm / 10).toFixed(1)}%`);

/// One loss: the sum that closes, then its records.
export function rowMarkup(r, { money, t, when }){
  const u = r.unit || '';
  const flags = (r.flags || []).map(f => pill(f === 'over30pm' ? 'bad' : 'warn', { key: f === 'over30pm' ? 'ls_over' : 'ls_unusual' })).join('');
  const sum = [['ls_opening', r.opening], ['ls_received', `+${r.received}`], ['ls_sold', `-${r.sold}`], ['ls_wasted', `-${r.wasted}`], ['ls_prep', `-${r.prep}`], ['ls_closing', r.closing]]
    .map(([k, v]) => `<tr><td data-t="${k}">${esc(t(k))}</td><td>${esc(String(v))} ${esc(u)}</td></tr>`).join('');
  const recs = (r.records || []).map(m => `<tr><td>${esc(when(m.at))}</td><td>${esc(t('inv_mv_' + m.kind))}${m.reason ? ' · ' + esc(t(m.reason)) : ''}</td><td>${esc(String(m.qty))}</td><td>${m.value != null ? money(m.value) : ''}</td></tr>`).join('');
  const lost = r.unexplained > 0;
  const value = r.value != null ? ` · <span class="${lost ? 'neg' : 'pos'}">${money(Math.abs(r.value))}</span>` : '';
  const share = r.revenuePm != null ? ` · ${pct(r.revenuePm)} <span data-t="ls_ofSales">${esc(t('ls_ofSales'))}</span>` : '';
  return `<details class="fold ls-row" data-loss="${esc(r.id)}"><summary><b>${esc(r.name)}</b> · ${lost ? '' : `<span data-t="ls_found">${esc(t('ls_found'))}</span> `}${Math.abs(r.unexplained)} ${esc(u)}${value}${share} ${flags}</summary>
    <div class="kt-wrap"><table class="kt"><tbody>${sum}<tr><td><b data-t="ls_unexplained">${esc(t('ls_unexplained'))}</b></td><td><b>${r.unexplained} ${esc(u)}</b></td></tr></tbody></table></div>
    <p class="eyebrow" data-t="ls_records">${esc(t('ls_records'))}</p>
    <div class="kt-wrap"><table class="kt"><tbody>${recs}</tbody></table></div></details>`;
}

/// The whole answer: the top losses, the rest, who must count twice, what is not tracked.
export function lossesMarkup(avt, fmt){
  const { t } = fmt;
  const rows = avt?.rows || [];
  const top = new Set(avt?.top || []);
  const lead = rows.filter(r => top.has(r.id)), rest = rows.filter(r => !top.has(r.id));
  const list = (title, xs) => (xs.length ? `<p class="eyebrow mt-3" data-t="${title}">${esc(t(title))}</p><p class="hint">${xs.map(x => esc(x.name)).join(', ')}</p>` : '');
  return (rows.length ? '' : `<p class="hint" data-t="ls_none">${esc(t('ls_none'))}</p>`) +
    (lead.length ? `<p class="eyebrow" data-t="ls_top">${esc(t('ls_top'))}</p>${lead.map(r => rowMarkup(r, fmt)).join('')}` : '') +
    rest.map(r => rowMarkup(r, fmt)).join('') +
    list('ls_twice', avt?.countTwice || []) + list('ls_untracked', avt?.notTracked || []);
}
