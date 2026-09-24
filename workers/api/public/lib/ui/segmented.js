// Segmented control: ONE choice out of a few, all visible at once (language,
// a period, a view). It is a radio group -- role="radiogroup", each segment
// role="radio" with aria-checked -- and the keyboard behaves like one: arrows
// move AND select, Home/End jump, only the checked segment is in the tab order.
//
// Markup from `segmented()`, behaviour from `bindSegmented()`. The binding
// reports the value; it does not re-render, so the surface stays the owner of
// its state.
import { cx, attrs, merge, icon, label, i18nAttr, uid } from './core.js';

/// @param o { name, label (group's accessible name), options:[{value, label,
///            icon}], value, id, cls, attrs }
export function segmented(o = {}){
  const opts = o.options || [];
  if (!opts.length) throw new Error('ui.segmented: options are required');
  const name = o.name || uid('seg');
  const cur = opts.some(x => x.value === o.value) ? o.value : opts[0].value;
  const segs = opts.map(x => {
    const on = x.value === cur;
    const a = merge({ type: 'button', role: 'radio', class: 'ui-seg-b', 'aria-checked': String(on),
                      tabindex: on ? '0' : '-1', data: { value: String(x.value) } },
                    i18nAttr('aria-label', x.ariaLabel));
    return `<button${attrs(a)}>${icon(x.icon)}${label(x.label)}</button>`;
  }).join('');
  const g = merge({ id: o.id, class: cx('ui-seg', o.cls), role: 'radiogroup', data: { name } },
                  i18nAttr('aria-label', o.label), o.attrs);
  return `<div${attrs(g)}>${segs}</div>`;
}

/// Wire a rendered group. `onChange(value, button)` fires on a real change.
/// Returns an unbind function.
export function bindSegmented(root, onChange){
  if (!root) return () => {};
  const segs = () => [...root.querySelectorAll('[role="radio"]')];
  const select = (b, focus) => {
    if (!b || b.getAttribute('aria-checked') === 'true') { if (focus && b) b.focus(); return; }
    for (const s of segs()) { const on = s === b; s.setAttribute('aria-checked', String(on)); s.setAttribute('tabindex', on ? '0' : '-1'); }
    if (focus) b.focus();
    onChange && onChange(b.dataset.value, b);
  };
  const click = e => { const b = e.target.closest && e.target.closest('[role="radio"]'); if (b && root.contains(b)) select(b, false); };
  const key = e => {
    const all = segs(), i = all.indexOf(e.target.closest ? e.target.closest('[role="radio"]') : null);
    if (i < 0) return;
    const to = { ArrowRight: i + 1, ArrowDown: i + 1, ArrowLeft: i - 1, ArrowUp: i - 1, Home: 0, End: all.length - 1 }[e.key];
    if (to == null) return;
    e.preventDefault();
    select(all[(to + all.length) % all.length], true);
  };
  root.addEventListener('click', click);
  root.addEventListener('keydown', key);
  return () => { root.removeEventListener('click', click); root.removeEventListener('keydown', key); };
}
