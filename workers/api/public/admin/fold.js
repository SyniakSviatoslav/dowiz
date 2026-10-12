// ONE COLLAPSED SECTION, REMEMBERED PER DEVICE (lane W-TKUI, 2026-10-12,
// operator TZ 2 and the sweep): an optional group of a long form sits under
// <details>/<summary> -- the summary is in the tab order and toggles on Enter
// and Space by itself, so nothing is scripted for the keyboard. Whether THIS
// device left it open is remembered in localStorage; a blocked storage (a
// private window) only means "closed". ASCII QUOTES ONLY in this file.

import { $$, esc } from '/admin/core.js';

const PREFIX = 'dw_fold_';
/// PURE over `storage`: was fold `id` left open on this device?
export const foldOpen = (storage, id) => { try { return storage?.getItem(PREFIX + id) === 'open'; } catch { return false; } };
/// Remember the toggle; never throws.
export const rememberFold = (storage, id, open) => { try { storage?.setItem(PREFIX + id, open ? 'open' : 'closed'); } catch {} };
const storage = () => { try { return globalThis.localStorage; } catch { return null; } };

/// The markup: `titleKey` (a T key) or `title` (text) on the summary; `open`
/// forces the state (null = as remembered, closed the first time).
export function fold({ id, titleKey = null, title = '', open = null, cls = '', tour = null }, html){
  const on = open ?? foldOpen(storage(), id);
  return `<details class="fold${cls ? ' ' + cls : ''}" id="${esc(id)}" data-fold="${esc(id)}"${on ? ' open' : ''}${tour ? ` data-tour="${esc(tour)}"` : ''}>
    <summary><span class="eyebrow"${titleKey ? ` data-t="${esc(titleKey)}"` : ''}>${titleKey ? '' : esc(title)}</span></summary>${html}</details>`;
}

/// Every fold under `root` remembers its toggles.
export function bindFolds(root){
  for (const d of $$('[data-fold]', root)) d.addEventListener('toggle', () => rememberFold(storage(), d.dataset.fold, d.hasAttribute('open')));
}
