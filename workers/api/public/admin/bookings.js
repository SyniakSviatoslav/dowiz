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
import { ui, btn, field, input, select, empty } from '/admin/parts.js';
import { venueDayRange, venueMidnightMs, venueSlot, slotMinuteOfDay } from '/lib/booking-time.js';

const WORDS = {
  sq: { rsHint: 'Rezervimet e ditës. Konfirmoni, uleni ose anuloni; asgjë nuk fshihet.', rsNone: 'Asnjë rezervim këtë ditë.',
    rsNew: 'Rezervim i ri', rsName: 'Emri', rsPhone: 'Telefoni', rsParty: 'Persona', rsTime: 'Ora', rsTable: 'Tavolina', rsAny: 'Çdo tavolinë',
    rsBook: 'Rezervo', rsReason: 'Arsyeja (klienti e sheh)', rsCount: 'rezervime', rsGuests: 'persona',
    rsDo_CONFIRMED: 'Konfirmo', rsDo_DECLINED: 'Refuzo', rsDo_SEATED: 'U ulën', rsDo_COMPLETED: 'Mbaroi', rsDo_NO_SHOW: 'Nuk erdhi', rsDo_CANCELLED_BY_VENUE: 'Anulo',
    rsSt_REQUESTED: 'Kërkesë', rsSt_CONFIRMED: 'Konfirmuar', rsSt_SEATED: 'Në tavolinë', rsSt_COMPLETED: 'Mbaroi', rsSt_DECLINED: 'Refuzuar',
    rsSt_CANCELLED_BY_GUEST: 'Anuloi klienti', rsSt_CANCELLED_BY_VENUE: 'Anuluar', rsSt_NO_SHOW: 'Nuk erdhi', rsLate: 'anuloi vonë' },
  en: { rsHint: 'The day\'s bookings. Confirm, seat or cancel; nothing is erased.', rsNone: 'No bookings on this day.',
    rsNew: 'New booking', rsName: 'Name', rsPhone: 'Phone', rsParty: 'Guests', rsTime: 'Time', rsTable: 'Table', rsAny: 'Any table',
    rsBook: 'Book', rsReason: 'Reason (the guest sees it)', rsCount: 'bookings', rsGuests: 'guests',
    rsDo_CONFIRMED: 'Confirm', rsDo_DECLINED: 'Decline', rsDo_SEATED: 'Seated', rsDo_COMPLETED: 'Finished', rsDo_NO_SHOW: 'No-show', rsDo_CANCELLED_BY_VENUE: 'Cancel',
    rsSt_REQUESTED: 'Request', rsSt_CONFIRMED: 'Confirmed', rsSt_SEATED: 'Seated', rsSt_COMPLETED: 'Finished', rsSt_DECLINED: 'Declined',
    rsSt_CANCELLED_BY_GUEST: 'Guest cancelled', rsSt_CANCELLED_BY_VENUE: 'Cancelled', rsSt_NO_SHOW: 'No-show', rsLate: 'cancelled late' },
  uk: { rsHint: 'Бронювання дня. Підтвердіть, посадіть або скасуйте; нічого не стирається.', rsNone: 'Цього дня бронювань немає.',
    rsNew: 'Нове бронювання', rsName: "Ім'я", rsPhone: 'Телефон', rsParty: 'Гостей', rsTime: 'Час', rsTable: 'Стіл', rsAny: 'Будь-який стіл',
    rsBook: 'Забронювати', rsReason: 'Причина (гість її бачить)', rsCount: 'бронювань', rsGuests: 'гостей',
    rsDo_CONFIRMED: 'Підтвердити', rsDo_DECLINED: 'Відхилити', rsDo_SEATED: 'Посадили', rsDo_COMPLETED: 'Завершено', rsDo_NO_SHOW: 'Не прийшли', rsDo_CANCELLED_BY_VENUE: 'Скасувати',
    rsSt_REQUESTED: 'Запит', rsSt_CONFIRMED: 'Підтверджено', rsSt_SEATED: 'За столом', rsSt_COMPLETED: 'Завершено', rsSt_DECLINED: 'Відхилено',
    rsSt_CANCELLED_BY_GUEST: 'Гість скасував', rsSt_CANCELLED_BY_VENUE: 'Скасовано', rsSt_NO_SHOW: 'Не прийшли', rsLate: 'скасував пізно' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// How far ahead the day strip runs; the storefront books two weeks out.
const DAYS = 14;
/// The moves that end a booking ask first, and take a reason the guest sees.
const ASKS = ['DECLINED', 'CANCELLED_BY_VENUE', 'NO_SHOW'];
/// A party a phone call books; the kernel refuses past 20.
const PARTY_MAX = 20;

const view = { day: 0, rows: [], plan: null };
const fail = e => toast(String(e.message || e));
const tz = () => S.venue?.tz || 'Europe/Tirane';
/// The venue's day `n` as slot minutes, `[from, to)`, FROM TWO MIDNIGHTS, each
/// in its own offset: 25 October is 25 hours long, and `from + 1440` dropped
/// its last hour (audit D3). A slot is shown in the offset of its own instant.
const dayRange = n => venueDayRange(Date.now(), tz(), n);
const hhmm = m => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
const localMin = slot => slotMinuteOfDay(tz(), slot);

function ensureCss(){
  if (document.getElementById('rsCss')) return;
  const l = document.createElement('link');
  l.id = 'rsCss'; l.rel = 'stylesheet'; l.href = '/admin/bookings.css';
  document.head.appendChild(l);
}

async function load(){
  const [from, to] = dayRange(view.day);
  view.rows = (await api(`/owner/reservations?from=${from}&to=${to}`)).reservations || [];
}

function row(r){
  const live = (r.next || []).length > 0;
  const table = r.tableN ? `${t('rsTable')} ${r.tableN}` : t('rsAny');
  return `<article class="rs-row ${live ? '' : 'off'}" data-tour="bookings.row">
    <div class="rs-top"><span class="rs-time">${hhmm(localMin(r.slotMin))}</span>
      <span class="rs-who"><b>${esc(r.name || '-')}</b><small>${esc(r.party)} ${esc(t('rsGuests'))} · ${esc(table)}${r.occasion ? ` · ${esc(r.occasion)}` : ''}${r.lateCancel ? ` · ${esc(t('rsLate'))}` : ''}</small></span>
      <span class="rs-pill ${esc(r.status)}">${esc(t('rsSt_' + r.status))}</span></div>
    ${r.phone ? `<a class="muted small" href="tel:${esc(r.phone)}">${icon('phone')} ${esc(r.phone)}</a>` : ''}
    ${live ? `<div class="rs-acts">${r.next.map(to => btn({ variant: ASKS.includes(to) ? 'ghost' : 'secondary', key: 'rsDo_' + to,
      data: { id: r.id, to }, tour: 'bookings.' + (to === 'CONFIRMED' ? 'confirm' : to === 'DECLINED' ? 'decline' : to.toLowerCase()) })).join('')}</div>` : ''}
  </article>`;
}

function draw(){
  const days = Array.from({ length: DAYS }, (_, n) => n);
  const names = new Intl.DateTimeFormat(undefined, { weekday: 'short', timeZone: tz() });
  sheet(`<p class="eyebrow" data-t="roomGroup"></p><h2 data-t="bookings"></h2><p class="muted small" data-t="rsHint"></p>
    <div class="rs-days" role="radiogroup">${days.map(n => {
      const d = new Date(venueMidnightMs(Date.now(), tz(), n) + 12 * 3600_000);
      return `<button type="button" class="ui-chip rs-day" data-day="${n}" data-tour="bookings.day" aria-pressed="${n === view.day}"><b>${esc(names.format(d))}</b><i>${new Intl.DateTimeFormat(undefined, { day: 'numeric', timeZone: tz() }).format(d)}</i></button>`;
    }).join('')}</div>
    <div class="btn-row">${btn({ id: 'rsNew', variant: 'primary', icon: 'plus', key: 'rsNew', tour: 'bookings.new' })}</div>
    <p class="rs-count">${view.rows.length} ${esc(t('rsCount'))} · ${view.rows.filter(r => (r.next || []).length).reduce((s, r) => s + (r.party | 0), 0)} ${esc(t('rsGuests'))}</p>
    <div class="rs-list" data-tour="bookings.list">${view.rows.length ? view.rows.map(row).join('') : empty('clock', { key: 'rsNone' })}</div>`,
    { name: 'bookings', keepScroll: true });
  const root = $('#sheetIn');
  for (const b of $$('[data-day]', root)) b.onclick = async () => { view.day = +b.dataset.day; await refresh(); };
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
  if (!view.plan) { try { view.plan = await api('/owner/floorplan'); } catch { view.plan = { zones: [] }; } }
  const tables = (view.plan.zones || []).flatMap(z => (z.tables || []).map(tb => ({ z, tb })));
  sheet(`<p class="eyebrow" data-t="bookings"></p><h2 data-t="rsNew"></h2>
    ${field({ id: 'rn-name', key: 'rsName', maxlength: 80, autocomplete: 'off', tour: 'bookings.name' })}
    ${field({ id: 'rn-phone', key: 'rsPhone', type: 'tel', inputmode: 'tel', placeholder: '+355...', tour: 'bookings.phone' })}
    <div class="grid2">${field({ id: 'rn-party', key: 'rsParty', type: 'number', value: 2, attrs: { min: 1, max: PARTY_MAX }, tour: 'bookings.party' })}
    ${input({ id: 'rn-time', type: 'time', key: 'rsTime', step: 900, value: '20:00', tour: 'bookings.time' })}</div>
    ${select({ id: 'rn-table', key: 'rsTable', value: '', tour: 'bookings.table', options: [{ value: '', key: 'rsAny' },
      ...tables.map(({ z, tb }) => ({ value: `${z.id}|${tb.n}`, label: `${z.name} · ${t('rsTable')} ${tb.n} (${tb.seats})` }))] })}
    <div class="btn-row">${btn({ id: 'rnGo', variant: 'primary', icon: 'check', key: 'rsBook', tour: 'bookings.book' })}</div>`, { name: 'bookingNew' });
  $('#rnGo').onclick = () => busy($('#rnGo'), async () => {
    const [h, m] = ($('#rn-time').value || '20:00').split(':').map(Number);
    // The wall time on THAT day, in that day's offset: `midnight + h*60 + m`
    // is an hour off on 25 October, whose midnight is in the other offset.
    const slotMin = venueSlot(Date.now(), tz(), view.day, h * 60 + m);
    const pick = $('#rn-table').value.split('|');
    const body = {
      party: Math.max(1, Math.min(PARTY_MAX, Number($('#rn-party').value) | 0)), slotMin,
      contactName: $('#rn-name').value.trim(), contactPhone: $('#rn-phone').value.trim(),
      requestId: `console_${slotMin}_${Math.random().toString(36).slice(2, 10)}`,
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
  view.plan = null;
  await refresh();
}
