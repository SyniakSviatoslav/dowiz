// The couriers -- who rides for the venue, who is on shift, where they are.
//
// A courier is invited by phone with a code that lives seven days; the
// courier types it into their own app and sets their own password. The list
// shows each one with a shift chip; a tap opens the courier: today's runs,
// the last place they were seen (from the same fixes the live estimate
// uses), and the switch that lets them take orders at all.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, busy, ago, switchEl } from '/admin/core.js';
import { loadCouriers, rerender } from '/admin/app.js';

/// A courier's last fix is shown as a map link at this zoom.
const MAP_ZOOM = 16;

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabCouriers"></p><h1 data-t="tabCouriers"></h1></div>
    <button type="button" class="act pri" id="invite">${icon('user-plus')}<span data-t="invite"></span></button></div>
    <div id="clist"><div class="skel skel-row"></div></div>`;
  $('#invite', host).onclick = openInvite;
  let d;
  try { d = await api('/owner/couriers'); S.couriers = d.couriers || []; } catch (e) { $('#clist', host).innerHTML = `<div class="empty">${icon('alert-triangle')}<b>${esc(t('loadFail'))}</b></div>`; return; }
  const cs = d.couriers || [], inv = (d.invites || []).filter(i => !i.expired);
  $('#clist', host).innerHTML = `
    ${cs.length ? `<div class="rows">${cs.map(c => `<button type="button" class="rowc ${c.active ? '' : 'off'}" data-c="${esc(c.id)}">${icon('bike')}
      <span class="t"><b>${esc(c.name || c.phone || c.id)}</b><small class="mono">${esc(c.phone || '')}</small></span>
      <span class="pill ${c.onShift ? 'ok' : ''}" data-t="${c.onShift ? 'onShift' : 'offShift'}"></span>${icon('chevron-right', 'chev')}</button>`).join('')}</div>`
      : `<div class="empty">${icon('bike')}<b data-t="noCouriers"></b><span class="muted small" data-t="inviteHint"></span></div>`}
    ${inv.length ? `<div class="group mt-3"><p class="eyebrow" data-t="invite"></p><div class="rows">${inv.map(i => `<div class="rowc">${icon('ticket')}<span class="t"><b>${esc(i.name)}</b><small>${esc(t('inviteCode'))} · ${esc(new Date(i.untilMs).toLocaleDateString())}</small></span>
      <button type="button" class="act danger" data-uninvite="${esc(i.id)}">${icon('x')}</button></div>`).join('')}</div></div>` : ''}`;
  host.onclick = async e => {
    const u = e.target.closest('[data-uninvite]'); if (u) { try { await busy(u, () => post(`/owner/couriers/${encodeURIComponent(u.dataset.uninvite)}/uninvite`, withLoc())); rerender(); } catch (err) { toast(String(err.message || err)); } return; }
    const r = e.target.closest('[data-c]'); if (r) openCourier(r.dataset.c);
  };
}

function openInvite(){
  sheet(`<p class="eyebrow" data-t="tabCouriers"></p><h2 data-t="invite"></h2><p class="muted small" data-t="inviteHint"></p>
    <label for="i-name" data-t="name"></label><input id="i-name" autocomplete="off">
    <label for="i-phone" data-t="phone"></label><input id="i-phone" type="tel" inputmode="tel" placeholder="+355…">
    <div class="btn-row"><button class="btn" id="iGo">${icon('user-plus')}<span data-t="invite"></span></button></div>
    <div id="iOut"></div>`, { name: 'invite' });
  $('#iGo').onclick = async () => {
    try {
      const d = await busy($('#iGo'), () => post('/owner/couriers/invite', withLoc({ phone: $('#i-phone').value.trim(), name: $('#i-name').value.trim() })));
      $('#iOut').innerHTML = `<p class="eyebrow mt-3" data-t="inviteCode"></p><div class="code" id="iCode">${esc(d.code)}</div><p class="hint" data-t="keyOnce"></p>
        <button class="btn ghost mt-2" id="iCopy">${icon('copy')}<span data-t="copy"></span></button>`;
      for (const el of $$('[data-t]', $('#iOut'))) el.textContent = t(el.dataset.t);
      $('#iCopy').onclick = async () => { try { await navigator.clipboard.writeText(d.code); toast(t('copied')); } catch {} };
      loadCouriers().then(rerender);
    } catch (e) { toast(String(e.message || e)); }
  };
}

async function openCourier(id){
  const c = S.couriers.find(x => x.id === id) || { id };
  sheet(`<p class="eyebrow" data-t="courier"></p><h2>${esc(c.name || c.phone || id)}</h2><div id="cBody"><div class="skel skel-row"></div></div>`, { name: 'courier' });
  let d = null;
  try { d = await api(`/owner/couriers/${encodeURIComponent(id)}`); } catch {}
  const fix = d?.lastFix;
  $('#cBody').innerHTML = `
    <div class="fact">${icon('phone')}<span class="v"><span class="k" data-t="phone"></span>${c.phone ? `<a href="tel:${esc(c.phone)}">${esc(c.phone)}</a>` : '—'}</span></div>
    <div class="fact">${icon('clock')}<span class="v"><span class="k" data-t="onShift"></span><span data-t="${(d?.onShift ?? c.onShift) ? 'onShift' : 'offShift'}"></span></span></div>
    <div class="fact">${icon('scroll')}<span class="v"><span class="k" data-t="deliveries"></span><b class="mono">${d?.today?.deliveries ?? 0}</b> · ${t('today')}</span></div>
    <div class="fact">${icon('map-pin')}<span class="v"><span class="k" data-t="lastSeen"></span>${fix ? `${esc(ago(fix.recordedAtMs))} · <a href="https://www.google.com/maps/search/?api=1&query=${fix.latUdeg / 1e6},${fix.lonUdeg / 1e6}&z=${MAP_ZOOM}" target="_blank" rel="noopener">Google Maps</a>` : '—'}</span></div>
    ${switchEl('c-active', d?.active ?? c.active, 'activeC')}`;
  for (const el of $$('[data-t]', $('#cBody'))) el.textContent = t(el.dataset.t);
  $('#c-active').onchange = async e => {
    try { await post(`/owner/couriers/${encodeURIComponent(id)}/active`, withLoc({ active: e.target.checked })); toast(t('saved')); loadCouriers().then(rerender); }
    catch (err) { toast(String(err.message || err)); e.target.checked = !e.target.checked; }
  };
}
