// The couriers -- who rides for the venue, who is on shift, where they are.
//
// A courier is invited by phone with a code that lives seven days; the
// courier types it into their own app and sets their own password. The list
// shows each one with a shift chip; a tap opens the courier: today's runs,
// the last place they were seen (from the same fixes the live estimate
// uses), and the switch that lets them take orders at all.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, busy, ago, switchEl, money, confirm } from '/admin/core.js';
import { loadCouriers, rerender } from '/admin/app.js';
import { btn, iconBtn, field, pill, empty, loading, rowBtn, rowDiv } from '/admin/parts.js';

/// A courier's last fix is shown as a map link at this zoom.
const MAP_ZOOM = 16;

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><h1 data-t="tabCouriers"></h1></div>
    ${btn({ id: 'invite', variant: 'primary', icon: 'user-plus', key: 'invite', tour: 'couriers.invite' })}</div>
    <p class="screen-hint" data-t="couriersHint"></p>
    <div id="clist" data-tour="couriers.list">${loading(3)}</div>`;
  $('#invite', host).onclick = openInvite;
  let d;
  try { d = await api('/owner/couriers'); S.couriers = d.couriers || []; } catch (e) { $('#clist', host).innerHTML = empty('alert-triangle', { key: 'loadFail', alert: true }); return; }
  const cs = d.couriers || [], inv = d.invites || [];
  $('#clist', host).innerHTML = `
    ${cs.length ? `<div class="rows">${cs.map(c => rowBtn({ cls: c.active ? '' : 'off', data: { c: c.id }, tour: 'couriers.card', leading: icon('bike'),
      title: c.name || c.phone || c.id, sub: `<span class="mono">${esc(c.phone || '')}</span>`,
      trailing: `${pill(c.onShift ? 'ok' : '', { key: c.onShift ? 'onShift' : 'offShift', tour: 'couriers.shift' })}${icon('chevron-right', 'chev')}` })).join('')}</div>`
      : empty('bike', { key: 'noCouriers', bodyKey: 'inviteHint' })}
    ${inv.length ? `<div class="group mt-3"><p class="eyebrow" data-t="invite"></p><div class="rows">${inv.map(i => rowDiv({ cls: i.expired ? 'off' : '', leading: icon('ticket'),
      title: i.name, sub: `${esc(t(i.expired ? 'inviteExpired' : 'inviteWaiting'))} · ${esc(new Date(i.untilMs).toLocaleDateString())}`,
      trailing: iconBtn({ icon: 'x', ariaKey: 'remove', data: { uninvite: i.id }, tour: 'couriers.uninvite' }) })).join('')}</div></div>` : ''}`;
  host.onclick = async e => {
    const u = e.target.closest('[data-uninvite]'); if (u) { try { await busy(u, () => post(`/owner/couriers/${encodeURIComponent(u.dataset.uninvite)}/uninvite`, withLoc())); rerender(); } catch (err) { toast(String(err.message || err)); } return; }
    const r = e.target.closest('[data-c]'); if (r) openCourier(r.dataset.c);
  };
}

function openInvite(){
  sheet(`<p class="eyebrow" data-t="tabCouriers"></p><h2 data-t="invite"></h2><p class="muted small" data-t="inviteHint"></p>
    ${field({ id: 'i-name', key: 'name', autocomplete: 'off', tour: 'couriers.inviteName' })}
    ${field({ id: 'i-phone', key: 'phone', type: 'tel', inputmode: 'tel', placeholder: '+355…', tour: 'couriers.invitePhone' })}
    <div class="btn-row">${btn({ id: 'iGo', variant: 'primary', icon: 'user-plus', key: 'invite', tour: 'couriers.inviteSend' })}</div>
    <div id="iOut"></div>`, { name: 'invite' });
  $('#iGo').onclick = async () => {
    try {
      const d = await busy($('#iGo'), () => post('/owner/couriers/invite', { phone: $('#i-phone').value.trim(), name: $('#i-name').value.trim() }));
      $('#iOut').innerHTML = `<p class="eyebrow mt-3" data-t="inviteCode"></p><div class="code" id="iCode">${esc(d.code)}</div><p class="hint" data-t="keyOnce"></p>
        ${btn({ id: 'iCopy', variant: 'ghost', icon: 'copy', key: 'copy', cls: 'mt-2', tour: 'couriers.inviteCopy' })}`;
      for (const el of $$('[data-t]', $('#iOut'))) el.textContent = t(el.dataset.t);
      $('#iCopy').onclick = async () => { try { await navigator.clipboard.writeText(d.code); toast(t('copied')); } catch {} };
      loadCouriers().then(rerender);
    } catch (e) { toast(String(e.message || e)); }
  };
}

async function openCourier(id){
  const c = S.couriers.find(x => x.id === id) || { id };
  sheet(`<p class="eyebrow" data-t="courier"></p><h2>${esc(c.name || c.phone || id)}</h2><div id="cBody">${loading(1)}</div>`, { name: 'courier' });
  let d = null;
  try { d = await api(`/owner/couriers/${encodeURIComponent(id)}`); } catch {}
  const fix = d?.lastFix;
  $('#cBody').innerHTML = `
    <div class="fact">${icon('phone')}<span class="v"><span class="k" data-t="phone"></span>${c.phone ? `<a href="tel:${esc(c.phone)}">${esc(c.phone)}</a>` : '—'}</span></div>
    <div class="fact">${icon('clock')}<span class="v"><span class="k" data-t="onShift"></span><span data-t="${(d?.onShift ?? c.onShift) ? 'onShift' : 'offShift'}"></span></span></div>
    <div class="stats">
      <div class="stat"><small data-t="deliveries"></small><b>${d?.today?.deliveries ?? 0}</b><small class="muted" data-t="today"></small></div>
      <div class="stat"><small data-t="deliveries30"></small><b>${d?.delivered30d ?? 0}</b></div>
      <div class="stat"><small data-t="inFlight"></small><b>${d?.inFlight ?? 0}</b></div>
      <div class="stat"><small data-t="cashHeld"></small><b>${money(d?.today?.cashCollected ?? 0)}</b></div>
    </div>
    <div class="fact">${icon('map-pin')}<span class="v"><span class="k" data-t="lastSeen"></span>${fix ? `${esc(ago(fix.recordedAtMs))} · <a href="https://www.google.com/maps/search/?api=1&query=${fix.latUdeg / 1e6},${fix.lonUdeg / 1e6}&z=${MAP_ZOOM}" target="_blank" rel="noopener">Google Maps</a>` : '—'}</span></div>
    ${switchEl('c-active', d?.active ?? c.active, 'activeC', null, 'couriers.active')}`;
  for (const el of $$('[data-t]', $('#cBody'))) el.textContent = t(el.dataset.t);
  $('#c-active').onchange = async e => {
    if (!e.target.checked) {
      // Deactivating ends every session the courier holds; that deserves a pause.
      const ok = await confirm(t('deactivate'), t('deactivateHint'), { danger: true });
      if (!ok) return openCourier(id);
    }
    try { await post(`/owner/couriers/${encodeURIComponent(id)}/active`, { active: e.target.checked }); toast(t('saved')); loadCouriers().then(rerender); openCourier(id); }
    catch (err) { toast(String(err.message || err)); e.target.checked = !e.target.checked; }
  };
}
