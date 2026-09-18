// Book a Table 1:4845 / 1:5287 / 1:5472 and My Booking 1:8023 / 1:8166 /
// 1:8303. Light set: 1:15250, 1:15695, 1:15881, 1:18449, 1:18593, 1:18731.
//
// The three Book a Table frames are the same screen with a different step
// answered, so they are one screen; the three My Booking frames are one list
// with a different status filter, exactly like My Orders.
//
// RESERVATIONS ARE A DOWIZ DOMAIN NOW. `dowiz_kernel::reservation` carries the
// same decide/fold Law the order machine does — a forbidden transition is an
// error, the state is replayed from events and never written, and the decision
// reads no clock of its own. Continue posts the request; the kernel decides
// whether it may become a booking, and this screen reports whichever answer
// came back.

import { icon, esc } from '/kit/app.js';
import { topBar, ctaBar, plate, navbar } from '/kit/parts.js';
import { reservations, requestId } from '/kit/data.js';

const DOW = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'];
const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June',
                'July', 'August', 'September', 'October', 'November', 'December'];

// 11:00 to 21:00 on the half hour, as the frame lists them.
const SLOTS = Array.from({ length: 21 }, (_, i) => {
  const m = 11 * 60 + i * 30;
  return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
});

const OCCASIONS = ['None', 'Birthday', 'Anniversary', 'Business', 'Date'];

// The frame is drawn on April 2026.
const state = { guests: 6, year: 2026, month: 3, day: 15, slot: '19:30', occasion: 'Birthday',
                tab: 'upcoming' };

const BOOKINGS = [
  { id: '#RSV8475', status: 'upcoming', venue: 'Brooklyn Bites', rating: 4.9,
    address: '789 Park Avenue, New Yo...', when: 'April 15, 2026 - 07:30 PM' },
  { id: '#RSV8390', status: 'completed', venue: 'The Savory Spot', rating: 4.8,
    address: '88 Bedford Street, New Y...', when: 'April 02, 2026 - 08:00 PM' },
  { id: '#RSV8201', status: 'cancelled', venue: 'Food Fusion Hub', rating: 5.0,
    address: '600 Lexington Ave, New ...', when: 'March 28, 2026 - 07:00 PM' },
];

const TABS = [
  { id: 'upcoming',  name: 'Upcoming' },
  { id: 'completed', name: 'Completed' },
  { id: 'cancelled', name: 'Cancelled' },
];

const ACTIONS = {
  upcoming:  [{ label: 'Cancel', to: 'cancel-booking' },
              { label: 'Navigate', to: 'get-direction', primary: true }],
  completed: [{ label: 'Leave Review', to: 'leave-review' },
              { label: 'Book Again', to: 'book-a-table', primary: true }],
  cancelled: [{ label: 'View Details', to: 'booking-summary' },
              { label: 'Book Again', to: 'book-a-table', primary: true }],
};

// ── Calendar ───────────────────────────────────────────────────────────────

function monthGrid(year, month){
  const first = new Date(Date.UTC(year, month, 1));
  const lead = first.getUTCDay();
  const days = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  const prevDays = new Date(Date.UTC(year, month, 0)).getUTCDate();
  const cells = [];
  for (let i = lead - 1; i >= 0; i--) cells.push({ n: prevDays - i, out: true });
  for (let d = 1; d <= days; d++) cells.push({ n: d, out: false });
  while (cells.length % 7) cells.push({ n: cells.length - lead - days + 1, out: true });
  return cells;
}

const calendar = () => `
  <div class="k-cal">
    <div class="k-cal-head">
      <button class="k-cal-nav is-prev" type="button" data-month="-1"
              aria-label="Попередній місяць">${icon('arrow-right')}</button>
      <span>${MONTHS[state.month]} ${state.year}</span>
      <button class="k-cal-nav" type="button" data-month="1"
              aria-label="Наступний місяць">${icon('arrow-right')}</button>
    </div>
    <div class="k-cal-grid">
      ${DOW.map(d => `<span class="k-cal-dow">${d}</span>`).join('')}
      ${monthGrid(state.year, state.month).map(c => c.out
        ? `<span class="k-day is-out">${c.n}</span>`
        : `<button class="k-day" type="button" data-day="${c.n}"
                   aria-pressed="${c.n === state.day}">${c.n}</button>`).join('')}
    </div>
  </div>`;

