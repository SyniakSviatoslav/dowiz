// The staff -- who works at the venue: kitchen, waiter, counter-manager.
//
// A staff member is invited by email with a code that lives seven days; the
// staff member types it into the room app and sets their own password. The list
// shows each one with their role; a tap opens the staff member: their role and
// whether they are active. The owner may change role or suspend/restore.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, busy, switchEl, confirm } from '/admin/core.js';
import { loadStaff, rerender } from '/admin/app.js';

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabStaff"></p><h1 data-t="tabStaff"></h1></div>
    <button type="button" class="act pri" id="invite">${icon('user-plus')}<span data-t="inviteStaff"></span></button></div>
    <p class="screen-hint" data-t="staffHint"></p>
    <div id="slist"><div class="skel skel-row"></div></div>`;
  $('#invite', host).onclick = openInvite;
  let d;
  try { d = await api('/owner/staff'); S.staff = d.staff || []; } catch (e) { $('#slist', host).innerHTML = `<div class="empty">${icon('alert-triangle')}<b>${esc(t('loadFail'))}</b></div>`; return; }
  const staff = d.staff || [], inv = d.invites || [];
  $('#slist', host).innerHTML = `
    ${staff.length ? `<div class="rows">${staff.map(s => `<button type="button" class="rowc ${s.active ? '' : 'off'}" data-s="${esc(s.id)}">${icon('apron')}
      <span class="t"><b>${esc(s.name)}</b><small>${esc(t('role-' + (s.role || 'unknown')))}</small></span>${icon('chevron-right', 'chev')}</button>`).join('')}</div>`
      : `<div class="empty">${icon('apron')}<b data-t="noStaff"></b><span class="muted small" data-t="staffHint"></span></div>`}
    ${inv.length ? `<div class="group mt-3"><p class="eyebrow" data-t="inviteStaff"></p><div class="rows">${inv.map(i => `<div class="rowc ${i.expired ? 'off' : ''}">${icon('ticket')}<span class="t"><b>${esc(i.name)}</b><small>${esc(t(i.expired ? 'inviteExpired' : 'inviteWaiting'))} · ${esc(new Date(i.untilMs).toLocaleDateString())}</small></span></div>`).join('')}</div></div>` : ''}`;
  host.onclick = async e => {
    const r = e.target.closest('[data-s]'); if (r) openStaff(r.dataset.s);
  };
}

function openInvite(){
  sheet(`<p class="eyebrow" data-t="tabStaff"></p><h2 data-t="inviteStaff"></h2><p class="muted small" data-t="staffHint"></p>
    <label for="i-email" data-t="email"></label><input id="i-email" type="email" autocomplete="off">
    <label for="i-name" data-t="name"></label><input id="i-name" autocomplete="off">
    <label for="i-role" data-t="role"></label><select id="i-role">
      <option value="kitchen" data-t="role-kitchen"></option>
      <option value="waiter" data-t="role-waiter"></option>
      <option value="counter-manager" data-t="role-counter-manager"></option>
    </select>
    <div class="btn-row"><button class="btn" id="iGo">${icon('user-plus')}<span data-t="inviteStaff"></span></button></div>
    <div id="iOut"></div>`, { name: 'invite' });
  // Translate role options
  for (const opt of $$('#i-role option')) opt.textContent = t(opt.dataset.t);
  $('#iGo').onclick = async () => {
    try {
      const d = await busy($('#iGo'), () => post('/owner/staff/invite', {
        email: $('#i-email').value.trim(),
        name: $('#i-name').value.trim(),
        role: $('#i-role').value
      }));
      $('#iOut').innerHTML = `<p class="eyebrow mt-3" data-t="inviteCode"></p><div class="code" id="iCode">${esc(d.code)}</div><p class="hint" data-t="keyOnce"></p>
        <button class="btn ghost mt-2" id="iCopy">${icon('copy')}<span data-t="copy"></span></button>`;
      for (const el of $$('[data-t]', $('#iOut'))) el.textContent = t(el.dataset.t);
      $('#iCopy').onclick = async () => { try { await navigator.clipboard.writeText(d.code); toast(t('copied')); } catch {} };
      loadStaff().then(rerender);
    } catch (e) { toast(String(e.message || e)); }
  };
}

async function openStaff(id){
  const s = S.staff.find(x => x.id === id) || { id };
  sheet(`<p class="eyebrow" data-t="staff"></p><h2>${esc(s.name || id)}</h2><div id="sBody"><div class="skel skel-row"></div></div>`, { name: 'staff' });
  // The list already carries id, name, role, active: there is no per-person
  // GET. An invite is withdrawn by inviting the same email again (the backend
  // revokes the pending one), so there is no revoke route to call either.
  const staff = s;
  $('#sBody').innerHTML = `
    <div class="fact">${icon('tag')}<span class="v"><span class="k" data-t="role"></span><span data-t="role-${esc(staff.role || 'unknown')}"></span></span></div>
    ${switchEl('s-active', staff.active ?? s.active, 'active')}
    <label for="s-role" data-t="role"></label><select id="s-role">
      <option value="kitchen" data-t="role-kitchen"></option>
      <option value="waiter" data-t="role-waiter"></option>
      <option value="counter-manager" data-t="role-counter-manager"></option>
    </select>
    <button type="button" class="btn mt-3" id="sGo">${icon('check')}<span data-t="save"></span></button>`;
  for (const el of $$('[data-t]', $('#sBody'))) el.textContent = t(el.dataset.t);
  $('#s-role').value = staff.role || 'kitchen';
  $('#s-role').onchange = async () => {
    try {
      await post(`/owner/staff/${encodeURIComponent(id)}`, { role: $('#s-role').value, ...withLoc() });
      toast(t('saved'));
      loadStaff().then(() => { rerender(); openStaff(id); });
    } catch (err) { toast(String(err.message || err)); $('#s-role').value = staff.role; }
  };
  $('#s-active').onchange = async e => {
    if (!e.target.checked) {
      const ok = await confirm(t('deactivate'), t('deactivateStaff'), { danger: true });
      if (!ok) return openStaff(id);
    }
    try {
      await post(`/owner/staff/${encodeURIComponent(id)}`, { active: e.target.checked, ...withLoc() });
      toast(t('saved'));
      loadStaff().then(() => { rerender(); openStaff(id); });
    } catch (err) { toast(String(err.message || err)); e.target.checked = !e.target.checked; }
  };
}
