// ADD TO A ROUND, from the venue's own menu.
//
// THE READ IS THE STOREFRONT'S: `GET /api/public/locations/:slug/menu`
// (storefront.rs `menu`), the one public read of the catalogue, with the
// venue's currency beside it (`location.currencyCode`). The prices drawn here
// are a READING; the server prices every added line itself with THE pricer
// (`handlers.rs priced_line`) and the tablet sends only `{product_id,
// quantity}`, never a price.
//
// A DISH WITH MODIFIER GROUPS is offered without them. If one is required the
// pricer refuses the line, and its words are shown -- choosing modifiers on a
// tablet is not built yet (see the lane's OPEN list).
import { esc, money } from './logic.js';

/// Fetch and keep the menu. Sets `S.menu` (categories) and `S.currency`.
export async function loadMenu(c) {
  const { S } = c;
  if (!S.slug) { S.menuError = 'noSlug'; return; }
  try {
    const d = await c.api(`/public/locations/${encodeURIComponent(S.slug)}/menu?locale=${encodeURIComponent(c.lang())}`);
    S.menu = Array.isArray(d?.categories) ? d.categories : [];
    const cur = d?.location?.currencyCode;
    if (cur) { S.currency = cur; c.remember({ currency: cur }); }
    S.menuError = null;
  } catch (e) {
    S.menuError = e.offline ? 'offline' : 'menuFailed';
  }
}

/// The picker: categories, a search, a basket of taps, one send.
export function renderAdd(c) {
  const { S, t } = c;
  const loc = c.locale();
  const q = (S.menuQ || '').trim().toLowerCase();
  const basket = S.basket || {};
  const n = Object.values(basket).reduce((a, b) => a + b, 0);
  let body;
  if (!S.menu) body = `<p class="muted">${esc(t(S.menuError === 'noSlug' ? 'noSlug' : S.menuError ? S.menuError : 'loading'))}</p>`;
  else {
    body = S.menu.map(cat => {
      const ps = (cat.products || []).filter(p => !q || String(p.name || '').toLowerCase().includes(q));
      if (!ps.length) return '';
      return `<h3>${esc(cat.name)}</h3><ul class="menu">${ps.map(p => {
        const k = basket[p.id] || 0;
        const off = p.available === false;
        return `<li><button class="row" data-act="pick" data-id="${esc(p.id)}" ${off ? 'disabled' : ''}>
          <span class="name">${esc(p.name)}${off ? ` · <em>${esc(t('unavailable'))}</em>` : ''}</span>
          <span class="amt">${money(p.price || 0, S.currency, loc)}</span>${k ? `<span class="count">${k}</span>` : ''}</button>
          ${k ? `<button class="btn sq" data-act="unpick" data-id="${esc(p.id)}" aria-label="${esc(t('less'))}">−</button>` : ''}</li>`;
      }).join('')}</ul>`;
    }).join('') || `<p class="muted">${esc(t('noMatch'))}</p>`;
  }
  return `
    <div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div>
    <h2>${esc(t('addItem'))}</h2>
    <input class="search" type="search" data-in="q" value="${esc(S.menuQ || '')}" placeholder="${esc(t('search'))}" aria-label="${esc(t('search'))}">
    ${body}
    <div class="dock"><button class="cta" data-act="send" ${n ? '' : 'disabled'}>${esc(t('addN').replace('{n}', n))}</button></div>`;
}

export function bindAdd(c, root, round, amend) {
  const { S } = c;
  root.oninput = ev => {
    if (ev.target.dataset.in !== 'q') return;
    S.menuQ = ev.target.value;
    const pos = ev.target.selectionStart;
    c.render();
    const i = root.querySelector('[data-in="q"]');
    if (i) { i.focus(); try { i.setSelectionRange(pos, pos); } catch {} }
  };
  root.onclick = async ev => {
    const b = ev.target.closest('[data-act]');
    if (!b || b.disabled) return;
    const act = b.dataset.act, id = b.dataset.id;
    S.basket = S.basket || {};
    if (act === 'back') { S.view = 'round'; return c.render(); }
    if (act === 'pick') { S.basket[id] = (S.basket[id] || 0) + 1; return c.render(); }
    if (act === 'unpick') { S.basket[id] = Math.max(0, (S.basket[id] || 0) - 1); if (!S.basket[id]) delete S.basket[id]; return c.render(); }
    if (act === 'send') {
      const ops = Object.entries(S.basket).map(([product_id, quantity]) => ({ op: 'add', product_id, modifier_ids: [], quantity }));
      if (!ops.length) return;
      b.disabled = true;
      const moved = await amend(c, round, ops);
      if (moved) { S.basket = {}; S.view = 'round'; }
      c.render();
    }
  };
}
