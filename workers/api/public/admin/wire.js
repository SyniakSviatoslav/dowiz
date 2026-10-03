// THE W-WIRE TILES: every backend capability that had no screen, one tap
// each (operator 2026-09-26: "усе що є на бекенді, має бути підключено, видно
// та мати змогу використовуватись на ui також"). The navigation belongs to
// lane W-UX; it places these with one line (the lane's hand-back):
//   import { mountTiles } from '/admin/wire.js';  mountTiles(host);
// Each tile loads its screen on the tap, so the console's first paint does
// not grow by eight modules.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $$, icon, store } from '/admin/core.js';
import { principalOf } from '/admin/kitchen-logic.js';
import { tilesFor, canTile } from '/admin/access.js';
import { rowBtn, pill } from '/admin/parts.js';
import { paint } from '/admin/wire-core.js';
import '/admin/offline-i18n.js';

/// key: the i18n title; sub: its one-line hint; mod/fn: what a tap opens.
export const TILES = [
  { id: 'messages', icon: 'message-2', key: 'w_messages', sub: 'w_messagesSub', mod: '/admin/threads.js', fn: 'openMessages' },
  { id: 'wallets', icon: 'wallet', key: 'w_wallets', sub: 'w_walletsSub', mod: '/admin/wallet.js', fn: 'openWallets' },
  { id: 'history', icon: 'history', key: 'w_olderOrders', sub: 'w_olderOrdersSub', mod: '/admin/history.js', fn: 'openHistory' },
  { id: 'tax', icon: 'receipt', key: 'w_taxTitle', sub: 'w_taxSub', mod: '/admin/tax.js', fn: 'openTax' },
  { id: 'catWords', icon: 'language', key: 'w_catWords', sub: 'w_catWordsSub', mod: '/admin/cat-i18n.js', fn: 'openCategoryWords' },
  { id: 'brand', icon: 'photo', key: 'w_brandExtra', sub: 'w_brandExtraSub', mod: '/admin/brand-extra.js', fn: 'openBrandExtra' },
  { id: 'safety', icon: 'key', key: 'w_safety', sub: 'w_safetySub', mod: '/admin/safety.js', fn: 'openSafety' },
  { id: 'graph', icon: 'sparkles', key: 'w_assistSources', sub: 'w_assistSourcesSub', mod: '/admin/assist-sources.js', fn: 'openAssistSources' },
  // W-OFFSALE: the room tablet's offline cash sales and their 48 h fiscal deadline.
  { id: 'offlineSales', icon: 'receipt', key: 'of_title', sub: 'of_sub', mod: '/admin/offline.js', fn: 'openOfflineSales' },
];

/// The tiles the signed-in person opens: all for an owner; for a member of
/// staff only those their words open (KITCHEN-ACCESS-2026-09-27, `access.js`).
export const myTiles = (token = store.t) => tilesFor(principalOf(token), TILES);

/// Open one tile's screen (never one the person's words do not open).
export const openTile = id => {
  const tile = canTile(principalOf(store.t), id) && TILES.find(x => x.id === id);
  return tile ? import(tile.mod).then(m => m[tile.fn]()) : Promise.resolve();
};

/// Draw the tiles into `host`; the messages tile carries the unread count.
export async function mountTiles(host){
  if (!host) return;
  const tiles = myTiles();
  if (!tiles.length) { host.innerHTML = ''; return; }
  host.innerHTML = `<div class="rows" role="list">${tiles.map(x => rowBtn({ data: { wtile: x.id }, leading: icon(x.icon), title: { t: x.key },
    sub: `<span data-t="${x.sub}"></span>`, trailing: x.id === 'messages' ? '<span data-wunread></span>' : icon('chevron-right') })).join('')}</div>`;
  paint(host);
  for (const b of $$('[data-wtile]', host)) b.onclick = () => openTile(b.dataset.wtile);
  if (!tiles.some(x => x.id === 'messages')) return;
  const n = await import('/admin/threads.js').then(m => m.unread()).catch(() => 0);
  const slot = host.querySelector('[data-wunread]');
  if (slot) slot.innerHTML = n ? pill('warn', { label: String(n) }) : icon('chevron-right');
}
