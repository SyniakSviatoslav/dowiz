// The queue -- what the console exists for.
//
// Live orders as rows the kitchen scans: who, what, how much, how long ago,
// the time that is left (the hub's live estimate, courier and kitchen
// included), and ONE next step as a gold button. A tap opens the order as a
// sheet: the stepper, the customer, the door part by part, the dish list,
// the courier to hand it to, the way it is paid. History is the same rows,
// quiet. A new order rings and glows once.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, money, moneyEl, ago, clock, day,
         busy, confirm, ORDER_ID_SHOWN } from '/admin/core.js';
import { st, payName } from '/admin/i18n.js';
import { liveOrders, loadOrders, rerender } from '/admin/app.js';

const FLOW = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'];
const DEAD = new Set(['REJECTED', 'CANCELLED']);
/// The one next step for each state: the action the hub takes, its word.
const NEXT = { PENDING: ['confirm', 'accept'], CONFIRMED: ['preparing', 'startCooking'], PREPARING: ['ready', 'markReady'] };
/// History shows this many, newest first.
const HISTORY_MAX = 60;
/// The items line on a row shows this many dishes before "…".
const ITEMS_SHOWN = 3;

const view = { mode: 'live', q: '' };

const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');
function matching(){
  const all = view.mode === 'history' ? S.orders.filter(o => !liveOrders().includes(o)).slice(0, HISTORY_MAX) : liveOrders();
  const q = norm(view.q).trim(); if (!q) return all;
  return all.filter(o => norm([o.id.slice(0, ORDER_ID_SHOWN), o.contact?.name, o.contact?.phone, ...(o.items || []).map(i => i.name)].join(' ')).includes(q));
}
const etaText = o => o.eta && o.eta.range ? `${o.eta.range} ${t('etaMin') === 'etaMin' ? 'min' : ''}`.trim() : '';
const isPickup = o => o.fulfilment?.kind === 'pickup';

function row(o){
  const step = NEXT[o.status];
  const items = (o.items || []);
  const line = items.slice(0, ITEMS_SHOWN).map(i => `${i.quantity}× ${esc(i.name || i.product_id)}`).join(' · ') + (items.length > ITEMS_SHOWN ? ' …' : '');
  const fresh = S.fresh.has(o.id) ? 'fresh' : '';
  return `<article class="orow ${fresh}" data-st="${esc(o.status)}" data-st-var="${esc(o.status)}" data-o="${esc(o.id)}">
    <span class="st" data-t-st="${esc(o.status)}"></span>
    <span class="who">${esc(o.contact?.name || o.contact?.phone || '#' + o.id.slice(0, ORDER_ID_SHOWN))}</span>
    <span class="amt">${moneyEl(o.total ?? 0)}</span>
    <span class="num">${o.contact?.name || o.contact?.phone ? '#' + esc(o.id.slice(0, ORDER_ID_SHOWN)) : ''}</span>
    <span class="meta">
      <span>${icon(isPickup(o) ? 'walk' : 'bike')}<span data-t="${isPickup(o) ? 'pickup' : 'delivery'}"></span></span>
      <span>${icon(o.payment === 'cash' ? 'cash' : o.payment === 'crypto' ? 'currency-bitcoin' : 'credit-card')}${esc(payName(o.payment))}</span>
      ${o.eta?.range ? `<span class="live">${icon('clock')}<b>${esc(o.eta.range)}</b> min</span>` : ''}
      ${o.courier_id ? `<span>${icon('bike')}${esc(courierName(o.courier_id))}</span>` : ''}
    </span>
    <span class="age">${esc(ago(o.created_at_ms || Date.now()))}</span>
    <span class="items">${line}</span>
    ${step && !DEAD.has(o.status) ? `<span class="go"><button type="button" class="act pri" data-act="${step[0]}" data-o="${esc(o.id)}">${icon('check')}<span data-t="${step[1]}"></span></button>
      ${o.status === 'PENDING' ? `<button type="button" class="act danger" data-act="reject" data-o="${esc(o.id)}">${icon('x')}</button>` : ''}
      ${o.status !== 'PENDING' && !isPickup(o) && !o.courier_id ? `<button type="button" class="act" data-assign="${esc(o.id)}">${icon('bike')}<span data-t="assign"></span></button>` : ''}</span>` : ''}
  </article>`;
}
const courierName = id => (S.couriers.find(c => c.id === id) || {}).name || id.slice(0, 6);