// ── Screens ────────────────────────────────────────────────────────────────

const bookPane = () => `
  <h2 class="k-block-h">Guests</h2>
  <div class="k-guests">
    <button class="k-obtn" type="button" data-guest="-1" aria-label="Менше гостей">−</button>
    <span class="k-guests-n" id="guests" aria-live="polite">${state.guests}</span>
    <button class="k-obtn is-primary" type="button" data-guest="1"
            aria-label="Більше гостей">+</button>
  </div>

  <h2 class="k-block-h">Select Date</h2>
  <div id="cal">${calendar()}</div>

  <h2 class="k-block-h">Select Time</h2>
  <div class="k-slots" role="group" aria-label="Час">
    ${SLOTS.map(t => `
      <button class="k-slot" type="button" data-slot="${esc(t)}"
              aria-pressed="${t === state.slot}">${esc(t)}</button>`).join('')}
  </div>

  <h2 class="k-block-h">Special Occasion</h2>
  <div class="k-choice-row" role="group" aria-label="Привід">
    ${OCCASIONS.map(o => `
      <button class="k-choice-pill" type="button" data-occ="${esc(o)}"
              aria-pressed="${o === state.occasion}">${esc(o)}</button>`).join('')}
  </div>
  <p class="k-page-p" id="said" role="status"></p>`;

const bookingCard = b => `
  <article class="k-bcard" data-booking="${esc(b.id)}">
    <div class="k-bcard-top">
      <span class="k-ocard-id"><span>Booking ID</span> ${esc(b.id)}</span>
      ${b.status === 'upcoming'
        ? `<span class="k-remind">${icon('notification-bing')}Remind me</span>`
        : `<span class="k-status ${b.status === 'completed' ? 'is-done' : 'is-cancelled'}">${
             esc(b.status === 'completed' ? 'Completed' : 'Cancelled')}</span>`}
    </div>
    <div class="k-ocard-rule"></div>
    <div class="k-bfacts">
      <span><span class="k-fact-k">Booking Date &amp; Time</span>
        <span class="k-fact-v">${esc(b.when)}</span></span>
    </div>
    <div class="k-ocard-rule"></div>
    <div class="k-rrow" data-go="restaurant-menu">
      <div class="k-rrow-img">${plate(b.venue)}</div>
      <div class="k-rrow-body">
        <div class="k-rrow-head">
          <span class="k-rrow-name">${esc(b.venue)}</span>
          <span class="k-rate">${icon('star2')}${b.rating}</span>
        </div>
        <span class="k-rrow-line">${icon('pin-17')}${esc(b.address)}</span>
      </div>
    </div>
    <div class="k-ocard-acts">
      ${ACTIONS[b.status].map(a => `
        <button class="k-obtn${a.primary ? ' is-primary' : ''}" type="button"
                data-go="${esc(a.to)}">${esc(a.label)}</button>`).join('')}
    </div>
  </article>`;

const listPane = () => {
  const rows = BOOKINGS.filter(b => b.status === state.tab);
  return rows.length ? rows.map(bookingCard).join('')
    : `<div class="k-empty">${icon('calendar')}
         <p class="t-title">Порожньо</p>
         <p class="t-body muted2">Тут з’являться броні зі статусом
           «${esc(TABS.find(t => t.id === state.tab).name)}».</p></div>`;
};

export function render(params, routeName = 'book-a-table'){
  if (routeName === 'my-booking'){
    state.tab = TABS.some(t => t.id === params?.get('tab')) ? params.get('tab') : 'upcoming';
    return `
    ${topBar('My Booking')}
    <div class="wrap">
      <div class="k-tabs k-tabs-wide" role="tablist">
        ${TABS.map(t => `
          <button type="button" role="tab" data-tab="${esc(t.id)}"
                  aria-selected="${t.id === state.tab}">${esc(t.name)}</button>`).join('')}
      </div>
      <div id="bookings" class="k-block" role="tabpanel">${listPane()}</div>
    </div>
    ${navbar('')}`;
  }

  return `
  ${topBar('Book a Table')}
  <div class="wrap k-page" data-book>${bookPane()}</div>
  ${ctaBar('Continue')}`;
}

