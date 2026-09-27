// PASSWORDS FOR STAFF (operator 2026-09-27): production had no way to change
// one. Two sheets:
//   openOwn()        -- a member of staff changes their own, from the profile
//                       sheet (POST /api/staff/password; `signin.js`).
//   openReset(id, n) -- the owner sets a new one for a member of staff of THEIR
//                       venue, from the staff card (POST /api/owner/staff/:id/password).
// The hub checks everything (the old password, the length, the venue); these
// sheets only say its words.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { $, esc, t, post, toast, sheet, closeSheet, busy, store, withLoc } from '/admin/core.js';
import { btn, field } from '/admin/parts.js';
import { changePassword, MIN_PASSWORD_CHARS } from '/admin/signin.js';

/// A refusal in words: the two the page decides itself, else the hub's own.
const said = e => { const m = String((e && e.message) || e); return m === 'missing' ? t('acc_missing') : m === 'short' ? t('acc_short') : m; };

/// The signed-in member of staff changes their own password. The hub answers
/// with a fresh session; the console carries on with it.
export function openOwn(){
  sheet(`<h2 data-t="acc_changePw"></h2><p class="sheet-hint" data-t="acc_pwHint"></p>
    ${field({ id: 'pwEmail', key: 'email', type: 'email', autocomplete: 'username', inputmode: 'email' })}
    ${field({ id: 'pwOld', key: 'acc_oldPw', type: 'password', autocomplete: 'current-password' })}
    ${field({ id: 'pwNew', key: 'acc_newPw', type: 'password', autocomplete: 'new-password' })}
    <p class="hint" id="pwOut"></p>
    <div class="btn-row">${btn({ id: 'pwGo', variant: 'primary', icon: 'key', key: 'acc_changePw' })}</div>`, { name: 'password' });
  $('#pwGo').onclick = async () => {
    try {
      const s = await busy($('#pwGo'), () => changePassword(fetch.bind(globalThis), $('#pwEmail').value, $('#pwOld').value, $('#pwNew').value));
      store.t = s.token; store.loc = s.loc;
      toast(t('acc_pwChanged')); closeSheet();
    } catch (e) { $('#pwOut').textContent = said(e); }
  };
}

/// The owner sets a new password for one member of staff of this venue.
export function openReset(id, name = ''){
  sheet(`<p class="eyebrow" data-t="staff"></p><h2>${esc(name || id)}</h2><p class="sheet-hint" data-t="acc_setPwHint"></p>
    ${field({ id: 'rpNew', key: 'acc_newPw', type: 'password', autocomplete: 'new-password' })}
    <p class="hint" id="rpOut"></p>
    <div class="btn-row">${btn({ id: 'rpGo', variant: 'primary', icon: 'key', key: 'acc_setPw' })}</div>`, { name: 'password' });
  $('#rpGo').onclick = async () => {
    const pw = $('#rpNew').value;
    if ([...pw].length < MIN_PASSWORD_CHARS) { $('#rpOut').textContent = t('acc_short'); return; }
    try {
      await busy($('#rpGo'), () => post(`/owner/staff/${encodeURIComponent(id)}/password`, withLoc({ new_password: pw })));
      toast(t('acc_pwChanged')); closeSheet();
    } catch (e) { $('#rpOut').textContent = said(e); }
  };
}