export async function render(host){
  const s = S.stats || {};
  const list = matching();
  host.innerHTML = `
    <div class="stats">
      <div class="stat"><small data-t="todayOrders"></small><b>${s.todayOrders ?? '—'}</b></div>
      <div class="stat ${s.pending ? 'warn' : ''}"><small data-t="pending"></small><b>${s.pending ?? '—'}</b></div>
      <div class="stat"><small data-t="active"></small><b>${s.active ?? '—'}</b></div>
      <div class="stat"><small data-t="revenue"></small><b>${s.todayRevenue != null ? money(s.todayRevenue) : '—'}</b></div>
    </div>
    <div class="seg" role="tablist">
      <button type="button" class="seg-b ${view.mode === 'live' ? 'on' : ''}" data-mode="live"><span data-t="live"></span><span class="n">${liveOrders().length}</span></button>
      <button type="button" class="seg-b ${view.mode === 'history' ? 'on' : ''}" data-mode="history"><span data-t="history"></span></button>
    </div>
    <label class="srch">${icon('search')}<input id="oq" type="search" value="${esc(view.q)}" data-t-attr="placeholder:findOrder"></label>
    ${S.phase === 'error' ? `<div class="empty">${icon('alert-triangle')}<b>${esc(t('loadFail'))}</b><span class="muted small">${esc(S.error || '')}</span></div>` : ''}
    ${S.phase === 'ready' && !list.length ? `<div class="empty">${icon('scroll')}<b data-t="${view.mode === 'live' ? 'noLive' : 'noOrders'}"></b></div>` : ''}
    <div class="orders" id="olist">${list.map(row).join('')}</div>`;
  S.fresh.clear();
  host.onclick = async e => {
    const mode = e.target.closest('[data-mode]'); if (mode) { view.mode = mode.dataset.mode; return rerender(); }
    const act = e.target.closest('[data-act]'); if (act) { e.stopPropagation(); return doAction(act.dataset.o, act.dataset.act, act); }
    const asg = e.target.closest('[data-assign]'); if (asg) { e.stopPropagation(); return openAssign(asg.dataset.assign); }
    const r = e.target.closest('.orow'); if (r) return openOrder(r.dataset.o);
  };
  const q = $('#oq', host);
  q.oninput = () => { view.q = q.value; const l = $('#olist', host); if (l) l.innerHTML = matching().map(row).join(''); };
}

async function doAction(id, action, el){
  let reason = '';
  if (action === 'reject' || action === 'cancel') {
    const c = await confirm(t(action === 'reject' ? 'reject' : 'cancelOrder'), '#' + id.slice(0, ORDER_ID_SHOWN), { danger: true, reasonLabel: t('reason') });
    if (!c) return; reason = c.reason;
  }
  try {
    await busy(el, () => post(`/owner/orders/${encodeURIComponent(id)}/action`, withLoc({ action, ...(reason ? { reason } : {}) })));
    navigator.vibrate?.(12);
    await loadOrders(); await rerender();
    if ($('#sheet').dataset.name === 'order') openOrder(id);
  } catch (e) { toast(String(e.message || e)); }
}

