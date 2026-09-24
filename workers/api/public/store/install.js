// "Put it on your home screen": offered once the menu has arrived, as a small
// sheet with the venue's mark, and never again once the customer says so.
//
// Chrome hands over an install prompt; iOS never does and is told the two
// taps instead. Inside an installed app the offer is pointless and is not
// made. The decision to stop asking is the customer's: a checkbox, kept in
// this browser only.

import { state } from '/store/state.js';
import { t } from '/store/i18n.js';
import { $, esc, icon, sheet, closeSheet, sheetName, toast } from '/store/ui.js';
import { safeGet, safeSet } from '/store/storage.js';
import { k, cta, ghost } from '/store/parts.js';

/// The customer's "don't show again", per venue.
const HIDE_KEY = 'dw_install_hide';
/// Asked once per visit at most; the key lives for the tab's life.
const ASKED_KEY = 'dw_install_asked';
/// The spread lands first; the offer rises after it, not over it.
const OFFER_AFTER_MS = 1800;
const IOS = /iPad|iPhone|iPod/.test(navigator.userAgent) && !window.MSStream;

let app = null;
export function bindInstall(mod){ app = mod; }

const hidden = () => safeGet(HIDE_KEY) === '1';
const asked = () => { try { return sessionStorage.getItem(ASKED_KEY) === '1'; } catch { return false; } };
const markAsked = () => { try { sessionStorage.setItem(ASKED_KEY, '1'); } catch {} };

/// Offer after the loader has gone, unless installed, declined for good, or already asked this visit.
export function offerInstall(){
  if (!app || app.isStandalone() || hidden() || asked()) return;
  // Nothing to offer on a desktop browser that will not install; Chrome on a
  // phone fires its event, iOS gets its hint.
  if (!app.canInstall() && !IOS) {
    // Chrome may still fire the event a moment later; wait for it once.
    const once = () => { removeEventListener('dw:installable', once); if (app.canInstall() && !asked() && !sheetName()) setTimeout(open, OFFER_AFTER_MS); };
    addEventListener('dw:installable', once);
    return;
  }
  setTimeout(() => { if (!sheetName()) open(); }, OFFER_AFTER_MS);
}

function open(){
  markAsked();
  const name = state.loc?.name || 'dowiz', logo = state.loc?.logoUrl;
  sheet(`<div class="install-pop">
      ${logo ? `<img class="install-mark" src="${esc(logo)}" alt="">` : `<span class="install-mark install-mark-ink">${icon('bento')}</span>`}
      <p class="eyebrow" data-t="installApp"></p>
      <h2>${esc(name)}</h2>
      <p class="muted small" data-t="installBody"></p>
      ${app.canInstall() ? cta({ id: 'insGo', cls: 'mb-1', icon: 'bento', label: k('installNow') }) : `<p class="install-ios">${icon('share')}<span data-t="installHint"></span></p>`}
      ${ghost({ id: 'insLater', cls: 'mb-2', label: k('installLater') })}
      <label class="install-never"><input type="checkbox" id="insNever"><span data-t="installNever"></span></label>
    </div>`, { name: 'install' });
  const remember = () => { if ($('#insNever')?.checked) safeSet(HIDE_KEY, '1'); };
  const go = $('#insGo');
  if (go) go.onclick = async () => { remember(); if (await app.promptInstall()) { toast(t('installed')); closeSheet(); } };
  $('#insLater').onclick = () => { remember(); closeSheet(); };
  $('#insNever').onchange = e => { if (e.target.checked) safeSet(HIDE_KEY, '1'); else { try { localStorage.removeItem(HIDE_KEY); } catch {} } };
}
