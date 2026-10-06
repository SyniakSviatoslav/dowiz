// W-SNN: THE SHEAF NETWORK'S SHADOW, as the owner sees it (More -> Customers, under the segments).
// The network ranks dishes beside the current "For you" ranker and is NEVER shown to a guest while
// the switch is "shadow" (the default); this line says how often the two agree, counted per venue,
// never per guest (`GET /api/owner/snn`, src/services/customers/taste/snn.rs). The switch is
// `POST /api/owner/snn {mode}`: shadow | on | off. "Off" runs nothing at all.
//
// The words and the markup are `snn-view.js` (pure, node-tested). ASCII QUOTES ONLY as string delimiters.

import { $, api, post, retranslate } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { WORDS, snnMarkup } from '/admin/snn-view.js';

for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// Fill `#cuSnn` above the customers list; a chip sets the switch and redraws.
export async function mountSnn(){
  const el = $('#cuSnn');
  if (!el) return;
  let d;
  try { d = await api('/owner/snn'); } catch { return; /* the list stands without it */ }
  if (!$('#cuSnn')) return;
  el.innerHTML = snnMarkup(d);
  retranslate(el);
  for (const b of el.querySelectorAll('[data-snn-mode]')) {
    b.onclick = async () => {
      b.disabled = true;
      try { await post('/owner/snn', { mode: b.dataset.snnMode }); } catch { /* redraw shows the switch as stored */ }
      return mountSnn();
    };
  }
}
