// Booking a TABLE: the venue's floor, drawn from data, for one slot.
//
// WHAT THIS IS BUILT FROM. `docs/design/booking-template.html` is the
// operator's prototype and this page keeps its LAYOUT, its INTERACTION and its
// DATA MODEL: a tab per zone, an SVG plan, chairs placed by seat count, one
// selected table, a legend, and Enter/Space on a `role="button"` group.
//
// THREE THINGS OF THE PROTOTYPE'S DELIBERATELY DO NOT TRAVEL:
//
//   1. ITS GEOMETRY. The prototype hard-codes one restaurant's tables in a
//      `ZONES` const. The real floor plan is the venue's and arrives from
//      `GET /api/public/locations/:slug/tables`; an owner changes it without a
//      deploy. Nothing below knows where a table stands.
//   2. ITS `occ` FLAG. Occupancy is not a property of a table -- table 5 is
//      free at 18:00 and taken at 20:00 -- so a time is chosen FIRST and the
//      plan is asked for that slot. The CTA cannot be reached without both.
//   3. ITS PALETTE. `#F1EFE8` bone and `#085041` green are the prototype's
//      choice. A booking screen in a different palette from the menu the
//      customer just left reads as a different restaurant, so every colour
//      here is a `--brand-*` token the venue's own theme paints (state.js
//      `applyTheme`). The contrast pairs this page relies on are the ones the
//      hub's `palette.rs` ENFORCES at AA on the way in -- text on surface,
//      muted on surface, on-primary on primary -- which is why they are the
//      only pairs `store.css` puts words on. Measured for the shipped default
//      and for Dubin & Sushi's served set; see `.bk-*` in store.css.
//
// The floor's three states are FREE / SELECTED / TAKEN. They are not kernel
// order statuses and nothing here restates that vocabulary, which is generated
// into `/lib/vocab.js` and has already been hand-copied four times.

import { API, SLUG, state, hhmm } from '/store/state.js';
import { t, lang, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, sheetName, toast } from '/store/ui.js';
// THE SLOT ARITHMETIC IS NOT IN THIS FILE. It reads no clock, so it can be
// examined at a chosen instant -- `lib/booking-time.test.mjs` runs it in node
// with no browser, which is the only way any of it was ever going to be tested.
import { midnightMs, slotOf as slotAt, weekdayOf as weekdayAt, minuteNow, timesOn as timesIn, offsetMinutes }
  from '/lib/booking-time.js';
// A GUEST BOOKS WITHOUT AN ACCOUNT, the way a guest orders: a name and a
// phone, and the booking comes back with a token for that one booking. The
// rules that are not visual are in `lib/booking-guest.js` (node-tested); the
// guest's own list, the link and the cancel are `booking-mine.js`.
import { safeGet, safeSet } from '/store/storage.js';
import { guestErrors, bookingBody, refusalText } from '/lib/booking-guest.js';
import { keepBooking, myBookings, openMine, takeLinkFromHash } from '/store/booking-mine.js';
import '/store/booking-words.js';

/// How far ahead the day strip runs. The kernel's horizon is sixty days; two
/// weeks is what a phone strip can hold without becoming a date picker.
const DAYS_AHEAD = 14;
/// Party sizes offered. The kernel's `max_party` is 20; a larger party is a
/// phone call, which the venue sheet already carries.
const PARTIES = [1, 2, 3, 4, 5, 6, 8];
/// Only when the venue has published no week at all. Stated, not hidden: a
/// venue with no hours still gets a bookable evening rather than a blank page.
const FALLBACK_WINDOW = [{ open: 11 * 60, close: 23 * 60 }];
/// Weekday 0 is Monday, the way `dowiz_hub::hours` counts.
const DAY_NAMES = {
  sq: ['Hën', 'Mar', 'Mër', 'Enj', 'Pre', 'Sht', 'Die'],
  en: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
  uk: ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Нд'],
};

// TWO SEPARATE WAITS. `busy` is the plan being fetched, which replaces the
// drawing with a skeleton; `sending` is the booking being posted, which must
// NOT -- a guest watching their chosen table disappear while the request is in
// flight has no idea whether they booked it.
const ui = { party: 2, day: 0, minute: null, zone: 0, pick: null, plan: null,
             busy: false, sending: false, err: null, refocus: false,
             name: safeGet('dw_name') || '', phone: safeGet('dw_phone') || '' };

// ── time, in the venue's own day ────────────────────────────────────────────

/// FROM THE VENUE'S ZONE NAME. This read `state.loc.tzOffsetMinutes`, a field
/// the hub never sends, so every slot was computed at +120 -- right until the
/// last Sunday of October, an hour wrong after it.
const tzOffset = () => offsetMinutes(state.loc?.tz || 'Europe/Tirane', now());

