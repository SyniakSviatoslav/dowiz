// THE STAMP CARD on the customer's own order page (C5, BLUEPRINT-CRM-CONSENT-
// LOYALTY section 3.5). "3 / 10", what a full card takes off, and -- on the
// order that spent one -- what it took. Nothing else: no expiry, no levels,
// no "almost there", and nothing is ever sent to anyone about it. The count
// is the hub's fold (`services/loyalty/stamps.rs`); this file only draws it.
//
// ASCII QUOTES ONLY in this file: a typographic quote is a syntax error.

import { API, moneyEl } from '/store/state.js';
import { retranslate } from '/store/i18n.js';
import { stampView } from '/lib/stamps.js';

/// The last answer per order: the tracking sheet re-renders every few
/// seconds, and the card does not change between two polls of one status.
const seen = new Map();

/// The card's markup from `GET /api/order/:id/stamps` (`stampView`), or ''
/// when the venue has none.
export function cardMarkup(d){
  const v = stampView(d);
  if (!v) return '';
  const { have, n } = v;
  const dots = Array.from({ length: n }, (_, i) => `<i class="${i < have ? 'done' : ''}"></i>`).join('');
  return `<p class="eyebrow" data-t="stampCard"></p>
    <p class="mono"><b>${have} / ${n}</b></p>
    <div class="ep-dots" aria-hidden="true">${dots}</div>
    <p class="muted small"><span data-t="stampRule"></span> ${moneyEl(v.reward)}</p>
    ${v.used > 0 ? `<p class="small"><span data-t="stampUsed"></span> ${moneyEl(v.used)}</p>` : ''}`;
}

/// The placeholder the tracking sheet draws; filled by `mountStamps`.
export const stampsMarkup = order => order?.id ? `<section class="credits-wrap" id="stampCard" hidden></section>` : '';

/// Fill it with the order's own token. Silent on failure: the order is the
/// page, the card is a line on it.
export async function mountStamps(order, tok){
  const box = document.getElementById('stampCard');
  if (!box || !tok) return;
  const k = `${order.id}|${order.status}`;
  let d = seen.get(k);
  if (!d) {
    try {
      const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/stamps`, { headers: { authorization: 'Bearer ' + tok } });
      if (!r.ok) return;
      d = await r.json();
      seen.set(k, d);
    } catch { return; }
  }
  const el = document.getElementById('stampCard');
  const html = cardMarkup(d);
  if (!el || !html) return;
  el.innerHTML = html;
  el.hidden = false;
  retranslate(el);
}
