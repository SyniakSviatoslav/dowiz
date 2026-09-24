// BOOKINGS: the day's reservations, and the venue's word on each.
//
// A VIEW OVER SERVER STATE. The list is `GET /api/owner/reservations` for the
// venue's own day (its local midnight, from its zone name -- the arithmetic
// the storefront books with). Each row carries `next`: the moves the kernel
// allows from where the booking is, narrowed to the venue's side. The buttons
// ARE that list, so no status chain lives in this file.
//
// NOTHING IS ERASED. "Cancel" is CANCELLED_BY_VENUE, one more event in the
// booking's history; the booking stays on the day, dimmed.
//
// A phone call is booked here too ("New booking"): the same route a guest
// uses, with the owner's token, so it lands CONFIRMED and signed VENUE.
//
// ASCII QUOTES ONLY in this file.

import { $, $$, esc, icon, t, S, api, post, toast, sheet, confirm, busy } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { midnightMs, offsetMinutes, DAY_MIN } from '/lib/booking-time.js';

const WORDS = {
  sq: { rsHint: 'Rezervimet e ditës. Konfirmoni, uleni ose anuloni; asgjë nuk fshihet.', rsNone: 'Asnjë rezervim këtë ditë.',
    rsNew: 'Rezervim i ri', rsName: 'Emri', rsPhone: 'Telefoni', rsParty: 'Persona', rsTime: 'Ora', rsTable: 'Tavolina', rsAny: 'Çdo tavolinë',
    rsBook: 'Rezervo', rsReason: 'Arsyeja (klienti e sheh)', rsCount: 'rezervime', rsGuests: 'persona',
    rsDo_CONFIRMED: 'Konfirmo', rsDo_DECLINED: 'Refuzo', rsDo_SEATED: 'U ulën', rsDo_COMPLETED: 'Mbaroi', rsDo_NO_SHOW: 'Nuk erdhi', rsDo_CANCELLED_BY_VENUE: 'Anulo',
    rsSt_REQUESTED: 'Kërkesë', rsSt_CONFIRMED: 'Konfirmuar', rsSt_SEATED: 'Në tavolinë', rsSt_COMPLETED: 'Mbaroi', rsSt_DECLINED: 'Refuzuar',
    rsSt_CANCELLED_BY_GUEST: 'Anuloi klienti', rsSt_CANCELLED_BY_VENUE: 'Anuluar', rsSt_NO_SHOW: 'Nuk erdhi' },
  en: { rsHint: 'The day\'s bookings. Confirm, seat or cancel; nothing is erased.', rsNone: 'No bookings on this day.',
    rsNew: 'New booking', rsName: 'Name', rsPhone: 'Phone', rsParty: 'Guests', rsTime: 'Time', rsTable: 'Table', rsAny: 'Any table',
    rsBook: 'Book', rsReason: 'Reason (the guest sees it)', rsCount: 'bookings', rsGuests: 'guests',
    rsDo_CONFIRMED: 'Confirm', rsDo_DECLINED: 'Decline', rsDo_SEATED: 'Seated', rsDo_COMPLETED: 'Finished', rsDo_NO_SHOW: 'No-show', rsDo_CANCELLED_BY_VENUE: 'Cancel',
    rsSt_REQUESTED: 'Request', rsSt_CONFIRMED: 'Confirmed', rsSt_SEATED: 'Seated', rsSt_COMPLETED: 'Finished', rsSt_DECLINED: 'Declined',
    rsSt_CANCELLED_BY_GUEST: 'Guest cancelled', rsSt_CANCELLED_BY_VENUE: 'Cancelled', rsSt_NO_SHOW: 'No-show' },
  uk: { rsHint: 'Бронювання дня. Підтвердіть, посадіть або скасуйте; нічого не стирається.', rsNone: 'Цього дня бронювань немає.',
    rsNew: 'Нове бронювання', rsName: "Ім'я", rsPhone: 'Телефон', rsParty: 'Гостей', rsTime: 'Час', rsTable: 'Стіл', rsAny: 'Будь-який стіл',
    rsBook: 'Забронювати', rsReason: 'Причина (гість її бачить)', rsCount: 'бронювань', rsGuests: 'гостей',
    rsDo_CONFIRMED: 'Підтвердити', rsDo_DECLINED: 'Відхилити', rsDo_SEATED: 'Посадили', rsDo_COMPLETED: 'Завершено', rsDo_NO_SHOW: 'Не прийшли', rsDo_CANCELLED_BY_VENUE: 'Скасувати',
    rsSt_REQUESTED: 'Запит', rsSt_CONFIRMED: 'Підтверджено', rsSt_SEATED: 'За столом', rsSt_COMPLETED: 'Завершено', rsSt_DECLINED: 'Відхилено',
    rsSt_CANCELLED_BY_GUEST: 'Гість скасував', rsSt_CANCELLED_BY_VENUE: 'Скасовано', rsSt_NO_SHOW: 'Не прийшли' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// How far ahead the day strip runs; the storefront books two weeks out.
const DAYS = 14;
/// The moves that end a booking ask first, and take a reason the guest sees.
const ASKS = ['DECLINED', 'CANCELLED_BY_VENUE', 'NO_SHOW'];
/// A party a phone call books; the kernel refuses past 20.
const PARTY_MAX = 20;

const ui = { day: 0, rows: [], plan: null };
const fail = e => toast(String(e.message || e));
const tz = () => S.venue?.tz || 'Europe/Tirane';
const off = () => offsetMinutes(tz(), Date.now());
/// The venue's day `n` as slot minutes, `[from, to)`.
const dayRange = n => { const from = Math.floor(midnightMs(Date.now(), off(), n) / 60_000); return [from, from + DAY_MIN]; };
const hhmm = m => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
const localMin = slot => (((slot + off()) % DAY_MIN) + DAY_MIN) % DAY_MIN;

function ensureCss(){
  if (document.getElementById('rsCss')) return;
  const l = document.createElement('link');
  l.id = 'rsCss'; l.rel = 'stylesheet'; l.href = '/admin/bookings.css';
  document.head.appendChild(l);
}

async function load(){
  const [from, to] = dayRange(ui.day);
  ui.rows = (await api(`/owner/reservations?from=${from}&to=${to}`)).reservations || [];
}

function row(r){
  const live = (r.next || []).length > 0;
  const table = r.tableN ? `${t('rsTable')} ${r.tableN}` : t('rsAny');
  return `<article class="rs-row ${live ? '' : 'off'}">
    <div class="rs-top"><span class="rs-time">${hhmm(localMin(r.slotMin))}</span>
      <span class="rs-who"><b>${esc(r.name || '-')}</b><small>${esc(r.party)} ${esc(t('rsGuests'))} · ${esc(table)}${r.occasion ? ` · ${esc(r.occasion)}` : ''}</small></span>
      <span class="rs-pill ${esc(r.status)}">${esc(t('rsSt_' + r.status))}</span></div>
    ${r.phone ? `<a class="muted small" href="tel:${esc(r.phone)}">${icon('phone')} ${esc(r.phone)}</a>` : ''}
    ${live ? `<div class="rs-acts">${r.next.map(to => `<button type="button" class="btn ${ASKS.includes(to) ? 'ghost' : ''}" data-id="${esc(r.id)}" data-to="${esc(to)}">${esc(t('rsDo_' + to))}</button>`).join('')}</div>` : ''}
  </article>`;
}

function draw(){
  const days = Array.from({ length: DAYS }, (_, n) => n);
  const names = new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: tz() });
  sheet(`<p class="eyebrow" data-t="roomGroup"></p><h2 data-t="bookings"></h2><p class="muted small" data-t="rsHint"></p>
    <div class="rs-days" role="radiogroup">${days.map(n => {
      const d = new Date(midnightMs(Date.now(), off(), n) + 12 * 3600_000);
      return `<button type="button" class="chip ${n === ui.day ? 'on' : ''}" data-day="${n}" aria-pressed="${n === ui.day}"><b>${esc(names.format(d))}</b><i>${new Intl.DateTimeFormat(undefined, { day: 'numeric', timeZone: tz() }).format(d)}</i></button>`;
    }).join('')}</div>
    <div class="btn-row"><button class="btn" id="rsNew">${icon('plus')}<span data-t="rsNew"></span></button></div>
    <p class="rs-count">${ui.rows.length} ${esc(t('rsCount'))} · ${ui.rows.filter(r => (r.next || []).length).reduce((s, r) => s + (r.party | 0), 0)} ${esc(t('rsGuests'))}</p>
    <div class="rs-list">${ui.rows.length ? ui.rows.map(row).join('') : `<div class="empty">${icon('clock')}<b data-t="rsNone"></b></div>`}</div>`,
    { name: 'bookings', keepScroll: true });
  const root = $('#sheetIn');
  for (const b of $$('[data-day]', root)) b.onclick = async () => { ui.day = +b.dataset.day; await refresh(); };
  for (const b of $$('[data-to]', root)) b.onclick = () => act(b.dataset.id, b.dataset.to, b);
  $('#rsNew', root).onclick = () => newBooking().catch(fail);
}

