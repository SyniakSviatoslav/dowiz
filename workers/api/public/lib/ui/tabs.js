// Tabs: switch between PANELS of one screen. Different from a segmented control
// in one respect that matters to a screen reader: a tab owns a panel
// (aria-controls), a segment sets a value.
//
// WAI-ARIA tabs pattern, automatic activation: arrows move focus and select,
// Home/End jump, only the selected tab is in the tab order. Panels are the
// surface's; `tabs()` renders the list and `bindTabs()` toggles `hidden` on the
// panels it names and tells the surface.
import { cx, attrs, merge, icon, label, i18nAttr } from './core.js';

/// @param o { id, label, items:[{ id, label, icon, panel, badge }], active, cls }
///   `panel` is the id of the element the tab shows (default `${item.id}-panel`).
export function tabs(o = {}){
  const items = o.items || [];
  if (!items.length) throw new Error('ui.tabs: items are required');
  const cur = items.some(x => x.id === o.active) ? o.active : items[0].id;
  const list = items.map(x => {
    const on = x.id === cur;
    const a = { id: `tab-${x.id}`, type: 'button', role: 'tab', class: 'ui-tab',
                'aria-selected': String(on), 'aria-controls': x.panel || `${x.id}-panel`,
                tabindex: on ? '0' : '-1', data: { tab: x.id } };
    const badge = x.badge ? `<span class="ui-tab-badge">${label(String(x.badge))}</span>` : '';
    return `<button${attrs(a)}>${icon(x.icon)}${label(x.label)}${badge}</button>`;
  }).join('');
  const t = merge({ id: o.id, class: cx('ui-tabs', o.cls), role: 'tablist' }, i18nAttr('aria-label', o.label));
  return `<div${attrs(t)}>${list}</div>`;
}

/// Wire a rendered tab list. `doc` finds the panels (defaults to the list's
/// owner document). `onSelect(id)` fires on a real change. Returns unbind.
export function bindTabs(root, onSelect, doc){
  if (!root) return () => {};
  const d = doc || root.ownerDocument;
  const all = () => [...root.querySelectorAll('[role="tab"]')];
  const select = (b, focus) => {
    if (!b) return;
    const changed = b.getAttribute('aria-selected') !== 'true';
    for (const x of all()) {
      const on = x === b;
      x.setAttribute('aria-selected', String(on));
      x.setAttribute('tabindex', on ? '0' : '-1');
      const p = d && d.getElementById(x.getAttribute('aria-controls'));
      if (p) p.hidden = !on;
    }
    if (focus) b.focus();
    if (changed && onSelect) onSelect(b.dataset.tab, b);
  };
  const click = e => { const b = e.target.closest && e.target.closest('[role="tab"]'); if (b && root.contains(b)) select(b, false); };
  const key = e => {
    const list = all(), i = list.indexOf(e.target.closest ? e.target.closest('[role="tab"]') : null);
    if (i < 0) return;
    const to = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: list.length - 1 }[e.key];
    if (to == null) return;
    e.preventDefault();
    select(list[(to + list.length) % list.length], true);
  };
  root.addEventListener('click', click);
  root.addEventListener('keydown', key);
  return () => { root.removeEventListener('click', click); root.removeEventListener('keydown', key); };
}
