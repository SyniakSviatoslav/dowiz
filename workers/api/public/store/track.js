// Act 3 -- RECEIVE. The order, watched over the Sea.
//
// This is where the Sea lives: a window of ink-wash ocean at the top of the
// sheet, developing with the order -- a young scattered sea when it is
// received, an aligned swell with the courier on the road, a full gold sea
// with the sun high when it is delivered. Over it floats one glass card, the
// way the direction's example draws it: status in the money face, the total,
// the order's line, a progress bar that is the same phase the sea has. The
// Sheet carries what the Sea cannot -- the words, the honest time range, the
// wallet to pay into for a crypto order, the way to be told, and the sentence
// the customer can leave. A customer with reduced motion or no WebGL loses
// nothing: the pills say it all.
//
// The state belongs to the server, so it is asked every twelve seconds with
// the order's OWN token; a poll that fails three times says so. The ocean's
// canvas SURVIVES each re-render: it is lifted out before the sheet is
// redrawn and put back after, so the sea keeps its time and its phase.

import { state, tokenFor, moneyEl, on, API } from '/store/state.js';
import { t } from '/store/i18n.js';
import { $, esc, icon, sheet, closeSheet, toast, whenSheetCloses } from '/store/ui.js';
import { openOcean, phaseOf, seaRest } from '/store/sea.js';

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
/// The progress bar never reads empty while an order exists.
const PROGRESS_FLOOR = 0.06;

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

/// The wallet to pay into, for an order paid in crypto and not yet paid.
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

/// The payment, as one word for the card's line.
const PAY_KEY = { cash: 'cash', card: 'card', apple_pay: 'applePay', google_pay: 'googlePay', crypto: 'crypto' };

/// The window of sea and the card that floats on it.
function oceanMarkup(order, eta){
  const st = order.status, dead = DEAD.has(st);
  return `<div class="ocean" data-status="${esc(st)}">
    <canvas class="ocean-cv" aria-hidden="true"></canvas>
    <div class="ocean-card" id="oceanCard">
      <span class="edge"></span>
      <div class="oc-row"><span class="oc-st" data-t-st="${esc(st)}"></span><span class="oc-amt">${moneyEl(order.total ?? order.subtotal ?? 0)}</span></div>
      <p class="oc-ln">#${esc(String(order.id).slice(0, ORDER_ID_SHOWN))} · ${esc(state.loc?.name || '')}${PAY_KEY[order.payment] ? ` · <span data-t="${PAY_KEY[order.payment]}"></span>` : ''}</p>
      <div class="pg" role="progressbar" aria-valuemin="0" aria-valuemax="100"><i id="oceanBar"></i></div>
      ${eta && !dead && st !== 'DELIVERED' ? `<p class="oc-note">${icon('clock')}<span data-t="etaRange"></span>: <b>${esc(eta.text)} <span data-t="etaMin"></span></b></p>` : ''}
    </div>
  </div>`;
}

export function openTracking(order){
  const st = order.status, i = FLOW.indexOf(st);
  const dead = DEAD.has(st);
  const bot = state.loc?.telegramBot;
  const eta = order.eta || state.lastEta;
  const follow = (bot && !dead && st !== 'DELIVERED') ? `
    <a class="btn btn-ghost mb-1" href="https://t.me/${encodeURIComponent(bot)}?start=${encodeURIComponent(order.id)}" target="_blank" rel="noopener noreferrer">
      ${icon('brand-telegram')}<span data-t="notify"></span></a>
    <p class="muted trk-note" data-t="notifyHint"></p>` : '';
  // The sea's canvas outlives the sheet's markup: lifted here, put back below.
  const keep = $('#sheet').dataset.name === 'track' ? $('.ocean-cv') : null;
  whenSheetCloses(seaRest);
  sheet(`
    <div class="tsheet" data-status="${esc(st)}">
      ${oceanMarkup(order, eta)}
      <h2 class="tsheet-h">${dead ? `<span data-t-st="${esc(st)}"></span>` : `<span data-t="sent"></span>`}</h2>
      ${!dead ? `<div class="pills" role="list">${FLOW.map((s, n) => `
        <span class="pill-st ${n < i ? 'done' : n === i ? 'now' : ''}" role="listitem">
          <span class="pill-dot">${n < i ? icon('check') : ''}</span><span data-t-st="${s}"></span></span>`).join('')}</div>` : ''}
      ${cryptoBlock(order)}
      ${follow}
      ${sayBlock(order)}
      <button class="btn btn-ghost mb-2" id="closeTrack" data-t="done"></button>
    </div>`, { name: 'track', attending: !dead && st !== 'DELIVERED' });
  if (keep) $('.ocean-cv')?.replaceWith(keep);
  const phase = dead ? PROGRESS_FLOOR : Math.max(PROGRESS_FLOOR, phaseOf(st));
  const bar = $('#oceanBar'); if (bar) { bar.style.width = `${Math.round(phase * 100)}%`; bar.parentElement.setAttribute('aria-valuenow', String(Math.round(phase * 100))); }
  const card = $('#oceanCard'); if (card) card.style.setProperty('--pg', String(phase));
  openOcean($('.ocean-cv'), st);
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