async function refresh(){ try { await load(); } catch (e) { fail(e); } draw(); }

async function act(id, to, btn){
  let reason = '';
  if (ASKS.includes(to)) {
    const ok = await confirm(t('bookings'), t('rsDo_' + to), { danger: true, reasonLabel: t('rsReason') });
    if (!ok) return refresh();
    reason = ok.reason || '';
  }
  try {
    await busy(btn, () => post(`/owner/reservations/${encodeURIComponent(id)}/action`, { to, reason }));
    toast(t('saved'));
  } catch (e) { fail(e); }
  await refresh();
}

/// A phone call, booked. The same public route a guest uses, with the owner's
/// token: the hub signs it VENUE and lands it CONFIRMED.
async function newBooking(){
  if (!ui.plan) { try { ui.plan = await api('/owner/floorplan'); } catch { ui.plan = { zones: [] }; } }
  const tables = (ui.plan.zones || []).flatMap(z => (z.tables || []).map(tb => ({ z, tb })));
  sheet(`<p class="eyebrow" data-t="bookings"></p><h2 data-t="rsNew"></h2>
    <label for="rn-name" data-t="rsName"></label><input id="rn-name" maxlength="80" autocomplete="off">
    <label for="rn-phone" data-t="rsPhone"></label><input id="rn-phone" type="tel" inputmode="tel" placeholder="+355...">
    <div class="grid2"><div><label for="rn-party" data-t="rsParty"></label><input id="rn-party" type="number" min="1" max="${PARTY_MAX}" value="2"></div>
    <div><label for="rn-time" data-t="rsTime"></label><input id="rn-time" type="time" step="900" value="20:00"></div></div>
    <label for="rn-table" data-t="rsTable"></label><select id="rn-table"><option value="" data-t="rsAny"></option>
      ${tables.map(({ z, tb }) => `<option value="${esc(z.id)}|${tb.n}">${esc(z.name)} · ${esc(t('rsTable'))} ${tb.n} (${tb.seats})</option>`).join('')}</select>
    <div class="btn-row"><button class="btn" id="rnGo" data-t="rsBook"></button></div>`, { name: 'bookingNew' });
  $('#rnGo').onclick = () => busy($('#rnGo'), async () => {
    const [h, m] = ($('#rn-time').value || '20:00').split(':').map(Number);
    const [from] = dayRange(ui.day);
    const pick = $('#rn-table').value.split('|');
    const body = {
      party: Math.max(1, Math.min(PARTY_MAX, Number($('#rn-party').value) | 0)), slotMin: from + h * 60 + m,
      contactName: $('#rn-name').value.trim(), contactPhone: $('#rn-phone').value.trim(),
      requestId: `console_${from + h * 60 + m}_${Math.random().toString(36).slice(2, 10)}`,
      ...(pick.length === 2 ? { zoneId: pick[0], tableN: Number(pick[1]) } : {}),
    };
    try {
      await post(`/public/locations/${encodeURIComponent(S.venue?.slug || '')}/reservations`, body);
      toast(t('saved'));
      await refresh();
    } catch (e) { fail(e); }
  });
}

export async function open(){
  ensureCss();
  ui.plan = null;
  await refresh();
}
