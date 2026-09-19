// Act 3 -- RECEIVE. The order, watched, like waiting for the next episode.
//
// The whole screen is the Sea: an ink-wash ocean across the entire surface,
// developing with the order. Over it, ONE state -- the one the order is in
// now -- as a title, the way an episode's name comes up, with the step count
// beneath it and the honest time range. Not six pills: a customer waiting
// wants to know where their food is, not a diagram of where it could be.
// While they wait, the venue's own guests speak: one positive review at a
// time, fading in and out like end credits. The wallet to pay into (crypto),
// the way to be told (Telegram) and the sentence the customer can leave are
// below, quiet.
//
// The state belongs to the server, so it is asked every twelve seconds with
// the order's OWN token; a poll that fails three times says so. The ocean's
// canvas and the review reel SURVIVE each re-render: lifted out before the
// sheet is redrawn and put back after, so the sea keeps its time and the
// credits keep their place. The title changes only when the status does.

import { state, tokenFor, moneyEl, on, API } from '/store/state.js';
import { t, lang } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, toast, whenSheetCloses, stars } from '/store/ui.js';
import { openOcean, phaseOf, seaRest } from '/store/sea.js';
import * as trackMap from '/store/track-map.js';

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
/// The progress line never reads empty while an order exists.
const PROGRESS_FLOOR = 0.06;
/// The credits: a review is positive from this rating, long enough to say
/// something from this many characters, shown for this long, and cut here.
const REVIEW_MIN_RATING = 4;
const REVIEW_MIN_CHARS = 24;
const REVIEW_MAX_CHARS = 220;
const REVIEW_MS = 7000;
const REVIEW_FADE_MS = 700;
/// The payment, as one word for the line.
const PAY_KEY = { cash: 'cash', card: 'card', apple_pay: 'applePay', google_pay: 'googlePay', crypto: 'crypto' };

let lastStatus = null;
let reel = null;

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

// ── the credits: the venue's guests, one at a time ──────────────────────────
/// The language a review is written in, from its letters: Cyrillic is
/// Ukrainian here, the Albanian diacritics are Albanian, the rest English.
const CYRILLIC = /[\u0400-\u04FF]/;
const ALBANIAN = /[ëçË]|\b(dhe|është|shumë|për|nuk)\b/i;
const langOf = text => CYRILLIC.test(text) ? 'uk' : ALBANIAN.test(text) ? 'sq' : 'en';
/// The review in the reader's language: a translation the venue holds, else
/// the original when it already is that language, else nothing.
function reviewText(r){
  const own = String(r.text || '').trim();
  const tr = r.translations?.[lang];
  if (tr && String(tr).trim()) return String(tr).trim();
  return langOf(own) === lang ? own : null;
}
function goodReviews(){
  const list = state.loc?.google?.reviews;
  if (!Array.isArray(list)) return [];
  const good = list.filter(r => (r.rating || 0) >= REVIEW_MIN_RATING && String(r.text || '').trim().length >= REVIEW_MIN_CHARS);
  const inLang = good.filter(r => reviewText(r));
  // A venue with no review in the reader's language still has guests: the
  // originals are shown rather than nothing.
  return inLang.length ? inLang : good;
}
function reviewMarkup(r){
  const text = reviewText(r) || String(r.text).trim();
  return `<figure class="credit">
    <blockquote>“${esc(text.length > REVIEW_MAX_CHARS ? text.slice(0, REVIEW_MAX_CHARS - 1) + '…' : text)}”</blockquote>
    <figcaption><span class="rev-stars" aria-hidden="true">${stars(r.rating)}</span><b>${esc(r.author || '')}</b></figcaption>
  </figure>`;
}
/// Start the reel in `host`, or keep the one already running.
function startReel(host){
  const list = goodReviews();
  if (!host || !list.length) return;
  if (reel && reel.host === host) return;
  stopReel();
  let i = Math.floor(Math.random() * list.length);
  const show = () => {
    host.innerHTML = reviewMarkup(list[i]);
    i = (i + 1) % list.length;
  };
  show();
  const timer = setInterval(() => {
    const fig = host.querySelector('.credit'); if (fig) fig.classList.add('out');
    setTimeout(show, REVIEW_FADE_MS);
  }, REVIEW_MS);
  reel = { host, timer };
}
function stopReel(){ if (reel) { clearInterval(reel.timer); reel = null; } }