/// `Date.now()` IS READ HERE AND NOWHERE BELOW IT. Every rule takes the
/// instant, so the screen is the only thing in this flow that touches a clock.
const now = () => Date.now();
const dayStartMs = n => midnightMs(now(), tzOffset(), n);
const slotOf = (day, minute) => slotAt(now(), tzOffset(), day, minute);
const weekdayOf = day => weekdayAt(now(), tzOffset(), day);

function windowsOn(day) {
  const week = Array.isArray(state.loc?.hours) && state.loc.hours.length === 7 ? state.loc.hours : null;
  const wins = week ? week[weekdayOf(day)] || [] : FALLBACK_WINDOW;
  return wins.length ? wins : [];
}

/// Every bookable minute on a day, with today's past times dropped.
const timesOn = day =>
  timesIn(windowsOn(day), day === 0 ? minuteNow(now(), tzOffset()) : -1);

// ── the plan, for the chosen slot ───────────────────────────────────────────

async function loadPlan() {
  if (ui.minute === null) { ui.plan = null; return; }
  ui.busy = true; ui.err = null; draw();
  const url = `${API}/public/locations/${encodeURIComponent(SLUG)}/tables`
    + `?slotMin=${slotOf(ui.day, ui.minute)}&party=${ui.party}`;
  try {
    const r = await fetch(url);
    if (!r.ok) throw new Error(await r.text());
    ui.plan = await r.json();
    // A table chosen for one slot is not chosen for another: the same table can
    // be free at 18:00 and taken at 20:00, so the pick is dropped whenever the
    // slot moves and is re-made against the answer that came back.
    ui.pick = null;
    if (ui.zone >= (ui.plan.zones || []).length) ui.zone = 0;
  } catch (e) {
    ui.plan = null;
    ui.err = String(e && e.message ? e.message : e).slice(0, 200) || t('bkPlanFail');
  } finally {
    ui.busy = false; draw();
  }
}

const zoneAt = () => (ui.plan?.zones || [])[ui.zone] || null;
const tableOf = p => (ui.plan?.zones || []).find(z => z.id === p?.zone)?.tables.find(x => x.n === p?.n) || null;

// ── the drawing ─────────────────────────────────────────────────────────────
// Every colour is a class; store.css maps each class to a `--brand-*` token,
// so the plan is repainted by the venue's theme like everything else.

const chair = (x, y, a) => `<g class="bk-chair" transform="translate(${x.toFixed(1)} ${y.toFixed(1)}) rotate(${a.toFixed(1)})">
  <rect x="-9" y="-6" width="18" height="12" rx="3"/><path d="M-9 6 Q0 10 9 6" fill="none"/></g>`;

/// Chairs placed by seat count, as the prototype places them.
function chairs(tb) {
  const { x: cx, y: cy, w, h, seats } = tb;
  let s = '';
  if (tb.shape === 'circle') {
    const r = w / 2;
    for (let i = 0; i < seats; i++) {
      const a = Math.PI * 2 * i / seats - Math.PI / 2;
      s += chair(cx + (r + 13) * Math.cos(a), cy + (r + 13) * Math.sin(a), a * 180 / Math.PI + 270);
    }
    return s;
  }
  const hw = w / 2, hh = h / 2;
  let top = [], bot = [], lef = [], rig = [];
  if (seats <= 2) { top = [cx]; bot = [cx]; }
  else if (seats === 3) { top = [cx]; bot = [cx]; lef = [cy]; }
  else if (seats === 4) { if (w > h * 1.25) { top = [cx - w / 4, cx + w / 4]; bot = top; } else { top = [cx]; bot = [cx]; lef = [cy]; rig = [cy]; } }
  else { top = [cx - w / 4, cx + w / 4]; bot = top; lef = [cy]; rig = [cy]; }
  top.forEach(x => s += chair(x, cy - hh - 12, 180));
  bot.forEach(x => s += chair(x, cy + hh + 12, 0));
  lef.forEach(y => s += chair(cx - hw - 12, y, 90));
  rig.forEach(y => s += chair(cx + hw + 12, y, -90));
  return s;
}