async function openAssign(id){
  const list = S.couriers.filter(c => c.active);
  sheet(`<p class="eyebrow">#${esc(id.slice(0, ORDER_ID_SHOWN))}</p><h2 data-t="assign"></h2>
    ${list.length ? list.map(c => `<button type="button" class="choice" data-c="${esc(c.id)}">${icon('bike')}<span class="t"><b>${esc(c.name)}</b><br><small class="muted" data-t="${c.onShift ? 'onShift' : 'offShift'}"></small></span>${icon('chevron-right', 'ck')}</button>`).join('')
              : `<div class="empty">${icon('bike')}<b data-t="noCouriers"></b></div>`}`, { name: 'assign' });
  for (const b of $$('[data-c]', $('#sheetIn'))) b.onclick = async () => {
    try { await busy(b, () => post(`/owner/orders/${encodeURIComponent(id)}/assign`, withLoc({ courier_id: b.dataset.c }))); toast(t('saved')); await loadOrders(); await rerender(); openOrder(id); }
    catch (e) { toast(String(e.message || e)); }
  };
}

function orderText(o){
  const lines = (o.items || []).map(i => `${i.quantity}× ${i.name || i.product_id}`).join('\n');
  const addr = o.fulfilment?.address;
  const parts = addr?.parts ? Object.entries(addr.parts).filter(([k, v]) => k !== 'private' && v).map(([k, v]) => `${t(k)}: ${v}`).join(', ') : '';
  return `#${o.id.slice(0, ORDER_ID_SHOWN)} · ${st(o.status)}\n${o.contact?.name || ''} ${o.contact?.phone || ''}\n${isPickup(o) ? t('pickup') : (addr?.line || '')}${parts ? '\n' + parts : ''}\n${lines}\n${t('total')}: ${money(o.total ?? 0)} · ${payName(o.payment)}${o.fulfilment?.note || addr?.note ? '\n' + t('note') + ': ' + (o.fulfilment?.note || addr?.note) : ''}`;
}

