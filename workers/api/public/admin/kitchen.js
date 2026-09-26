// The KITCHEN BOARD (lane W-KITCHEN, 2026-09-26): the pass, inside the hub.
//
// OPERATOR: "everything visible at once, big clear controls, no hidden menus,
// fewest taps, plain words". One screen: the station chips, the all-day
// counts, three columns (New / Preparing / Ready) of tickets, and the stop
// list. Every ticket has ONE big button that moves it on; a tap on its head
// tells the hub the kitchen saw it (Q4: seen only after a tap); a refusal or a
// cancellation asks for the reason the customer will read (Q1). A dish is
// taken off sale here directly (Q2).
//
// A VIEW OVER SERVER STATE. The tickets are `S.orders`, which the shell keeps
// from the kitchen's own stripped read (`/api/staff/kitchen`, no customer, no
// money) and the kitchen socket; the rules are `kitchen-logic.js`; the hub's
// FSM decides whether an edge is legal.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, icon, t, S, store, post, toast } from '/admin/core.js';
import { ui, btn, chips, empty } from '/admin/parts.js';
import * as K from '/admin/kitchen-logic.js';
import '/admin/kitchen-i18n.js';
import { rerender, loadOrders, loadVenue } from '/admin/app.js';

/// The ages tick on the screen, never on the server.
const TICK_MS = 30_000;
const view = { station: 'all', q: '' };
let ticker = null;

/// The stylesheet is this screen's own, linked once on first draw.
function linkStyle(){
  if (document.querySelector('link[data-kds]')) return;
  const l = document.createElement('link');
  l.setAttribute('rel', 'stylesheet'); l.setAttribute('href', '/admin/kitchen.css'); l.setAttribute('data-kds', '1');
  document.head?.appendChild(l);
}

function lineOf(l){
  const st = K.stationOf(l);
  return `<li class="kds-line"><b class="kds-qty">${esc(String(l.quantity | 0))}&times;</b> <span>${esc(l.name || l.product_id || '')}</span>${st !== 'kitchen' ? ` <em class="kds-st">${esc(t('st_' + st))}</em>` : ''}${l.note ? `<small class="kds-note">${esc(l.note)}</small>` : ''}</li>`;
}

function ticket(o, now, th){
  const where = K.whereOf(o);
  const age = K.ageMin(o.created_at_ms, now);
  const seen = !!(o.kitchen && o.kitchen.seen);
  const head = `#${K.shortId(o.id)} · ${t(where.key)}${where.table ? ' ' + where.table : ''} · ${age} ${t('kMin')} · ${t(seen ? 'kSeen' : 'kUnseen')}`;
  const bump = K.bumpFor(o), stop = K.stopFor(o);
  const note = o.fulfilment && o.fulfilment.note ? `<p class="kds-onote">${icon('note')} ${esc(o.fulfilment.note)}</p>` : '';
  return `<article class="kds-ticket kds-age-${K.ageClass(o.created_at_ms, now, th.warn, th.late)}${seen ? ' kds-seen' : ''}" data-o="${esc(o.id)}">
    ${btn({ variant: 'ghost', block: true, cls: 'kds-head', label: head, data: { seen: o.id }, tour: 'kitchen.seen', pressed: seen })}
    <ul class="kds-lines">${o.items.map(lineOf).join('')}</ul>${note}
    ${bump ? btn({ variant: bump === 'ready' ? 'success' : 'primary', size: 'lg', block: true, cls: 'kds-bump', key: 'bump_' + bump, data: { bump: o.id, act: bump }, tour: 'kitchen.bump' }) : ''}
    ${stop ? btn({ variant: 'ghost', block: true, cls: 'kds-stopbtn', icon: 'x', key: 'stop_' + stop, data: { stop: o.id, act: stop }, tour: 'kitchen.reject' }) : ''}
  </article>`;
}

function columns(now, th){
  const b = K.board(S.orders, view.station);
  return K.COLUMNS.map(([c]) => `<section class="kds-col kds-col-${c}" aria-label="${esc(t('col_' + c))}">
    <h2 class="kds-colh"><span data-t="col_${c}"></span> <span class="kds-n">${b[c].length}</span></h2>
    ${b[c].map(o => ticket(o, now, th)).join('')}</section>`).join('');
}

function allDayStrip(){
  const d = K.allDay(S.orders, view.station);
  if (!d.length) return '';
  return `<section class="kds-allday" data-tour="kitchen.allday"><p class="eyebrow" data-t="kAllDay"></p>
    <p class="muted small" data-t="kAllDayHint"></p>
    <div class="chips">${d.map(x => ui.chip({ label: `${x.name} ×${x.qty}`, tone: 'accent' })).join('')}</div></section>`;
}

const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

