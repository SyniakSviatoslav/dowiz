// Installing the app — the offer, and the honesty about what each browser does.
//
// Chrome and Edge fire `beforeinstallprompt` when the site qualifies (manifest,
// a worker with a fetch handler, HTTPS). Holding that event and calling
// `prompt()` from a tap is the only supported way to show the real dialog; the
// event cannot be replayed after it is used, so it is captured once and the
// affordance disappears with it.
//
// iOS Safari fires nothing and has no API. It installs through Share → Add to
// Home Screen, so the honest thing is to SAY that rather than show a button
// that cannot work. Detecting "iOS" is done by the two things that actually
// matter — a touch device that has no install event — never by sniffing a
// version out of the user agent.
//
// Nobody is asked twice in a month. A banner that returns on every load is an
// advert, and this one has a job.

const SNOOZE_KEY = 'dowiz.install.snoozed';
const MONTH = 30 * 24 * 60 * 60 * 1000;

let deferred = null;                // the captured beforeinstallprompt event

export const installed = () =>
  matchMedia('(display-mode: standalone)').matches
  || matchMedia('(display-mode: minimal-ui)').matches
  || navigator.standalone === true;          // iOS's own flag

const snoozed = () => {
  try {
    const at = Number(localStorage.getItem(SNOOZE_KEY) || 0);
    return at > 0 && Date.now() - at < MONTH;
  } catch { return false; }                  // private mode: offer it anyway
};
const snooze = () => { try { localStorage.setItem(SNOOZE_KEY, String(Date.now())); } catch {} };

/** Can this browser show the real dialog right now? */
export const canPrompt = () => !!deferred;

/** Does this browser install by hand, through its own share menu? */
// Three things, not two. "A touch device with no install event" also described
// Android Chrome in the moment before `beforeinstallprompt` fires and every
// headless test run -- both were shown a Share → Add to Home Screen instruction
// that is Safari's and does not exist on their menus. Every browser on iOS is
// WebKit and reports Apple as its vendor (Chrome and Firefox on iOS included,
// and they install the same way), so the vendor is the platform check. It is a
// platform, not a version: nothing here parses a number out of the user agent.
export const byHand = () => !deferred && !installed()
  && matchMedia('(hover: none) and (pointer: coarse)').matches
  && navigator.vendor === 'Apple Computer, Inc.';

/** Show the platform dialog. Returns 'accepted', 'dismissed' or 'unavailable'. */
export async function promptInstall() {
  if (!deferred) return 'unavailable';
  const e = deferred;
  deferred = null;                           // a captured event is single-use
  e.prompt();
  const { outcome } = await e.userChoice;
  if (outcome !== 'accepted') snooze();
  return outcome;
}

// ── The banner ────────────────────────────────────────────────────────────────
// It is built with the DOM rather than innerHTML because `style-src 'self'`
// forbids a style attribute, and because this is the one piece of UI that has
// to appear over whatever screen happens to be open.
//
// ONE ROW. It was icon-and-text over a row of two buttons, 130px tall, and it
// sat 96px up the screen: on every screen it covered the bottom third of the
// content, and the last card of a list could never be scrolled out from under
// it. Now it is icon, two short lines, and the actions, on the navbar's top
// edge; kit.css reserves the banner's height at the foot of the page while it
// is up. "Not now" is a round × rather than a word so the row fits at 320.

function banner(body, action) {
  const el = document.createElement('div');
  el.className = 'k-install';
  el.setAttribute('role', 'dialog');
  el.setAttribute('aria-label', 'Встановити застосунок');

  const img = document.createElement('img');
  img.src = '/kit/img/icon-192.png';
  img.alt = '';
  img.width = 40; img.height = 40;
  img.className = 'k-install-ico';

  const text = document.createElement('div');
  text.className = 'k-install-text';
  const h = document.createElement('strong');
  h.textContent = 'dowiz на головний екран';
  const p = document.createElement('span');
  p.textContent = body;
  text.append(h, p);

  const row = document.createElement('div');
  row.className = 'k-install-row';
  if (action) row.append(action);
  const no = document.createElement('button');
  no.type = 'button';
  no.className = 'k-install-no';
  no.textContent = '\u00d7';
  no.setAttribute('aria-label', 'Не зараз');
  no.addEventListener('click', () => { snooze(); el.remove(); });
  row.append(no);

  el.append(img, text, row);
  return el;
}

function offer() {
  if (installed() || snoozed() || document.querySelector('.k-install')) return;

  if (canPrompt()) {
    const yes = document.createElement('button');
    yes.type = 'button';
    yes.className = 'k-install-yes';
    yes.textContent = 'Встановити';
    yes.addEventListener('click', async () => {
      const el = yes.closest('.k-install');
      const outcome = await promptInstall();
      if (outcome !== 'dismissed') el?.remove();
    });
    document.body.append(banner('Працює як застосунок, і без мережі.', yes));
    return;
  }

  if (byHand()) {
    // No button, because there is no API behind one. The instruction is the UI.
    document.body.append(banner('Поділитися → На початковий екран.', null));
  }
}

// ── Wiring ───────────────────────────────────────────────────────────────────
addEventListener('beforeinstallprompt', e => {
  e.preventDefault();                        // keep the browser's own mini-bar away
  deferred = e;
  offer();
});

addEventListener('appinstalled', () => {
  deferred = null;
  document.querySelector('.k-install')?.remove();
  snooze();                                  // never offer again on this device
});

// Registration is deliberately late: a worker installing during the first paint
// competes with the modules and the font for the same connection.
export function register() {
  if (!('serviceWorker' in navigator)) return;
  addEventListener('load', () => {
    navigator.serviceWorker.register('/kit/sw.js', { scope: '/kit/' }).catch(err => {
      // Not fatal — the app works without it. But say so, because "why is it
      // not installable" is otherwise unanswerable.
      console.warn('service worker not registered:', err && err.message);
    });
  });
}

// iOS gives no event, so the hand-install offer is made once the page settles
// rather than in response to something the browser said.
addEventListener('load', () => { if (byHand()) setTimeout(offer, 1200); });
