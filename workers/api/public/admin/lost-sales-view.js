// ORDERS LOST TO STOCK-OUTS, DRAWN (A13, W-LOST): the `lost` block of the
// kitchen's numbers (`GET /api/owner/analytics/kitchen`, stock.refused.v1) in,
// markup out. PURE -- `money` is handed in, nothing is fetched -- so it
// renders in node (`lost-sales-view.test.mjs`). The words are `data-t` keys
// (`lost-sales-words.js`); every number is the hub's.
//
// A KITCHEN TOKEN gets the block without `revenue` (the hub strips it), and
// the revenue column is then not drawn rather than drawn empty.
//
// ASCII QUOTES ONLY in this file.

import { ui } from './parts.js';

const esc = ui.esc;
const th = keys => `<thead><tr>${keys.map(x => `<th data-t="${x}"></th>`).join('')}</tr></thead>`;
const tbl = (head, rows) => `<div class="kt-wrap"><table class="kt">${th(head)}<tbody>${rows}</tbody></table></div>`;
const cell = v => `<td>${v == null ? '-' : v}</td>`;

/// Does this answer carry lek? (an owner's does; a kitchen token's does not)
export const hasRevenue = lost => !!lost && lost.revenue != null;

/// The card, or '' when the answer has no `lost` block (an older hub).
export function drawLost(r, { money = n => String(n) } = {}){
  const lost = r && r.lost;
  if (!lost) return '';
  const lek = hasRevenue(lost);
  const head = `<p class="eyebrow" data-t="ls_title"></p><p class="hint" data-t="ls_hint"></p>`;
  if (!lost.rows) return `<div class="group" data-tour="kitchen.lost">${head}<p class="hint" data-t="ls_none"></p></div>`;
  const stat = (v, word) => `<div class="stat"><b>${v}</b><small data-t="${word}"></small></div>`;
  const stats = `<div class="stats kt-tot">${stat(lost.rows, 'ls_refused')}${stat(lost.portions, 'ls_portions')}${lek ? stat(money(lost.revenue), 'ls_revenue') : ''}</div>`;
  const dishHead = ['ls_dish', 'ls_refused', 'ls_portions', ...(lek ? ['ls_revenue'] : [])];
  const dishes = (lost.dishes || []).map(d => `<tr><td>${esc(d.name)}</td>${cell(d.rows)}${cell(d.portions)}${lek ? cell(money(d.revenue)) : ''}</tr>`).join('');
  const days = (lost.byDay || []).filter(d => d.rows).map(d => `<tr><td>${esc(d.day)}</td>${cell(d.rows)}${cell(d.portions)}${lek ? cell(money(d.revenue)) : ''}</tr>`).join('');
  const dayHead = ['ls_day', 'ls_refused', 'ls_portions', ...(lek ? ['ls_revenue'] : [])];
  return `<div class="group" data-tour="kitchen.lost">${head}${stats}${tbl(dishHead, dishes)}${tbl(dayHead, days)}<p class="hint" data-t="ls_limit"></p></div>`;
}