function stopRows(){
  const q = norm(view.q).trim();
  // Off sale first (what the pass must remember to put back), then by name.
  const list = (S.products || []).filter(p => !q || norm(p.name).includes(q))
    .sort((a, b) => Number(!!a.available) - Number(!!b.available) || String(a.name).localeCompare(String(b.name)));
  return list.map(p => `<div class="ui-row kds-dish${p.available ? '' : ' off'}" role="listitem">
    <span class="ui-row-body"><span class="ui-row-title ui-row-title--strong">${esc(p.name)}</span>${p.available ? '' : `<span class="ui-row-sub">${esc(t('kOffSale'))}${p.unavailableNote ? ' · ' + esc(p.unavailableNote) : ''}</span>`}</span>
    <span class="ui-row-trail">${p.available
      ? btn({ variant: 'danger', icon: 'x', key: 'k86', data: { off: p.id }, tour: 'kitchen.dishOff' })
      : btn({ variant: 'success', icon: 'check', key: 'kBackOn', data: { on: p.id }, tour: 'kitchen.dishOn' })}</span></div>`).join('');
}

export async function render(host){
  linkStyle();
  const now = Date.now();
  const th = K.thresholds(S.venue);
  const counts = K.stationCounts(S.orders);
  const stations = [{ value: 'all', label: `${t('kAll')} ${counts.all}` }, ...K.STATIONS.map(s => ({ value: s, label: `${t('st_' + s)} ${counts[s]}` }))];
  const open = counts.all > 0;
  host.innerHTML = `<div class="screen-h"><div><h1 data-t="tabKitchen"></h1></div></div>
    <p class="screen-hint" data-t="kBoardHint"></p>
    ${chips({ values: stations, value: view.station, attr: 'st', labelKey: 'tabKitchen', tour: 'kitchen.station' }).replace('class="chips"', 'class="chips kds-stations"')}
    ${allDayStrip()}
    ${open ? `<div class="kds-board" data-tour="kitchen.board">${columns(now, th)}</div>` : empty('flame', { key: 'kNoTickets', bodyKey: 'kNoTicketsHint' })}
    <section class="kds-stop" data-tour="kitchen.stoplist"><p class="eyebrow" data-t="kStopList"></p><p class="muted small" data-t="kStopHint"></p>
      ${ui.inputRow({ id: 'kdsQ', type: 'search', label: { t: 'kSearchDish' }, placeholder: { t: 'kSearchDish' }, attrs: { value: view.q, data: { tour: 'kitchen.dishSearch' } } })}
      <div class="rows" id="kdsDishes" role="list">${stopRows()}</div></section>`;
  const q = $('#kdsQ', host);
  if (q) q.oninput = e => { view.q = e.target.value; const r = $('#kdsDishes', host); if (r) r.innerHTML = stopRows(); };
  host.onclick = e => act(e.target, host);
  if (!ticker) {
    ticker = setInterval(() => { if (S.tab === 'kitchen' && !document.hidden) rerender(); }, TICK_MS);
    ticker.unref?.();
  }
}

/// A tapped control, by the data it carries.
export async function act(target, host){
  const el = target && target.closest ? target.closest('[data-st],[data-seen],[data-bump],[data-stop],[data-off],[data-on]') : null;
  if (!el) return;
  const d = el.dataset;
  if (d.st) { view.station = d.st; return rerender(); }
  try {
    if (d.seen) {
      if (el.getAttribute('aria-pressed') === 'true') return;
      await post(`/staff/orders/${encodeURIComponent(d.seen)}/kitchen-ack`, { location_id: store.loc });
      el.setAttribute('aria-pressed', 'true');
      return refresh();
    }
    if (d.bump) {
      const r = K.actionRequest(d.bump, d.act, store.loc);
      await post(r.path, r.body);
      navigator.vibrate?.(12);
      toast(t('saved'));
      return refresh();
    }
    if (d.stop) return askReason(d.stop, d.act);
    if (d.off || d.on) {
      const id = d.off || d.on;
      await post(`/owner/products/${encodeURIComponent(id)}`, { location_id: store.loc, available: !!d.on });
      toast(t('saved'));
      await loadVenue();
      return rerender();
    }
  } catch (err) { toast(String(err.message || err)); }
}

async function refresh(){ try { await loadOrders(); } catch {} await rerender(); }

/// Reject or cancel: the reason is required, and it is the one the customer reads.
function askReason(orderId, action){
  const actions = ui.button({ variant: 'ghost', label: { t: 'close' }, attrs: { 'data-ui-close': 'no', data: { tour: 'kitchen.reasonClose' } } })
    + ui.button({ variant: 'danger', icon: 'x', label: { t: 'stop_' + action }, attrs: { 'data-ui-close': 'yes', data: { tour: 'kitchen.reasonSend' } } });
  let why = '';
  const s = ui.openSheet({ title: { t: 'stop_' + action }, closeLabel: t('close'), actions,
    body: `<p class="ui-sheet-text" data-t="kReasonHint"></p>${ui.field({ id: 'kdsWhy', label: { t: 'kReason' }, attrs: { data: { tour: 'kitchen.reason' } } })}`,
    onClose: async v => {
      if (v !== 'yes') return;
      if (!why.trim()) return toast(t('kReasonNeeded'));
      try { const r = K.actionRequest(orderId, action, store.loc, why); await post(r.path, r.body); toast(t('saved')); await refresh(); }
      catch (e) { toast(String(e.message || e)); }
    } });
  const input = s.el.querySelector('#kdsWhy');
  if (input) input.oninput = () => { why = input.value; };
}

/// For the shell's tests and the assistant's "show me": the station in view.
export const station = () => view.station;
export function setStation(s){ if (s === 'all' || K.STATIONS.includes(s)) view.station = s; }
