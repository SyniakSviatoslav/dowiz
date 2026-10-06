// W-SNN, Art. 15 (main's decision 2026-10-06): the guest's own "what we store" lists the two
// top-3s held beside their profile for a quality check -- temporary, deleted at the next order
// (src/services/customers/taste/snn.rs held_view). PURE, node-tested (taste-held.test.mjs).
// ASCII QUOTES ONLY as string delimiters.

import { esc } from '../lib/ui/index.js';

/// W-SNN (Art. 15): the two top-3s held for a quality check, deleted at the next order.
export function heldLine(h){
  if (!h || typeof h !== 'object') return '';
  const names = rows => (Array.isArray(rows) ? rows : []).map(r => esc(r.name || r.id)).join(', ') || '-';
  return `<p class="small muted" data-vk-held="1"><span data-t="vk_held"></span> ${names(h.current)} / ${names(h.network)}</p>`;
}