function tableSvg(tb, selected) {
  const out = tb.occupied || tb.tooSmall;
  const cls = tb.occupied ? 'occ' : tb.tooSmall ? 'small' : selected ? 'sel' : 'free';
  const { x: cx, y: cy, w, h } = tb;
  const body = tb.shape === 'circle'
    ? `<circle class="bk-top" cx="${cx}" cy="${cy}" r="${w / 2}"/>`
    : `<rect class="bk-top" x="${cx - w / 2}" y="${cy - h / 2}" width="${w}" height="${h}" rx="6"/>`;
  const word = tb.occupied ? t('bkTaken') : tb.tooSmall ? t('bkSmall') : selected ? t('bkPicked') : t('bkFree');
  // The aria label is the whole sentence a screen reader needs: which table,
  // how many seats, and what state it is in for the slot being looked at.
  const label = `${t('bkTable')} ${tb.n}, ${tb.seats} ${t('bkSeats')}, ${word}`;
  return `<g class="bk-t ${cls}" data-n="${tb.n}" role="button" tabindex="${out ? -1 : 0}"
    aria-pressed="${selected}" aria-label="${esc(label)}"${out ? ' aria-disabled="true"' : ''}>
    ${chairs(tb)}${body}
    <text class="bk-num" x="${cx}" y="${cy - 1}" text-anchor="middle">${tb.n}</text>
    <text class="bk-sub" x="${cx}" y="${cy + 12}" text-anchor="middle">${tb.seats}</text></g>`;
}

/// THE ROOM IS NOT DRAWN FROM A FIXTURE. The prototype paints brick, parquet
/// and a bar because it knows one restaurant; this page knows none, so the
/// room is the plan's own bounds and a door, and everything else on it is a
/// table the owner placed.
function planSvg() {
  const z = zoneAt();
  if (!z) return '';
  const W = ui.plan.planW, H = ui.plan.planH;
  const tables = z.tables.map(tb => tableSvg(tb, ui.pick && ui.pick.zone === z.id && ui.pick.n === tb.n)).join('');
  return `<svg class="bk-plan" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" role="img"
    aria-label="${esc(z.name)}">
    <rect class="bk-floor" x="8" y="8" width="${W - 16}" height="${H - 16}" rx="6"/>
    <line class="bk-door" x1="${W / 2 - 28}" y1="${H - 8}" x2="${W / 2 + 28}" y2="${H - 8}"/>
    ${tables}</svg>`;
}

// ── the screen ──────────────────────────────────────────────────────────────

function head() {
  // A day with nothing bookable left on it is not offered at all -- an empty
  // time strip under a chosen date reads as a broken page.
  const days = Array.from({ length: DAYS_AHEAD }, (_, n) => n).filter(n => timesOn(n).length);
  const times = timesOn(ui.day);
  const mine = myBookings().length;
  return `
  <p class="eyebrow" data-t="bkRoom"></p>
  <h2 data-t="bkTitle"></h2>
  ${mine ? `<button type="button" class="btn ghost" id="bkMine">${icon('clock')}<span data-t="bkMine"></span> (${mine})</button>` : ''}
  <p class="bk-hint" data-t="bkHint"></p>

  <p class="bk-lbl" data-t="bkGuests"></p>
  <div class="bk-row" role="radiogroup" aria-label="${esc(t('bkGuests'))}">
    ${PARTIES.map(p => `<button type="button" class="bk-chip${p === ui.party ? ' on' : ''}"
      data-party="${p}" aria-pressed="${p === ui.party}">${p}</button>`).join('')}
  </div>

  <p class="bk-lbl" data-t="bkDate"></p>
  <div class="bk-row bk-scroll" role="radiogroup" aria-label="${esc(t('bkDate'))}">
    ${days.map(n => {
      const d = new Date(dayStartMs(n) + tzOffset() * 60_000);
      const name = (DAY_NAMES[lang] || DAY_NAMES.en)[weekdayOf(n)];
      return `<button type="button" class="bk-chip bk-day${n === ui.day ? ' on' : ''}"
        data-day="${n}" aria-pressed="${n === ui.day}"><b>${esc(name)}</b><i>${d.getUTCDate()}</i></button>`;
    }).join('') || `<span class="muted" data-t="closedNow"></span>`}
  </div>

  <p class="bk-lbl" data-t="bkTime"></p>
  <div class="bk-row bk-scroll" role="radiogroup" aria-label="${esc(t('bkTime'))}">
    ${times.map(m => `<button type="button" class="bk-chip${m === ui.minute ? ' on' : ''}"
      data-min="${m}" aria-pressed="${m === ui.minute}">${hhmm(m)}</button>`).join('')
      || `<span class="muted" data-t="closedNow"></span>`}
  </div>`;
}

