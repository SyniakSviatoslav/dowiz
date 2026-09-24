// Lists and rows. A row is the unit of almost every screen in this product:
// an order, a dish, a delivery, a day's takings. One shape, three uses.
//
//   row()            a plain row: leading, title + sub, trailing
//   row({ select })  a SELECTABLE row: a real <button> with aria-pressed and a
//                    drawn radio mark (CSS, not an icon -- the icons the courier
//                    used for this, `circle` and `circle-check-filled`, were never
//                    in /lib/icons.css, so the mark drew nothing)
//   row({ href })    a row that navigates
//   row({ act })     a row that ACTS (opens a sheet, starts a tour): a plain
//                    <button>, no aria-pressed, no radio mark
//   list(rows)       the container, role="list" when rows are not buttons
//
// `trailing` and `leading` are MARKUP (usually `amount()` or `badge()`), so a
// row never formats money itself. Title and sub are text and are escaped.
import { cx, attrs, merge, label } from './core.js';

/// @param o { title, sub, leading, trailing, select, pressed, href, id,
///            data, cls, attrs, strong, act }
export function row(o = {}){
  const body = `<span class="ui-row-body">${label(o.title, 'span', cx('ui-row-title', o.strong && 'ui-row-title--strong'))}${label(o.sub, 'span', 'ui-row-sub')}</span>`;
  const lead = o.select
    ? '<span class="ui-radio" aria-hidden="true"></span>'
    : (o.leading ? `<span class="ui-row-lead">${o.leading}</span>` : '');
  const trail = o.trailing ? `<span class="ui-row-trail">${o.trailing}</span>` : '';
  const cls = cx('ui-row', (o.select || o.href || o.act) && 'ui-row--action', o.cls);
  if (o.select) {
    const a = merge({ id: o.id, type: 'button', class: cls, 'aria-pressed': String(!!o.pressed), data: o.data }, o.attrs);
    return `<button${attrs(a)}>${lead}${body}${trail}</button>`;
  }
  if (o.act) {
    const a = merge({ id: o.id, type: 'button', class: cls, data: o.data }, o.attrs);
    return `<button${attrs(a)}>${lead}${body}${trail}</button>`;
  }
  if (o.href) {
    const a = merge({ id: o.id, href: o.href, class: cls, data: o.data }, o.attrs);
    return `<a${attrs(a)}>${lead}${body}${trail}</a>`;
  }
  const a = merge({ id: o.id, class: cls, role: 'listitem', data: o.data }, o.attrs);
  return `<div${attrs(a)}>${lead}${body}${trail}</div>`;
}

/// @param rows array of `row()` markup; o { label, id, cls, inset }
///   A list of selectable rows is a GROUP of buttons, not a list of items, so
///   it gets role="group" and the caller's label.
export function list(rows = [], o = {}){
  const interactive = rows.some(r => /^<(button|a)\b/.test(r));
  const a = merge({ id: o.id, class: cx('ui-list', o.inset && 'ui-list--inset', o.cls),
                    role: interactive ? 'group' : 'list', 'aria-label': o.label || null }, o.attrs);
  return `<div${attrs(a)}>${rows.join('')}</div>`;
}
