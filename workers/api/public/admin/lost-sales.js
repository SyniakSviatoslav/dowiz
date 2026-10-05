// The "orders lost to stock-outs" card on the kitchen numbers (A13, W-LOST).
// `kitchen-analytics.js` calls `fill` once its answer is drawn; the markup is
// `lost-sales-view.js` (pure), the words `lost-sales-words.js`.
//
// ASCII QUOTES ONLY in this file.

import '/admin/lost-sales-i18n.js';
import { money, retranslate, repaintMoney } from '/admin/core.js';
import { drawLost } from '/admin/lost-sales-view.js';

/// Append the card to `host` from the kitchen answer `r` already fetched.
export function fill(host, r){
  if (!host || !r || !r.lost) return;
  const box = document.createElement('div');
  box.id = 'kaLost';
  box.innerHTML = drawLost(r, { money });
  host.querySelector('#kaLost')?.remove();
  host.append(box);
  retranslate(box); repaintMoney(box);
}