export function bind(root){
  const repaintList = () => {
    const el = root.querySelector('#bookings');
    el.innerHTML = listPane();
    for (const p of el.querySelectorAll('.k-plate[data-hue]')){
      const hue = Number(p.dataset.hue);
      p.style.background =
        `linear-gradient(140deg,hsl(${hue} 46% 58%),hsl(${(hue + 38) % 360} 52% 42%))`;
      p.removeAttribute('data-hue');
    }
  };

  root.addEventListener('click', e => {
    const tab = e.target.closest('[data-tab]');
    if (tab){
      state.tab = tab.dataset.tab;
      for (const b of root.querySelectorAll('[role="tab"]'))
        b.setAttribute('aria-selected', String(b.dataset.tab === state.tab));
      history.replaceState(null, '', `#/my-booking?tab=${state.tab}`);
      return repaintList();
    }

    const g = e.target.closest('[data-guest]');
    if (g){
      state.guests = Math.max(1, Math.min(20, state.guests + Number(g.dataset.guest)));
      root.querySelector('#guests').textContent = state.guests;
      return;
    }

    const m = e.target.closest('[data-month]');
    if (m){
      const next = state.month + Number(m.dataset.month);
      state.month = (next + 12) % 12;
      state.year += next < 0 ? -1 : next > 11 ? 1 : 0;
      // A day that does not exist in the new month cannot stay chosen.
      const days = new Date(Date.UTC(state.year, state.month + 1, 0)).getUTCDate();
      state.day = Math.min(state.day, days);
      root.querySelector('#cal').innerHTML = calendar();
      return;
    }

    const d = e.target.closest('[data-day]');
    if (d){
      state.day = Number(d.dataset.day);
      for (const b of root.querySelectorAll('[data-day]'))
        b.setAttribute('aria-pressed', String(Number(b.dataset.day) === state.day));
      return;
    }

    const s = e.target.closest('[data-slot]');
    if (s){
      state.slot = s.dataset.slot;
      for (const b of root.querySelectorAll('[data-slot]'))
        b.setAttribute('aria-pressed', String(b.dataset.slot === state.slot));
      return;
    }

    const o = e.target.closest('[data-occ]');
    if (o){
      state.occasion = o.dataset.occ;
      for (const b of root.querySelectorAll('[data-occ]'))
        b.setAttribute('aria-pressed', String(b.dataset.occ === state.occasion));
      return;
    }

    if (e.target.closest('[data-cta]') && root.querySelector('[data-book]')){
      book(root, e.target.closest('[data-cta]'));
    }
  });
}

/// The slot as minutes since the Unix epoch — the kernel's unit. Built from the
/// chosen date and time in UTC so the number the venue stores and the number the
/// phone sent are the same integer.
function slotMinutes(){
  const [h, m] = state.slot.split(':').map(Number);
  return Math.floor(Date.UTC(state.year, state.month, state.day, h, m) / 60000);
}

async function book(root, button){
  const said = root.querySelector('#said');
  button.disabled = true;
  said.textContent = 'Надсилаємо…';

  const answer = await reservations.create({
    party: state.guests,
    slotMin: slotMinutes(),
    occasion: state.occasion === 'None' ? '' : state.occasion,
    requestId: requestId(),
  });

  button.disabled = false;

  if (!answer){
    // No venue in the host: the screen is being viewed as a design, and saying
    // so is better than a spinner that never stops.
    said.textContent =
      `${state.guests} гостей · ${state.day} ${MONTHS[state.month]} ${state.year} · ` +
      `${state.slot} · ${state.occasion}. Хост не називає заклад, тож бронь нікуди слати.`;
    return;
  }
  if (answer.error){
    // The kernel's own words. A refusal that says "невідома помилка" teaches
    // nobody anything; "slot is before now" tells you to pick another time.
    said.textContent = `Не вийшло: ${answer.error}`;
    return;
  }

  said.textContent = answer.replayed
    ? `Ця бронь уже створена (${answer.id}).`
    : `Бронь створена: ${answer.id}. Чекає підтвердження закладу.`;
  location.hash = '#/booking-summary';
}
