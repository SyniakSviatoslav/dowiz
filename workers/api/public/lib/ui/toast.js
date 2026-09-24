// Toasts: a short, transient notice about something that just happened.
//
// ONE HOST PER PAGE, present in the HTML from the start with role="status" and
// aria-live, because a live region that is CREATED at the moment of the notice
// is not announced by most screen readers -- it has to exist first. The host's
// markup is `toastHost()`; a surface that already has a `#toast` element just
// gives it the `ui-toast` class.
//
// `hidden` is the state (display:none), not opacity 0, so nothing is parked
// invisible over the content; the entry animates from @starting-style in
// ui.css. The text is ALWAYS escaped -- a toast often carries a server message.
import { esc, cx, icon, tone as checkTone } from './core.js';

export const TOAST_MS = 3000;

/// The host element, for a page that does not have one yet.
export function toastHost(id = 'toast'){
  return `<div class="ui-toast" id="${esc(id)}" role="status" aria-live="polite" aria-atomic="true" hidden></div>`;
}

/// A toaster bound to one host. `show(message, { icon, tone, ms })`.
/// `timers` is injectable so tests do not wait three seconds.
export function createToaster(host, { ms = TOAST_MS, timers = globalThis } = {}){
  let timer = null;
  function show(message, o = {}){
    if (!host) return;
    const tn = checkTone(o.tone, 'neutral');
    host.className = cx('ui-toast', tn !== 'neutral' && `ui-toast--${tn}`);
    host.innerHTML = `${icon(o.icon || 'info-circle')}<span>${esc(message)}</span>`;
    host.hidden = false;
    timers.clearTimeout(timer);
    timer = timers.setTimeout(() => { host.hidden = true; }, o.ms ?? ms);
  }
  function hide(){ timers.clearTimeout(timer); if (host) host.hidden = true; }
  return { show, hide };
}
