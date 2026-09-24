// A booking's SLOT: minutes since the Unix epoch, in the venue's own day.
//
// PURE, AND SEPARATE FROM THE SCREEN, for two reasons.
//
// IT READS NO CLOCK. `now` is an argument, the same correction
// `workers/api/src/booking.rs::now_min` made: a second read of the clock can
// land a minute away from the one the rest of the flow used, and for a booking
// window that is the difference between accepted and refused. It also means
// every rule below can be examined at a chosen instant instead of only at the
// real time -- which is why `booking-time.test.mjs` exists and can run in node
// with no browser at all.
//
// IT IS THE VENUE'S DAY, NOT THE PHONE'S. A diner in Kyiv booking a table in
// Durrës means the venue's 19:00. The offset is the number the hub published
// (`location.tzOffsetMinutes`), never `Date.getTimezoneOffset()`, and every
// step is integer arithmetic over it.

/// Offered on the half hour. Finer is a list nobody scrolls.
export const SLOT_STEP_MIN = 30;
/// No booking inside the last hour before the kitchen closes.
export const LAST_SITTING_MIN = 60;
/// Minutes in a day, and the wrap point `dowiz_hub::hours` uses.
export const DAY_MIN = 1440;

/// Epoch ms of local midnight, `n` venue-days from `nowMs`.
export function midnightMs(nowMs, offsetMin, n = 0) {
  const local = new Date(nowMs + offsetMin * 60_000);
  const day0 = Date.UTC(local.getUTCFullYear(), local.getUTCMonth(), local.getUTCDate());
  return day0 + n * 86_400_000 - offsetMin * 60_000;
}

/// The kernel's unit: minutes since the epoch, for venue-day `n` at local
/// minute `minute`.
export const slotOf = (nowMs, offsetMin, n, minute) =>
  Math.round(midnightMs(nowMs, offsetMin, n) / 60_000) + minute;

/// The venue's own weekday, 0 = Monday, the way `dowiz_hub::hours` counts.
export function weekdayOf(nowMs, offsetMin, n = 0) {
  const local = new Date(nowMs + offsetMin * 60_000 + n * 86_400_000);
  return (local.getUTCDay() + 6) % 7;
}

/// Local minute of the venue's day, right now.
export const minuteNow = (nowMs, offsetMin) => {
  const local = new Date(nowMs + offsetMin * 60_000);
  return local.getUTCHours() * 60 + local.getUTCMinutes();
};

/// Every bookable minute in a day's opening windows.
///
/// A WINDOW THAT CLOSES BEFORE IT OPENS WRAPS PAST MIDNIGHT. `hours.rs` reads
/// `close <= open` that way, and a venue serving 18:00-02:00 is the normal
/// case here -- refusing it would close every late kitchen on the platform.
/// The close is carried past midnight so the last sitting is measured against
/// 02:00 and not against 00:00, which would have cut an hour off the evening.
///
/// KNOWN AND STATED: the minutes AFTER midnight are not offered at all. They
/// belong to the previous day's window but to the next calendar day, and this
/// function is asked one day at a time. A guest wanting 00:30 phones the venue.
/// It errs towards offering LESS than the venue is open, never more: a time
/// this returns is a time the kitchen is lit.
///
/// `nowMinute` drops the times that have already gone today. Pass `-1` for any
/// day that is not today -- a future day has no past.
export function timesOn(windows, nowMinute = -1, step = SLOT_STEP_MIN, last = LAST_SITTING_MIN) {
  const out = [];
  for (const w of windows || []) {
    const open = Number(w?.open), raw = Number(w?.close);
    if (!Number.isFinite(open) || !Number.isFinite(raw)) continue;
    const close = raw <= open ? raw + DAY_MIN : raw;
    for (let m = open; m + last <= close && m < DAY_MIN; m += step) {
      if (m > nowMinute) out.push(m);
    }
  }
  // Two windows on one day (a lunch and a dinner service) come back as one
  // ordered strip, and a venue that filed them overlapping does not show the
  // same time twice.
  return [...new Set(out)].sort((a, b) => a - b);
}

/// The venue's UTC offset in minutes at an instant, from its zone NAME (the
/// menu's `location.tz`, e.g. `Europe/Tirane`). The hub sends a name and no
/// offset, so a caller that read `tzOffsetMinutes` read nothing and fell back
/// to +120 all year -- an hour wrong from the last Sunday of October. The
/// browser's own zone database answers; a name it does not know answers
/// `fallback`, loudly in the console.
export function offsetMinutes(tzName, atMs, fallback = 60) {
  try {
    const parts = new Intl.DateTimeFormat('en-US', {
      timeZone: tzName, hourCycle: 'h23', year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit', second: '2-digit',
    }).formatToParts(new Date(atMs));
    const v = k => Number(parts.find(p => p.type === k)?.value);
    const asUtc = Date.UTC(v('year'), v('month') - 1, v('day'), v('hour') % 24, v('minute'), v('second'));
    return Math.round((asUtc - Math.floor(atMs / 1000) * 1000) / 60_000);
  } catch {
    console.error(`booking-time: unknown time zone ${JSON.stringify(tzName)}; using UTC+${fallback / 60}`);
    return fallback;
  }
}
