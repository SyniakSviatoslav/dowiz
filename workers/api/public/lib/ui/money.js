// Money on screen. The formatter is /lib/money.js and only /lib/money.js; this
// module does not format, it PRESENTS: the `.money` class (mono, tabular, from
// lib/components.css -- the one place money is styled) and nothing that moves.
//
// "Money never tweens" (plan §8.4.3) is enforced by construction: there is no
// animated variant, and ui.css gives `.ui-amount` no transition.
//
// The value passed in is ALREADY FORMATTED by the surface's `money()`; a raw
// number is refused, because a number reaching the screen without the formatter
// is exactly how 1500 lek was once drawn as $15.00.
import { esc, cx, attrs, merge, label, tone } from './core.js';

const SIZES = ['sm', 'md', 'lg', 'xl'];

/// @param formatted  the string from the surface's money formatter
/// @param o { size='md', tone, sign, strong, id, cls, ariaLabel }
///   `sign` prefixes '+' or '−' as TEXT (a tip is +, a shortfall is −).
export function amount(formatted, o = {}){
  if (typeof formatted !== 'string') throw new Error('ui.amount: pass the formatted string from /lib/money.js, not a number');
  const size = o.size || 'md';
  if (!SIZES.includes(size)) throw new Error(`ui.amount: unknown size ${size}`);
  const tn = o.tone ? tone(o.tone) : null;
  const a = merge({ id: o.id, class: cx('money', 'ui-amount', `ui-amount--${size}`, tn && `ui-amount--${tn}`, o.strong && 'ui-amount--strong', o.cls),
                    'aria-label': o.ariaLabel || null }, o.attrs);
  const sign = o.sign === '+' ? '+' : o.sign === '-' ? '−' : '';
  return `<span${attrs(a)}>${sign}${esc(formatted)}</span>`;
}

/// A labelled figure: "Cash in hand  4 500 L". The value is markup (usually
/// `amount()`), the label is text.
/// @param o { label, value, hint, emphasis, id, cls }
export function stat(o = {}){
  const a = merge({ id: o.id, class: cx('ui-stat', o.emphasis && 'ui-stat--hero', o.cls) }, o.attrs);
  return `<div${attrs(a)}>${label(o.label, 'span', 'ui-stat-k')}<span class="ui-stat-v">${o.value ?? ''}</span>${label(o.hint, 'span', 'ui-stat-hint')}</div>`;
}
