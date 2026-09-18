// Act 3 -- RECEIVE. The order, watched.
//
// The Sea is the centrepiece here: it holds the order's colour and grows with
// its status, terracotta to gold, and Delivered is the one loud beat. The
// Sheet carries what the Sea cannot -- the words, the honest time range, the
// total, the way to be told, the wallet to pay into when the order is paid in
// crypto, and the sentence the customer can leave. A customer with reduced
// motion or no WebGL loses nothing: the pills say it all.
//
// The state belongs to the server, so it is asked every twelve seconds with
// the order's OWN token; a poll that fails three times says so.

import { state, tokenFor, moneyEl, on, API } from '/store/state.js';
import { t } from '/store/i18n.js';
import { $, esc, icon, sheet, closeSheet, toast, whenSheetCloses } from '/store/ui.js';
import { seaForOrder, seaRest } from '/store/sea.js';

const FLOW = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'];
const DEAD = new Set(['REJECTED', 'CANCELLED']);
/// How often the order is re-asked while it is live, and how many misses in a
/// row before the customer is told the page has lost the hub.
const POLL_MS = 12_000;
const POLL_FAILS_TO_TELL = 3;
/// The order id is shown short: enough to tell two apart, short enough to say.
const ORDER_ID_SHOWN = 8;
/// A customer may write this much about the meal.
const FEEDBACK_MAX = 600;

function sayBlock(order){
  const over = order.status === 'DELIVERED' || DEAD.has(order.status);
  if (!over || !on('feedback')) return '';
  if (order.feedback) return `<p class="geo ok mb-2" data-t="saidIt"></p>`;
  return `<label for="f-say" data-t="sayHow"></label>
    <textarea id="f-say" maxlength="${FEEDBACK_MAX}" rows="2"></textarea>
    <p class="avoid-h" data-t="sayHint"></p>
    <button class="btn btn-ghost mb-2" id="sayGo" data-t="sayGo"></button>`;
}
function bindSay(order){
  const go = $('#sayGo'); if (!go) return;
  go.onclick = async () => {
    const text = $('#f-say').value.trim(); if (!text) return;
    go.disabled = true;
    try {
      const tok = tokenFor(order.id);
      const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/feedback`, {
        method: 'POST', headers: { 'content-type': 'application/json', ...(tok ? { authorization: 'Bearer ' + tok } : {}) },
        body: JSON.stringify({ text }) });
      const d = await r.json(); if (!r.ok) throw new Error(d.error || ('HTTP ' + r.status));
      openTracking({ ...order, feedback: { text } });
    } catch (e) { go.disabled = false; toast(String(e.message || e)); }
  };
}

/// The wallet to pay into, for an order paid in crypto and not yet paid. The
/// amount is the venue's own total in its own currency; the conversion is the
/// customer's wallet's business at the moment they pay, and the line says so.
function cryptoBlock(order){
  const w = order.crypto?.wallet;
  if (!w || order.crypto?.paid || DEAD.has(order.status)) return '';
  return `<div class="wallet">
    <p class="eyebrow"><span data-t="payWith"></span> ${esc(w.symbol)} · <span data-t="network"></span> ${esc(w.network)}</p>
    <p><span data-t="sendExactly"></span> ${moneyEl(order.total ?? 0)} <span data-t="toAddress"></span>:</p>
    <div class="wallet-addr"><code class="money" id="walletAddr">${esc(w.address)}</code>
      <button type="button" class="btn btn-ghost" id="walletCopy">${icon('copy')}<span data-t="copy"></span></button></div>
    ${w.note ? `<p class="muted small">${esc(w.note)}</p>` : ''}
    <p class="muted small" data-t="cryptoRate"></p>
    <p class="muted small" data-t="cryptoWait"></p>
  </div>`;
}
function bindCopy(){
  const b = $('#walletCopy'); if (!b) return;
  b.onclick = async () => {
    try { await navigator.clipboard.writeText($('#walletAddr').textContent); toast(t('copied')); }
    catch { toast($('#walletAddr').textContent); }
  };
}

export function openTracking(order){
  const st = order.status, i = FLOW.indexOf(st);
  const dead = DEAD.has(st);
  seaForOrder(st);
  whenSheetCloses(seaRest);
  const bot = state.loc?.telegramBot;
  const eta = order.eta || state.lastEta;
  const follow = (bot && !dead && st !== 'DELIVERED') ? `
    <a class="btn btn-ghost mb-1" href="https://t.me/${encodeURIComponent(bot)}?start=${encodeURIComponent(order.id)}" target="_blank" rel="noopener noreferrer">
      ${icon('brand-telegram')}<span data-t="notify"></span></a>
    <p class="muted trk-note" data-t="notifyHint"></p>` : '';
  sheet(`
    <div class="tsheet" data-status="${esc(st)}">
      <p class="eyebrow">#${esc(String(order.id).slice(0, ORDER_ID_SHOWN))}</p>
      <h2>${dead ? `<span data-t-st="${esc(st)}"></span>` : `<span data-t="sent"></span>`}</h2>
      ${!dead ? `<div class="pills" role="list">${FLOW.map((s, n) => `
        <span class="pill-st ${n < i ? 'done' : n === i ? 'now' : ''}" role="listitem">
          <span class="pill-dot">${n < i ? icon('check') : ''}</span><span data-t-st="${s}"></span></span>`).join('')}</div>` : ''}
      ${eta && !dead && st !== 'DELIVERED' ? `<p class="teta">${icon('clock')}<span data-t="etaRange"></span>: <b>${esc(eta.text)} <span data-t="etaMin"></span></b></p>` : ''}
      <div class="totals"><div class="row grand"><span data-t="total"></span>${moneyEl(order.total ?? order.subtotal ?? 0)}</div></div>
      ${cryptoBlock(order)}
      ${follow}
      ${sayBlock(order)}
      <button class="btn btn-ghost mb-2" id="closeTrack" data-t="done"></button>
    </div>`, { name: 'track', attending: !dead && st !== 'DELIVERED' });
  bindSay(order);
  bindCopy();
  $('#closeTrack').onclick = closeSheet;
  clearTimeout(openTracking._t);
  if (!dead && st !== 'DELIVERED') {
    openTracking._t = setTimeout(async () => {
      try {
        const tok = tokenFor(order.id);
        const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}`, tok ? { headers: { authorization: 'Bearer ' + tok } } : undefined);
        if (!r.ok) throw new Error('HTTP ' + r.status);
        const d = await r.json();
        openTracking._fails = 0;
        if ($('#sheet').dataset.name === 'track') openTracking({ ...d, eta: d.eta || eta });
      } catch {
        openTracking._fails = (openTracking._fails || 0) + 1;
        if (openTracking._fails === POLL_FAILS_TO_TELL) toast(t('loadFail'));
        if ($('#sheet').dataset.name === 'track') openTracking(order);
      }
    }, POLL_MS);
  }
}
