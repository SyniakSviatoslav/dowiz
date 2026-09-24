// Empty states: "there is nothing here, and this is why / what to do".
// The opposite of a skeleton (skeleton.js), which says something is coming.
// A FAILURE is an empty state too, and it is announced (`alert` -> role=alert);
// a state that CHANGED under the reader (`status`) is role=status; a quiet
// "nothing yet" carries no role -- the container it lands in decides.
import { cx, attrs, merge, icon, label, tone } from './core.js';

/// @param o { icon, title, body, tone, action (markup), alert, id, cls }
///   `alert: true` -> role="alert", `status: true` -> role="status".
export function emptyState(o = {}){
  const a = merge({ id: o.id, class: cx('ui-empty', o.tone && `ui-empty--${tone(o.tone)}`, o.cls),
                    role: o.alert ? 'alert' : o.status ? 'status' : null }, o.attrs);
  const reason = o.reason ? label(o.reason, 'span', 'ui-empty-reason') : '';
  return `<div${attrs(a)}>${icon(o.icon, 'ui-empty-i')}${label(o.title, 'b', 'ui-empty-title')}${label(o.body, 'span', 'ui-empty-body')}${reason}${o.action || ''}</div>`;
}
