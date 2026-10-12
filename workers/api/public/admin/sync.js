// AFTER A SAVE, THE SCREEN FOLLOWS WITHOUT F5 (lane W-TKUI, 2026-10-12,
// operator TZ 6; the shared mechanism for every console screen). Two needs:
//   syncDish(id)      -- the dish sheet saved: ONE read of the menu in the
//                        console language, folded into the copy in place
//                        (`sync-logic.js`), the screen redrawn. Not four
//                        languages, not the whole venue -- unless the dish is
//                        not in the answer or the read fails, when the whole
//                        venue is reloaded rather than a stale row kept.
//   redrawWhenClosed() -- an action that keeps its sheet open (a photo, a
//                        nudge): `rerender` skips a screen under a sheet, so
//                        the list behind it is redrawn the moment it closes.
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { S, api, store, whenSheetCloses } from '/admin/core.js';
import { lang } from '/admin/i18n.js';
import { loadVenue, rerender } from '/admin/app.js';
import { foldMenu } from '/admin/sync-logic.js';

// As `app.js venueSlug` (not exported there): <venue>.dowiz.org, else the stored location.
const HOST_LABELS_OF_A_VENUE = 3;
const venueSlug = () => { const h = String(globalThis.location?.hostname || '').split('.'); return h.length >= HOST_LABELS_OF_A_VENUE ? h[0] : store.loc; };

/// The dish `id` as the hub has it now, folded in and drawn. True when one read was enough.
export async function syncDish(id, { translations = {} } = {}){
  try {
    const d = await api(`/public/locations/${encodeURIComponent(venueSlug())}/menu?locale=${lang}&fresh=1`);
    if (foldMenu(S, d, id, lang, translations)) { await rerender(); return true; }
  } catch {}
  await loadVenue(); await rerender(); return false;
}

/// The list behind an open sheet is redrawn when the sheet closes.
export const redrawWhenClosed = () => whenSheetCloses(() => rerender());
