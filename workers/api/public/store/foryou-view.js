// PURE. "FOR YOU" ON THE ORDER PAGE, from the VENUE's own profile of the guest (W-TASTE2 row 2,
// operator 2026-10-06). The hub ranks (`GET /api/order/:id/taste/for-you`, services/customers/taste/
// foryou.rs over taste.dwb); this file only draws the answer. Relative imports only, so node renders
// it for real (foryou-view.test.mjs).
//
// Drawn ONLY when the hub says `shown` and at least one dish is on this page's menu: an objection,
// no profile or no taste draws NOTHING (no empty box, no "we know nothing about you"). Dish names
// come from the menu the page already holds; the answer carries ids and taste words, never a
// number, and this file shows none. The phone's own strip (taste-device.js) is untouched.
//
// ASCII QUOTES ONLY in this file.

import { chip } from '../lib/ui/chip.js';
import { becauseLine } from './sense-view.js';

/// The answer's states, as the hub names them.
export const STATES = ['off', 'no-profile', 'no-taste', 'shown'];

/// `d` = the hub's answer; `nameOf(id)` = the dish's name on this page's menu, or null (a dish the
/// page cannot name is left out rather than shown as an id). Answers '' when there is nothing to show.
export function forYouMarkup(d, nameOf){
  if (!d || d.state !== 'shown' || !Array.isArray(d.items)) return '';
  const items = d.items.map(x => ({ id: String(x?.id ?? ''), name: x?.id ? nameOf(String(x.id)) : null })).filter(x => x.id && x.name);
  if (!items.length) return '';
  return `<p class="eyebrow" data-t="fy_title"></p>
    <div class="fy-row">${items.map(x => chip({ as: 'button', label: x.name, cls: 'fy', attrs: { 'data-fy-order': x.id, 'data-tour': 'track.forYou' } })).join('')}</div>
    ${becauseLine(Array.isArray(d.because) ? d.because : [])}
    <p class="small muted" data-t="fy_hint"></p>`;
}
