// WHAT THE NOTES SAY, PER DISH PER WEEK (P16b): the hub's `topics` (from
// `GET /api/owner/analytics/kitchen`, services/analytics/kitchen/topics.rs)
// drawn as one card under the kitchen numbers. PURE -- markup out, nothing
// fetched -- so it renders in node (`kitchen-topics.test.mjs`).
//
// A TOPIC IS ABOUT A DISH, never a person: the rows are keyed by week, dish
// and topic, and the hub has already taken digits, names and addresses out
// of the example phrases. This file escapes them again and adds nothing.
//
// ASCII QUOTES ONLY in this file.

import { ui } from './parts.js';

const esc = ui.esc;
/// The topics the hub folds, in the order a cook reads them: problems first.
export const TOPICS = ['cold', 'late', 'salty', 'small_portion', 'not_tasty', 'stale', 'spicy', 'packaging', 'tasty', 'fresh'];
const GOOD = new Set(['tasty', 'fresh']);
const day = d => { const s = String(d); return `${s.slice(0, 4)}-${s.slice(4, 6)}-${s.slice(6, 8)}`; };

/// The card: a table of week, dish, topic, how many notes, and up to two
/// example phrases; or a line saying there is nothing yet.
export function topicsCard(topics){
  const rows = (Array.isArray(topics) ? topics : []).filter(r => TOPICS.includes(r.topic))
    .slice().sort((a, b) => (b.week - a.week) || (TOPICS.indexOf(a.topic) - TOPICS.indexOf(b.topic)) || (b.count - a.count));
  const head = '<p class="eyebrow mt-3" data-t="ft_title"></p><p class="hint" data-t="ft_hint"></p>';
  if (!rows.length) return `<section class="kt-topics" data-tour="kitchen.topics">${head}<p class="hint" data-t="ft_none"></p></section>`;
  const body = rows.map(r => `<tr><td>${esc(day(r.week))}</td><td>${esc(r.name || r.dish)}</td>`
    + `<td class="${GOOD.has(r.topic) ? 'pos' : 'neg'}" data-t="ft_${r.topic}"></td><td>${Number(r.count) || 0}</td>`
    + `<td>${(r.examples || []).slice(0, 2).map(x => `<q>${esc(x)}</q>`).join(' ')}</td></tr>`).join('');
  return `<section class="kt-topics" data-tour="kitchen.topics">${head}<div class="kt-wrap"><table class="kt"><thead><tr>`
    + ['ft_week', 'ka_dish', 'ft_topic', 'ft_notes', 'ft_example'].map(k => `<th data-t="${k}"></th>`).join('')
    + `</tr></thead><tbody>${body}</tbody></table></div></section>`;
}
