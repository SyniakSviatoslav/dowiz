// Chips: small, rounded, and either a FLOATING READOUT (the courier's shift and
// GPS over the map) or a TOGGLE (a filter). The two are distinguished by
// `as`, because a <span> that looks tappable is a lie and a <button> that
// does nothing is a trap.
//
//   chip({ as:'span' })   readout; can be `live` so a change is announced
//   chip({ as:'button' }) toggle; `selected` renders aria-pressed
//
// `floating` gives it the translucent over-the-map surface and elevation.
import { cx, attrs, merge, icon, label, i18nAttr, tone } from './core.js';

/// @param o { label, icon, as='span', selected, tone='neutral', dot, floating,
///            live, id, cls, hidden, ariaLabel, attrs }
export function chip(o = {}){
  const tn = tone(o.tone);
  const tag = o.as === 'button' ? 'button' : 'span';
  const a = merge(
    { id: o.id, class: cx('ui-chip', tn !== 'neutral' && `ui-chip--${tn}`, o.floating && 'ui-chip--float',
                          o.dot && 'ui-chip--dot', o.cls),
      hidden: !!o.hidden },
    tag === 'button' ? { type: 'button', 'aria-pressed': o.selected == null ? null : String(!!o.selected) } : null,
    o.live ? { role: 'status', 'aria-live': 'polite' } : null,
    i18nAttr('aria-label', o.ariaLabel),
    o.attrs,
  );
  const dot = o.dot ? '<span class="ui-dot" aria-hidden="true"></span>' : '';
  return `<${tag}${attrs(a)}>${dot}${icon(o.icon)}${label(o.label, 'span', o.labelCls || '')}</${tag}>`;
}
