// The staff -- who works at the venue: kitchen, waiter, counter-manager.
//
// A staff member is invited by email with a code that lives seven days; the
// staff member types it into the room app and sets their own password. The list
// shows each one with their role; a tap opens the staff member: their role and
// whether they are active. The owner may change role or suspend/restore.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, busy, switchEl, confirm } from '/admin/core.js';
import { loadStaff, rerender } from '/admin/app.js';
import { btn, field, select, empty, loading, rowBtn, rowDiv } from '/admin/parts.js';

/// The three roles a venue hires, as the select offers them.
const ROLES = ['kitchen', 'waiter', 'counter-manager'].map(r => ({ value: r, key: 'role-' + r }));

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabStaff"></p><h1 data-t="tabStaff"></h1></div>
    ${btn({ id: 'invite', variant: 'primary', icon: 'user-plus', key: 'inviteStaff', tour: 'staff.invite' })}</div>
    <p class="screen-hint" data-t="staffHint"></p>
    <div id="slist" data-tour="staff.list">${loading(3)}</div>`;
  $('#invite', host).onclick = openInvite;
  let d;
  try { d = await api('/owner/staff'); S.staff = d.staff || []; } catch (e) { $('#slist', host).innerHTML = empty('alert-triangle', { key: 'loadFail', alert: true }); return; }
  const staff = d.staff || [], inv = d.invites || [];
  $('#slist', host).innerHTML = `
    ${staff.length ? `<div class="rows">${staff.map(s => rowBtn({ cls: s.active ? '' : 'off', data: { s: s.id }, tour: 'staff.card', leading: icon('apron'),
      title: s.name, sub: esc(t('role-' + (s.role || 'unknown'))), trailing: icon('chevron-right', 'chev') })).join('')}</div>`
      : empty('apron', { key: 'noStaff', bodyKey: 'staffHint' })}
    ${inv.length ? `<div class="group mt-3"><p class="eyebrow" data-t="inviteStaff"></p><div class="rows">${inv.map(i => rowDiv({ cls: i.expired ? 'off' : '', leading: icon('ticket'),
      title: i.name, sub: `${esc(t(i.expired ? 'inviteExpired' : 'inviteWaiting'))} · ${esc(new Date(i.untilMs).toLocaleDateString())}` })).join('')}</div></div>` : ''}`;
  host.onclick = async e => {
    const r = e.target.closest('[data-s]'); if (r) openStaff(r.dataset.s);
  };
}

function openInvite(){
  sheet(`<p class="eyebrow" data-t="tabStaff"></p><h2 data-t="inviteStaff"></h2><p class="muted small" data-t="staffHint"></p>
    ${field({ id: 'i-email', key: 'email', type: 'email', autocomplete: 'off', tour: 'staff.inviteEmail' })}
    ${field({ id: 'i-name', key: 'name', autocomplete: 'off', tour: 'staff.inviteName' })}
    ${select({ id: 'i-role', key: 'role', value: 'kitchen', options: ROLES, tour: 'staff.inviteRole' })}
    <div class="btn-row">${btn({ id: 'iGo', variant: 'primary', icon: 'user-plus', key: 'inviteStaff', tour: 'staff.inviteSend' })}</div>
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
        ${btn({ id: 'iCopy', variant: 'ghost', icon: 'copy', key: 'copy', cls: 'mt-2', tour: 'staff.inviteCopy' })}`;
      for (const el of $$('[data-t]', $('#iOut'))) el.textContent = t(el.dataset.t);
      $('#iCopy').onclick = async () => { try { await navigator.clipboard.writeText(d.code); toast(t('copied')); } catch {} };
      loadStaff().then(rerender);
    } catch (e) { toast(String(e.message || e)); }
  };
}

async function openStaff(id){
  const s = S.staff.find(x => x.id === id) || { id };
  sheet(`<p class="eyebrow" data-t="staff"></p><h2>${esc(s.name || id)}</h2><div id="sBody">${loading(1)}</div>`, { name: 'staff' });
  // The list already carries id, name, role, active: there is no per-person
  // GET. An invite is withdrawn by inviting the same email again (the backend
  // revokes the pending one), so there is no revoke route to call either.
  const staff = s;
  $('#sBody').innerHTML = `
    <div class="fact">${icon('tag')}<span class="v"><span class="k" data-t="role"></span><span data-t="role-${esc(staff.role || 'unknown')}"></span></span></div>
    ${switchEl('s-active', staff.active ?? s.active, 'active', null, 'staff.active')}
    ${select({ id: 's-role', key: 'role', value: staff.role || 'kitchen', options: ROLES, tour: 'staff.role' })}
    ${btn({ id: 'sGo', variant: 'primary', icon: 'check', key: 'save', cls: 'mt-3', tour: 'staff.save' })}`;
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
