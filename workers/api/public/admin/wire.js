// THE W-WIRE TILES: every backend capability that had no screen, one tap
// each (operator 2026-09-26: "усе що є на бекенді, має бути підключено, видно
// та мати змогу використовуватись на ui також"). The navigation belongs to
// lane W-UX; it places these with one line (the lane's hand-back):
//   import { mountTiles } from '/admin/wire.js';  mountTiles(host);
// Each tile loads its screen on the tap, so the console's first paint does
// not grow by eight modules.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $$, icon } from '/admin/core.js';
import { rowBtn, pill } from '/admin/parts.js';
import { paint } from '/admin/wire-core.js';

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
];

/// Open one tile's screen.
export const openTile = id => {
  const tile = TILES.find(x => x.id === id);
  return tile ? import(tile.mod).then(m => m[tile.fn]()) : Promise.resolve();
};

/// Draw the tiles into `host`; the messages tile carries the unread count.
export async function mountTiles(host){
  if (!host) return;
  host.innerHTML = `<div class="rows" role="list">${TILES.map(x => rowBtn({ data: { wtile: x.id }, leading: icon(x.icon), title: { t: x.key },
    sub: `<span data-t="${x.sub}"></span>`, trailing: x.id === 'messages' ? '<span data-wunread></span>' : icon('chevron-right') })).join('')}</div>`;
  paint(host);
  for (const b of $$('[data-wtile]', host)) b.onclick = () => openTile(b.dataset.wtile);
  const n = await import('/admin/threads.js').then(m => m.unread()).catch(() => 0);
  const slot = host.querySelector('[data-wunread]');
  if (slot) slot.innerHTML = n ? pill('warn', { label: String(n) }) : icon('chevron-right');
}
