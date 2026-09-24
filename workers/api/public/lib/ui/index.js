// dowiz UI — the one import a surface needs.
//
//   import * as ui from '/lib/ui/index.js';
//   ui.useTranslator(t);                        // once, with the surface's t()
//   $('#app').innerHTML = ui.button({ label:{ t:'signIn' }, variant:'primary', icon:'login' });
//
// Every export below is either a PURE render function (options -> HTML string,
// escaped) or a small behaviour binder that takes an element. See
// docs/design/DESIGN-SYSTEM.md for the API, the do/don't and the migration
// recipe; /ui-gallery/ shows every component in every state.
export { esc, cx, attrs, icon, label, text, useTranslator, tr, uid, TONES } from './core.js';
export { button, iconButton, setBusy, VARIANTS } from './button.js';
export { badge, status, STATUSES } from './badge.js';
export { chip } from './chip.js';
export { field, inputRow, alert } from './field.js';
export { segmented, bindSegmented } from './segmented.js';
export { tabs, bindTabs } from './tabs.js';
export { row, list } from './list.js';
export { emptyState } from './empty.js';
export { skeleton } from './skeleton.js';
export { toastHost, createToaster, TOAST_MS } from './toast.js';
export { sheet, openSheet, confirmSheet } from './sheet.js';
export { amount, stat } from './money.js';
export { card, section, para } from './card.js';
export { when, mmss } from './time.js';
