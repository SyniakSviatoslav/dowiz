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
import { money } from './logic.js';
import { ui, k, act, backBar, actionRow, amt, loading } from './parts.js';

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

/// One dish: a tap adds one; a count badge and a "less" once it is in the basket.
function dish(c, p, n) {
  const { S, t } = c;
  const off = p.available === false;
  const pick = actionRow({ title: p.name || '', disabled: off,
    sub: off ? ui.esc(t('unavailable')) : '',
    trailing: amt(money(p.price || 0, S.currency, c.locale()), { cls: 'amt' }) + (n ? ui.badge({ tone: 'accent', label: String(n), cls: 'count' }) : ''),
    attrs: act('pick', { id: p.id }, 'menu.dish') });
  const less = n ? ui.iconButton({ icon: 'minus', ariaLabel: k('less'), attrs: act('unpick', { id: p.id }, 'menu.less') }) : '';
  return `<li>${pick}${less}</li>`;
}

/// The picker: categories, a search, a basket of taps, one send. `title` is
/// the heading key (`addItem` on a round, `openTable` when opening one) and
/// `lead` is markup drawn under it (open.js's table field).
export function renderAdd(c, title = 'addItem', lead = '') {
  const { S, t } = c;
  const q = (S.menuQ || '').trim().toLowerCase();
  const basket = S.basket || {};
  const n = Object.values(basket).reduce((a, b) => a + b, 0);
  let body;
  if (!S.menu && !S.menuError) body = loading(t);
  else if (!S.menu) body = ui.emptyState({ icon: 'plug-connected-x', title: k(S.menuError), alert: true });
  else {
    body = S.menu.map(cat => {
      const ps = (cat.products || []).filter(p => !q || String(p.name || '').toLowerCase().includes(q));
      if (!ps.length) return '';
      return `<h3>${ui.esc(cat.name)}</h3><ul class="menu">${ps.map(p => dish(c, p, basket[p.id] || 0)).join('')}</ul>`;
    }).join('') || ui.emptyState({ icon: 'search-off', title: k('noMatch') });
  }
  return `
    ${backBar()}
    <h2>${ui.esc(t(title))}</h2>
    ${lead}
    ${ui.inputRow({ type: 'search', label: k('search'), placeholder: k('search'), cls: 'search',
      attrs: { value: S.menuQ || '', data: { in: 'q', tour: 'menu.search' } } })}
    ${body}
    <div class="dock">${ui.button({ variant: 'primary', size: 'lg', block: true, icon: 'send', label: t('addN').replace('{n}', n), disabled: !n, attrs: act('send', {}, 'menu.send') })}</div>`;
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
