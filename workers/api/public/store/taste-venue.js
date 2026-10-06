// "WE REMEMBER YOUR TASTE · TURN OFF", on the guest's own order page (W-MR0 row MR8; DECISIONS.md
// D0 amendment 2026-10-04, operator ruling: automatic, legitimate interest, one-tap objection).
// It is the guest's copy of everything the venue holds about their taste (Art. 15: the export),
// the rule that put them in a segment, and the one tap that objects and deletes it (Art. 21).
// Drawn whenever the guest has not objected; after the objection it says so.
//
// ASCII QUOTES ONLY in this file: a typographic quote is a syntax error.

import { API, findProduct } from '/store/state.js';
import { forYouMarkup } from '/store/foryou-view.js';
import { t, retranslate } from '/store/i18n.js';
import { esc, toast } from '/store/ui.js';
import { objectVenue } from '/store/taste-device.js';
import { ui } from '/store/parts.js';
import '/store/taste-words.js';
import { heldLine } from '/store/taste-held.js'; // W-SNN

const seen = new Map();
const fySeen = new Map();

/// The markup from `GET /api/order/:id/taste`, or '' when the venue keeps nothing.
export function venueTasteMarkup(d){
  if (!d) return '';
  if (d.objected) return `<p class="small muted" data-t="vk_off"></p>`;
  const p = d.taste;
  const list = rows => (rows || []).map(r => esc(r.key)).join(', ') || t('tasteNothing');
  const body = p ? `
    <p class="small"><b>${esc(t('seg_' + p.segment))}</b> - <span class="muted">${esc(p.why)}</span></p>
    <p class="small"><span data-t="vk_orders"></span> ${Number(p.orders) | 0}</p>
    <p class="small"><span data-t="vk_tags"></span> ${list(p.tags)}</p>
    <p class="small"><span data-t="vk_cats"></span> ${list(p.cats)}</p>
    ${p.device ? `<p class="small muted" data-t="vk_device"></p>` : ''}
    <p class="small muted"><span data-t="vk_kept"></span> ${Number(p.daysSince) | 0}</p>${heldLine(p.held)}`
    : `<p class="small muted" data-t="vk_none"></p>`;
  return `<p class="eyebrow" data-t="vk_title"></p>${body}
    <p class="small muted" data-t="vk_never"></p>
    ${ui.button({ variant: 'ghost', label: { t: 'vk_withdraw' }, id: 'vkWithdraw', cls: 'linky', attrs: { data: { tour: 'track.tasteWithdraw' } } })}`;
}

export const venueTastePlace = order => order?.id ? `<section class="credits-wrap" id="forYouOrder" hidden></section><section class="credits-wrap" id="venueTaste" hidden></section>` : '';

/// W-TASTE2: "For you" from the venue's profile (store/foryou-view.js). Silent on failure and on
/// every answer that is not `shown`: nothing is drawn for a guest the venue keeps nothing about.
async function mountForYou(order, tok){
  const el = document.getElementById('forYouOrder'); if (!el) return;
  // Once per order per page: the tracking sheet redraws on every poll.
  let d = fySeen.get(order.id);
  if (d === undefined) {
    try {
      const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/taste/for-you`, { headers: { authorization: 'Bearer ' + tok } });
      d = r.ok ? await r.json() : null;
    } catch { d = null; }
    fySeen.set(order.id, d);
  }
  const html = forYouMarkup(d, id => findProduct(id)?.name || null);
  el.innerHTML = html; el.hidden = !html;
  if (!html) return;
  retranslate(el);
  for (const b of el.querySelectorAll('[data-fy-order]')) b.onclick = () => {
    const p = findProduct(b.dataset.fyOrder);
    if (p && p.available !== false) import('/store/dish.js').then(m => m.openDish(p)).catch(() => {});
  };
}

/// Fill it with the order's own token; silent on failure (the order is the page).
export async function mountVenueTaste(order, tok){
  if (!document.getElementById('venueTaste') || !tok) return;
  mountForYou(order, tok);
  const k = order.id;
  let d = seen.get(k);
  if (!d) {
    try {
      const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/taste`, { headers: { authorization: 'Bearer ' + tok } });
      if (!r.ok) return;
      d = await r.json();
      seen.set(k, d);
    } catch { return; }
  }
  const el = document.getElementById('venueTaste');
  const html = venueTasteMarkup(d);
  if (!el || !html) return;
  el.innerHTML = html;
  el.hidden = false;
  retranslate(el);
  const b = document.getElementById('vkWithdraw');
  if (b) b.onclick = async () => {
    const ok = await objectVenue([{ id: order.id, token: tok }], API);
    if (!ok) return toast(t('vk_failed'));
    seen.set(k, { objected: true, taste: null });
    fySeen.set(k, { state: 'off', items: [] });
    const fy = document.getElementById('forYouOrder'); if (fy) { fy.innerHTML = ''; fy.hidden = true; }
    el.innerHTML = venueTasteMarkup(seen.get(k));
    retranslate(el);
    toast(t('vk_withdrawn'));
  };
}
