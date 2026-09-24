// Cards and section headers: the two ways content is grouped.
//
//   card()     one idea on a raised surface (plan §8.3: "card = one idea").
//              `tone` tints its edge for an offer, a warning, a success.
//   section()  a titled group with an optional back control -- the header of a
//              panel that REPLACES a screen's content (earnings, history)
//              rather than stacking a second navigation model on top of it.
//   text()     a paragraph in the system's two voices: body and hint.
import { cx, attrs, merge, label, tone } from './core.js';
import { iconButton } from './button.js';

/// @param o { body (markup), title, eyebrow, tone, id, cls, attrs }
export function card(o = {}){
  const tn = o.tone ? tone(o.tone) : null;
  const a = merge({ id: o.id, class: cx('ui-card', tn && `ui-card--${tn}`, o.cls) }, o.attrs);
  const head = (o.eyebrow || o.title)
    ? `<div class="ui-card-head">${label(o.eyebrow, 'span', 'ui-eyebrow')}${label(o.title, 'h3', 'ui-card-title')}</div>` : '';
  return `<div${attrs(a)}>${head}${o.body || ''}</div>`;
}

/// @param o { title, sub, back: { id, label } , action (markup), level=2, id, cls }
export function section(o = {}){
  const lvl = o.level === 3 ? 'h3' : 'h2';
  const back = o.back ? iconButton({ id: o.back.id, icon: 'arrow-left', ariaLabel: o.back.label, variant: 'plain' }) : '';
  const a = merge({ id: o.id, class: cx('ui-section-head', o.cls) }, o.attrs);
  return `<header${attrs(a)}>${back}<div class="ui-section-titles">${label(o.title, lvl, 'ui-title')}${label(o.sub, 'p', 'ui-sub')}</div>${o.action || ''}</header>`;
}

/// @param v text or { t:key }; o { hint, center, cls }
export function para(v, o = {}){
  return label(v, 'p', cx(o.hint ? 'ui-hint' : 'ui-text', o.center && 'ui-center', o.cls));
}