function body() {
  if (ui.minute === null) return `<div class="empty bk-empty">${icon('clock', 'ico-lg')}<b data-t="bkPickTime"></b></div>`;
  if (ui.busy) return `<div class="skel skel-plan"></div>`;
  if (ui.err) return `<div class="empty bk-empty">${icon('alert-triangle', 'ico-lg')}<b data-t="bkPlanFail"></b>
    <span class="muted small">${esc(ui.err)}</span>
    <button type="button" class="btn" id="bkRetry" data-t="retry"></button></div>`;
  const zones = ui.plan?.zones || [];
  // NO PLAN IS NOT NO BOOKING. A venue that has not drawn its room still
  // takes a party and a time; it seats them where there is room.
  if (!hasTables())
    return `<div class="empty bk-empty">${icon('building', 'ico-lg')}<b data-t="bkNoPlanBook"></b>
      ${state.loc?.phone ? `<a class="btn ghost" href="tel:${esc(state.loc.phone)}" data-t="callUs"></a>` : ''}</div>${contact()}`;
  return `
  <div class="bk-tabs" role="tablist" aria-label="${esc(t('bkRoom'))}">
    ${zones.map((z, i) => `<button type="button" role="tab" class="bk-tab" data-zone="${i}"
      aria-selected="${i === ui.zone}">${esc(z.name)}</button>`).join('')}
  </div>
  ${planSvg()}
  <div class="bk-legend">
    <span><i class="free"></i><span data-t="bkFree"></span></span>
    <span><i class="sel"></i><span data-t="bkPicked"></span></span>
    <span><i class="occ"></i><span data-t="bkTaken"></span></span>
    <!-- The small number under each table's own number. The prototype writes
         "4 mis." on the table itself; at 11px inside a 40px table that word
         does not fit in three languages, so it is said once here instead, and
         in full in every table's aria-label. -->
    <span><b class="bk-key">4</b><span data-t="bkSeats"></span></span>
  </div>${contact()}`;
}

const hasTables = () => (ui.plan?.zones || []).some(z => z.tables.length);

/// Who the booking is under. Kept in `ui` as it is typed, because every chip
/// press redraws the whole sheet, and remembered with the checkout's keys so
/// a guest who ordered once does not type it twice.
const contact = () => `
  <label for="bk-name" data-t="bkName"></label>
  <input id="bk-name" autocomplete="name" maxlength="80" value="${esc(ui.name)}">
  <label for="bk-phone" data-t="bkPhone"></label>
  <input id="bk-phone" type="tel" inputmode="tel" autocomplete="tel" placeholder="+355..." value="${esc(ui.phone)}">`;

function foot() {
  const tb = tableOf(ui.pick);
  // A TABLE IS OPTIONAL: without one the venue seats the party where it likes.
  const ready = ui.minute !== null && !!ui.plan && !ui.busy;
  const when = ui.minute === null ? '' :
    `${(DAY_NAMES[lang] || DAY_NAMES.en)[weekdayOf(ui.day)]} ${hhmm(ui.minute)}`;
  const zoneName = (ui.plan?.zones || []).find(z => z.id === ui.pick?.zone)?.name || '';
  const line = tb && ui.minute !== null
    ? `<span data-t="bkChosen"></span>: <b>${esc(zoneName)} · ${esc(t('bkTable'))} ${tb.n}</b>
       · ${tb.seats} <span data-t="bkSeats"></span> · ${esc(when)}`
    : ui.minute === null ? `<span data-t="bkPickTime"></span>`
    : `<span data-t="bkAnyTable"></span> · ${esc(when)}`;
  const cta = ui.sending ? 'bkSending' : tb ? 'bkCta' : 'bkCtaAny';
  return `<div class="bk-foot"><p class="bk-pick">${line}</p>
    <button type="button" class="btn bk-cta" id="bkGo"${ready && !ui.sending ? '' : ' disabled'}
      data-t="${cta}"></button></div>`;
}

function draw() {
  if (sheetName() !== 'book') return;
  sheet(`<div class="bk">${head()}${body()}${foot()}</div>`, { name: 'book', full: true, keepScroll: true });
  bind();
}

