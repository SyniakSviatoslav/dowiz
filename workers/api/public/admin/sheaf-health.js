// THE CONSISTENCY RADIUS ON THE OWNER'S HEALTH PAGE (W-TASTE2 S7a; operator 2026-10-06: "radius goes
// to owner health, NOT a red alarm, and to the log"). `GET /api/owner/health` -> `sheaf.taste`:
// how far the phones' taste vectors and the venue's own profiles disagree, per mille, over the
// guests that have both. A figure with a neutral pill: nothing acts on it and it is never red.
//
// ASCII QUOTES ONLY in this file.

import { icon } from '/admin/core.js';
import { T, LANGS, retranslate } from '/admin/i18n.js';
import { pill, rowDiv } from '/admin/parts.js';
import { WORDS, radiusSub } from '/admin/sheaf-health-view.js';

for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// The health pane's row (more.js openHealth). Never a warning tone: it is a figure.
export function healthRow(host, sheaf){
  const sub = radiusSub(sheaf?.taste || (sheaf?.error ? sheaf : null));
  if (!host || !sub) return;
  host.insertAdjacentHTML('beforeend', `<div class="rows mt-3">${rowDiv({ leading: icon('radar-2'), title: { t: 'shR_title' }, sub, trailing: pill('ok', { label: 'i' }), tour: 'health.radius' })}</div>`);
  retranslate(host);
}
