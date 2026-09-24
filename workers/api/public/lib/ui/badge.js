// Badges: a short, NON-interactive label. Two kinds.
//
//   badge()   a tone pill: "Paid online", "3 unsent", "Offered to you".
//   status()  an ORDER status: a coloured dot + an ink label. The dot is the
//             only thing that carries the status hue, because several status
//             hues fail as text (IN_DELIVERY #3B82F6 is 3.68:1 on white); the
//             word is always ink. The hue comes from `--st-<STATUS>` (DOWIZ-FIXED
//             per plan §8.1 -- the lifecycle looks the same for every brand).
//
// Nothing here is a button. Something tappable that looks like this is a chip.
import { cx, attrs, merge, icon, label, tone } from './core.js';

/// @param o { label, tone='neutral', icon, dot=false, id, cls, attrs, live }
///   `live` marks a region that announces itself (a counter that changes).
export function badge(o = {}){
  const tn = tone(o.tone);
  const a = merge({ id: o.id, class: cx('ui-badge', `ui-badge--${tn}`, o.cls) },
                  o.live ? { role: 'status', 'aria-live': 'polite' } : null, o.attrs);
  return `<span${attrs(a)}>${o.dot ? '<span class="ui-dot" aria-hidden="true"></span>' : ''}${icon(o.icon)}${label(o.label)}</span>`;
}

// The statuses the order FSM can name. A status outside this list renders
// neutral rather than throwing: the hub may learn a state before the client.
export const STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'PICKED_UP', 'IN_DELIVERY',
  'DELIVERED', 'REJECTED', 'CANCELLED', 'SCHEDULED', 'REFUNDING', 'COMPENSATED_REFUND'];

/// @param o { label, status, pulse=false, id, cls }
///   `pulse` animates the dot (reduced motion: solid) for a state in motion.
export function status(o = {}){
  const s = STATUSES.includes(o.status) ? o.status : 'UNKNOWN';
  const a = merge({ id: o.id, class: cx('ui-status', o.pulse && 'ui-status--pulse', o.cls), data: { status: s } }, o.attrs);
  return `<span${attrs(a)}>${label(o.label)}</span>`;
}