function bind() {
  const root = $('#sheetIn');
  const set = (fn) => { fn(); draw(); };
  // The trailing `draw()` is load-bearing: with no time chosen yet `loadPlan`
  // returns without drawing, and the chip the finger just pressed would stay
  // looking unpressed.
  for (const b of $$('[data-party]', root)) b.onclick = () => { ui.party = +b.dataset.party; loadPlan(); draw(); };
  for (const b of $$('[data-day]', root)) b.onclick = () => set(() => { ui.day = +b.dataset.day; ui.minute = null; ui.plan = null; ui.pick = null; });
  for (const b of $$('[data-min]', root)) b.onclick = () => { ui.minute = +b.dataset.min; loadPlan(); };
  for (const b of $$('[data-zone]', root)) b.onclick = () => set(() => { ui.zone = +b.dataset.zone; });
  const retry = $('#bkRetry', root); if (retry) retry.onclick = loadPlan;
  // THE PROTOTYPE'S KEYBOARD, kept: a table is a `role="button"` group, so it
  // must answer Enter and Space, and an occupied one must not be reachable.
  for (const g of $$('.bk-t:not(.occ):not(.small)', root)) {
    const pick = () => {
      const n = +g.dataset.n, z = zoneAt()?.id;
      ui.pick = (ui.pick && ui.pick.zone === z && ui.pick.n === n) ? null : { zone: z, n };
      ui.refocus = true;
      draw();
    };
    g.addEventListener('click', pick);
    g.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); pick(); } });
  }
  const go = $('#bkGo', root); if (go) go.onclick = commit;
  const nm = $('#bk-name', root); if (nm) nm.oninput = () => { ui.name = nm.value; };
  const ph = $('#bk-phone', root); if (ph) ph.oninput = () => { ui.phone = ph.value; };
  const mine = $('#bkMine', root); if (mine) mine.onclick = () => openMine();
  // A REDRAW MUST NOT DROP THE KEYBOARD. The whole sheet is rewritten on every
  // change, the way the prototype re-renders, so the table the finger or the
  // Enter key just chose is given the focus back -- otherwise tabbing through
  // the plan sends the caret to the top of the sheet on every selection.
  if (ui.pick && ui.refocus) {
    ui.refocus = false;
    $(`.bk-t[data-n="${ui.pick.n}"]`, root)?.focus();
  }
}

// ── sending it ──────────────────────────────────────────────────────────────

async function commit() {
  const tb = tableOf(ui.pick);
  if (ui.minute === null || ui.sending) return;
  const errs = guestErrors(ui);
  if (errs.length) { toast(errs.map(k => t(k)).join('. ')); $(errs[0] === 'bkNeedPhone' ? '#bk-phone' : '#bk-name')?.focus(); return; }
  ui.sending = true; draw();
  const slotMin = slotOf(ui.day, ui.minute);
  const body = bookingBody({
    party: ui.party, slotMin, pick: tb ? ui.pick : null, name: ui.name, phone: ui.phone,
    // The caller's own key: a retried request must not book a second table.
    rid: `bk_${slotMin}_${tb ? `${ui.pick.zone}_${ui.pick.n}` : 'any'}_${Math.random().toString(36).slice(2, 10)}`,
  });
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/reservations`, {
      method: 'POST', headers: { 'content-type': 'application/json', 'idempotency-key': body.requestId },
      body: JSON.stringify(body),
    });
    const text = await r.text();
    if (r.ok) {
      const d = JSON.parse(text);
      safeSet('dw_name', ui.name.trim()); safeSet('dw_phone', ui.phone.trim());
      if (d.access_token) keepBooking({ id: d.id, t: d.access_token, slotMin, party: ui.party });
      toast(t('bkSent'));
      ui.pick = null; ui.sending = false;
      await openMine(d.id);
      return;
    }
    // THE REFUSAL IS THE VENUE'S SENTENCE, shown as it arrived. The hub names
    // the table it refused; replacing that with "booking failed" is how a
    // guest ends up retrying the one table they cannot have.
    const why = refusalText(text);
    toast(why ? `${t('bkFail')}: ${why.slice(0, 160)}` : t('bkFail'));
    // The floor moved under us if somebody else took it: ask again.
    if (r.status === 409) await loadPlan();
  } catch {
    toast(t('bkFail'));
  } finally {
    if (ui.sending) { ui.sending = false; draw(); }
  }
}

/// The screen, from the nav bar. Reachable in one tap from the menu.
export function openBooking() {
  ui.pick = null;
  sheet(`<div class="bk"></div>`, { name: 'book', full: true });
  draw();
  if (ui.minute !== null) loadPlan();
}

// A LANGUAGE SWITCH REWRITES `data-t` NODES IN PLACE (i18n.js `retranslate`),
// which cannot reach the words built into an SVG `aria-label` or into the
// "chosen" line. Watching the attribute app.js sets is one listener and keeps
// this page honest in all three languages without it knowing about app.js.
new MutationObserver(() => { if (sheetName() === 'book') { draw(); retranslate($('#sheetIn')); } })
  .observe(document.documentElement, { attributes: true, attributeFilter: ['lang'] });

// A BOOKING LINK opened on this phone (`#rsv=<id>&t=<token>`) is kept and
// shown. After the first paint, so the menu behind the sheet is there.
setTimeout(takeLinkFromHash, 0);