export function openOrder(id){
  const o = S.orders.find(x => x.id === id); if (!o) return;
  const i = FLOW.indexOf(o.status), dead = DEAD.has(o.status);
  const addr = o.fulfilment?.address, parts = addr?.parts || {};
  const step = NEXT[o.status];
  sheet(`
    <p class="eyebrow">#${esc(o.id.slice(0, ORDER_ID_SHOWN))} · ${esc(clock(o.created_at_ms || Date.now()))} · ${esc(day(o.created_at_ms || Date.now()))}</p>
    <h2 data-t-st="${esc(o.status)}"></h2>
    ${!dead ? `<div class="steps">${FLOW.map((f, n) => `<i class="${n < i ? 'done' : n === i ? 'now' : ''}"></i>`).join('')}</div>` : ''}
    ${o.eta?.range && !dead ? `<div class="fact">${icon('clock')}<span class="v"><span class="k" data-t="etaRange"></span><b>${esc(o.eta.range)} min</b>
      <small class="muted"> · ${[o.eta.parts?.prepLeftMin ? `${t('cookingMin')} ${o.eta.parts.prepLeftMin}` : '', o.eta.parts?.courierToVenueMin ? `${t('courier')} → ${o.eta.parts.courierToVenueMin}` : '', o.eta.parts?.toDoorMin ? `→ ${t('address')} ${o.eta.parts.toDoorMin}` : ''].filter(Boolean).join(' · ')}</small></span></div>` : ''}
    <div class="fact">${icon('user')}<span class="v"><span class="k" data-t="customer"></span>${esc(o.contact?.name || '—')}${o.contact?.phone ? ` · <a href="tel:${esc(o.contact.phone)}">${esc(o.contact.phone)}</a>` : ''}</span></div>
    <div class="fact">${icon(isPickup(o) ? 'walk' : 'map-pin')}<span class="v"><span class="k" data-t="${isPickup(o) ? 'pickup' : 'address'}"></span>
      ${isPickup(o) ? `<span data-t="pickup"></span>` : `${esc(addr?.line || '—')}
      ${Object.keys(parts).length ? `<br><small class="muted">${['street', 'house', 'apartment', 'entrance', 'floor'].filter(k => parts[k]).map(k => `${esc(t(k))}: ${esc(parts[k])}`).join(' · ')}${parts.private ? ` · ${esc(t('privateHouse'))}` : ''}</small>` : ''}
      ${addr?.lat_udeg ? `<br><a href="https://www.google.com/maps/search/?api=1&query=${addr.lat_udeg / 1e6},${addr.lon_udeg / 1e6}" target="_blank" rel="noopener">${esc(t('onMap') === 'onMap' ? 'Google Maps' : t('onMap'))}</a>` : ''}`}</span></div>
    ${(o.fulfilment?.note || addr?.note) ? `<div class="fact">${icon('note')}<span class="v"><span class="k" data-t="note"></span>${esc(o.fulfilment?.note || addr?.note)}</span></div>` : ''}
    <div class="fact">${icon(o.payment === 'cash' ? 'cash' : 'credit-card')}<span class="v"><span class="k" data-t="payment"></span>${esc(payName(o.payment))}${o.tip ? ` · ${t('tip')} ${money(o.tip)}` : ''}${o.crypto?.wallet ? ` · ${esc(o.crypto.wallet.symbol)}` : ''}</span></div>
    ${o.courier_id ? `<div class="fact">${icon('bike')}<span class="v"><span class="k" data-t="courier"></span>${esc(courierName(o.courier_id))}</span></div>` : ''}
    <p class="eyebrow mt-3" data-t="items"></p>
    ${(o.items || []).map(it => `<div class="line"><span class="q">${it.quantity}×</span><span class="n">${esc(it.name || it.product_id)}${it.modifier_ids?.length ? `<small>${esc(it.modifier_ids.join(', '))}</small>` : ''}</span>${moneyEl((it.unit_price ?? 0) * (it.quantity || 1))}</div>`).join('')}
    ${o.delivery_fee ? `<div class="line"><span class="n" data-t="delivery"></span>${moneyEl(o.delivery_fee)}</div>` : ''}
    ${o.discount ? `<div class="line"><span class="n" data-t="discount"></span><span>−${moneyEl(o.discount)}</span></div>` : ''}
    <div class="line total"><span class="n" data-t="total"></span>${moneyEl(o.total ?? 0)}</div>
    ${o.rejection_reason ? `<p class="err">${esc(o.rejection_reason)}</p>` : ''}
    ${o.feedback?.text ? `<div class="fact">${icon('message-2')}<span class="v">${esc(o.feedback.text)}</span></div>` : ''}
    <div class="btn-row">
      ${step && !dead ? `<button class="btn" data-act="${step[0]}" data-o="${esc(o.id)}">${icon('check')}<span data-t="${step[1]}"></span></button>` : ''}
      ${!dead && o.status !== 'PENDING' && !isPickup(o) && !o.courier_id ? `<button class="btn ghost" data-assign="${esc(o.id)}">${icon('bike')}<span data-t="assign"></span></button>` : ''}
    </div>
    <div class="btn-row">
      ${o.contact?.phone ? `<a class="btn ghost" href="tel:${esc(o.contact.phone)}">${icon('phone')}<span data-t="call"></span></a>` : ''}
      <button class="btn ghost" id="oCopy">${icon('copy')}<span data-t="print"></span></button>
      ${o.status === 'PENDING' ? `<button class="btn danger" data-act="reject" data-o="${esc(o.id)}">${icon('x')}<span data-t="reject"></span></button>`
        : !dead && o.status !== 'DELIVERED' ? `<button class="btn danger" data-act="cancel" data-o="${esc(o.id)}">${icon('x')}<span data-t="cancelOrder"></span></button>` : ''}
    </div>`, { name: 'order' });
  $('#sheetIn').onclick = e => {
    const act = e.target.closest('[data-act]'); if (act) return doAction(act.dataset.o, act.dataset.act, act);
    const asg = e.target.closest('[data-assign]'); if (asg) return openAssign(asg.dataset.assign);
  };
  $('#oCopy').onclick = async () => { try { await navigator.clipboard.writeText(orderText(o)); toast(t('copied')); } catch { toast(orderText(o)); } };
}
