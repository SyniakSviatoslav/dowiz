// Time on screen, in the reader's language. A `<time datetime>` carries the
// machine value, so a screen reader and a copy-paste get the instant and the
// eye gets "24 Sep" / "14:05".
//
// The clock is NOT read here: the caller passes the instant. A component that
// reads `Date.now()` renders differently every time it is tested, and a venue's
// day is the VENUE's (see /lib/booking-time.js), which only the caller knows.
import { esc, cx } from './core.js';

const STYLES = {
  time: { hour: '2-digit', minute: '2-digit' },
  date: { day: 'numeric', month: 'short' },
  datetime: { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' },
};

/// @param at  ms since epoch, or a Date
/// @param o { locale='en', style='time'|'date'|'datetime', timeZone, cls }
export function when(at, o = {}){
  const d = at instanceof Date ? at : new Date(Number(at));
  if (Number.isNaN(d.getTime())) throw new Error('ui.when: not an instant');
  const style = STYLES[o.style || 'time'];
  if (!style) throw new Error(`ui.when: unknown style ${o.style}`);
  const fmt = d.toLocaleString(o.locale || 'en', { ...style, ...(o.timeZone ? { timeZone: o.timeZone } : {}) });
  return `<time class="${esc(cx('ui-time', o.cls))}" datetime="${esc(d.toISOString())}">${esc(fmt)}</time>`;
}

/// "4:05" from seconds: a countdown's face. Integer arithmetic only.
export function mmss(seconds){
  const s = Math.max(0, Math.floor(Number(seconds) || 0));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}