/// The episode: eyebrow, the one state as the title, the step, the time.
function episodeMarkup(order, eta){
  const st = order.status, dead = DEAD.has(st), i = FLOW.indexOf(st);
  const phase = dead ? PROGRESS_FLOOR : Math.max(PROGRESS_FLOOR, phaseOf(st));
  const done = st === 'DELIVERED';
  return `<div class="ep">
    <p class="ep-eyebrow"><span data-t="episode"></span> · #${esc(String(order.id).slice(0, ORDER_ID_SHOWN))} · ${esc(state.loc?.name || '')}</p>
    <div class="ep-plate">
      <h2 class="ep-title ${st !== lastStatus ? 'flip' : ''}" data-t-st="${esc(st)}"></h2>
      ${!dead ? `<div class="ep-dots" aria-hidden="true">${FLOW.map((f, n) => `<i class="${n < i ? 'done' : n === i ? 'now' : ''}"></i>`).join('')}</div>` : ''}
    </div>
    ${!dead ? `<p class="ep-step mono"><span data-t="stepOf"></span> ${i + 1} <span data-t="ofSteps"></span> ${FLOW.length}${!done && i + 1 < FLOW.length ? ` · <span data-t="nextUp"></span>: <span data-t-st="${FLOW[i + 1]}"></span>` : ''}</p>` : ''}
    <div class="ep-line" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${Math.round(phase * 100)}"><i id="epBar"></i></div>
    <div class="ep-facts">
      ${eta && !dead && !done ? `<span class="${eta.live ? 'live' : ''}">${icon('clock')}<b>${esc(eta.text)} <span data-t="etaMin"></span></b>${eta.live ? `<i class="dot-live" aria-hidden="true"></i>` : ''}</span>` : ''}
      <span>${icon('coin-hole')}<b>${moneyEl(order.total ?? order.subtotal ?? 0)}</b></span>
      ${PAY_KEY[order.payment] ? `<span>${icon(order.payment === 'cash' ? 'cash' : order.payment === 'crypto' ? 'currency-bitcoin' : 'credit-card')}<b data-t="${PAY_KEY[order.payment]}"></b></span>` : ''}
    </div>
  </div>`;
}

export function openTracking(order){
  const st = order.status;
  const dead = DEAD.has(st);
  const bot = state.loc?.telegramBot;
  // THE TIME THAT IS LEFT, from the hub, as the order stands now -- the
  // kitchen's remaining minutes and the courier's real road -- re-read with
  // every poll. The checkout's quote is only the first frame's fallback.
  const live = order.eta && typeof order.eta === 'object' && order.eta.range ? { text: order.eta.range, live: true, parts: order.eta.parts, known: order.eta.known } : null;
  const eta = live || state.lastEta;
  const follow = (bot && !dead && st !== 'DELIVERED') ? `
    <a class="btn btn-ghost mb-1" href="https://t.me/${encodeURIComponent(bot)}?start=${encodeURIComponent(order.id)}" target="_blank" rel="noopener noreferrer">
      ${icon('brand-telegram')}<span data-t="notify"></span></a>` : '';
  // The sea's canvas and the credits outlive the sheet's markup.
  const wasTrack = $('#sheet').dataset.name === 'track';
  const keepCanvas = wasTrack ? $('.ocean-cv') : null;
  const keepReel = wasTrack ? $('#credits') : null;
  const keepMap = wasTrack ? trackMap.keep() : null;
  whenSheetCloses(() => { stopReel(); lastStatus = null; seaRest(); trackMap.destroy(); });
  sheet(`
    <div class="tsheet" data-status="${esc(st)}">
      <canvas class="ocean-cv" aria-hidden="true"></canvas>
      <div class="tsheet-in">
        ${episodeMarkup(order, eta)}
        ${trackMap.markup(order)}
        ${goodReviews().length ? `<section class="credits-wrap"><p class="eyebrow" data-t="whatTheySay"></p><div class="credits" id="credits"></div></section>` : ''}
        <div class="ep-more">
          ${cryptoBlock(order)}
          ${follow}
          ${sayBlock(order)}
          <button class="btn btn-ghost mb-2" id="closeTrack" data-t="done"></button>
        </div>
      </div>
    </div>`, { name: 'track', attending: !dead && st !== 'DELIVERED', full: true });
  if (keepCanvas) $('.ocean-cv')?.replaceWith(keepCanvas);
  if (keepReel) $('#credits')?.replaceWith(keepReel);
  trackMap.mount(order, keepMap);
  const phase = dead ? PROGRESS_FLOOR : Math.max(PROGRESS_FLOOR, phaseOf(st));
  const bar = $('#epBar'); if (bar) bar.style.width = `${Math.round(phase * 100)}%`;
  const ep = $('.ep'); if (ep) ep.style.setProperty('--pg', String(phase));
  lastStatus = st;
  openOcean($('.ocean-cv'), st);
  startReel($('#credits'));
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
        if ($('#sheet').dataset.name === 'track') openTracking({ ...d, eta: d.eta || (eta && !eta.live ? eta : undefined) });
      } catch {
        openTracking._fails = (openTracking._fails || 0) + 1;
        if (openTracking._fails === POLL_FAILS_TO_TELL) toast(t('loadFail'));
        if ($('#sheet').dataset.name === 'track') openTracking(order);
      }
    }, POLL_MS);
  }
}
