// Sheets: a MODAL surface that rises from the bottom on a phone and sits
// centred on a wide screen. For a decision that interrupts (confirm a refund,
// edit one dish), never for a screen's main content.
//
// What makes it a dialog and not a div that looks like one:
//   * role="dialog" + aria-modal + aria-labelledby its title
//   * focus moves INTO it on open and BACK to the opener on close
//   * Tab is trapped inside; Escape closes; a tap on the scrim closes
//   * any element with `data-ui-close` closes it, reporting its value
//
// `sheet()` is the markup, pure; `openSheet()` mounts it and wires the above;
// `confirmSheet()` is the promise-shaped yes/no built on both.
import { esc, cx, attrs, merge, label, uid, text } from './core.js';
import { button, iconButton } from './button.js';

/// @param o { id, title, body (markup), actions (markup), closeLabel, cls }
export function sheet(o = {}){
  const id = o.id || uid('sheet');
  const tId = `${id}-title`;
  const a = merge({ id, class: cx('ui-sheet', o.cls), role: 'dialog', 'aria-modal': 'true',
                    'aria-labelledby': o.title ? tId : null, 'aria-label': o.title ? null : text(o.label) || null,
                    tabindex: '-1' });
  const close = o.closeLabel
    ? iconButton({ icon: 'x', ariaLabel: o.closeLabel, variant: 'plain', cls: 'ui-sheet-x', attrs: { 'data-ui-close': 'dismiss' } })
    : '';
  return `<section${attrs(a)}><div class="ui-sheet-grip" aria-hidden="true"></div>`
    + `<header class="ui-sheet-head">${o.title ? `<h2 class="ui-sheet-title" id="${esc(tId)}">${label(o.title)}</h2>` : ''}${close}</header>`
    + `<div class="ui-sheet-body">${o.body || ''}</div>`
    + (o.actions ? `<footer class="ui-sheet-actions">${o.actions}</footer>` : '')
    + '</section>';
}

const FOCUSABLE = 'button:not([disabled]),[href],input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

/// Mount a sheet. Returns { el, close(value) }. `onClose(value)` gets the
/// `data-ui-close` value, 'escape', 'scrim', or whatever `close()` was given.
export function openSheet(o = {}, doc = globalThis.document){
  const scrim = doc.createElement('div');
  scrim.className = 'ui-scrim';
  scrim.innerHTML = sheet(o);
  const panel = scrim.querySelector('[role="dialog"]');
  const opener = doc.activeElement;
  let open = true;
  const close = value => {
    if (!open) return;
    open = false;
    doc.removeEventListener('keydown', onKey, true);
    scrim.remove();
    if (opener && opener.focus) opener.focus();
    o.onClose && o.onClose(value);
  };
  const onKey = e => {
    if (e.key === 'Escape') { e.preventDefault(); close('escape'); return; }
    if (e.key !== 'Tab') return;
    const f = [...panel.querySelectorAll(FOCUSABLE)];
    if (!f.length) { e.preventDefault(); panel.focus(); return; }
    const first = f[0], last = f[f.length - 1];
    if (e.shiftKey && doc.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && doc.activeElement === last) { e.preventDefault(); first.focus(); }
  };
  scrim.addEventListener('click', e => {
    if (e.target === scrim) return close('scrim');
    const c = e.target.closest && e.target.closest('[data-ui-close]');
    if (c && panel.contains(c)) close(c.getAttribute('data-ui-close'));
  });
  doc.addEventListener('keydown', onKey, true);
  doc.body.appendChild(scrim);
  const first = panel.querySelector('[autofocus]') || panel.querySelector(FOCUSABLE) || panel;
  first.focus();
  return { el: panel, close };
}

/// Yes/no, as a promise. Resolves true only on the confirm button.
export function confirmSheet(o = {}, doc = globalThis.document){
  return new Promise(resolve => {
    const actions = button({ label: o.cancelLabel || 'Cancel', variant: 'ghost', attrs: { 'data-ui-close': 'no' } })
      + button({ label: o.confirmLabel || 'OK', variant: o.danger ? 'danger' : 'primary', attrs: { 'data-ui-close': 'yes' } });
    openSheet({ ...o, body: o.body ? `<p class="ui-sheet-text">${label(o.body)}</p>` : '', actions,
                onClose: v => resolve(v === 'yes') }, doc);
  });
}
