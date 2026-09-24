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

/// One formatter per zone name: building an `Intl.DateTimeFormat` costs far
/// more than using one, and the per-day arithmetic below asks many times.
const FORMATTERS = new Map();
function formatter(tzName) {
  let f = FORMATTERS.get(tzName);
  if (!f) {
    f = new Intl.DateTimeFormat('en-US', {
      timeZone: tzName, hourCycle: 'h23', year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit', second: '2-digit',
    });
    FORMATTERS.set(tzName, f);
  }
  return f;
}

/// The venue's UTC offset in minutes at an instant, from its zone NAME (the
/// menu's `location.tz`, e.g. `Europe/Tirane`). The hub sends a name and no
/// offset, so a caller that read `tzOffsetMinutes` read nothing and fell back
/// to +120 all year -- an hour wrong from the last Sunday of October. The
/// browser's own zone database answers; a name it does not know answers
/// `fallback`, loudly in the console.
export function offsetMinutes(tzName, atMs, fallback = 60) {
  try {
    const parts = formatter(tzName).formatToParts(new Date(atMs));
    const v = k => Number(parts.find(p => p.type === k)?.value);
    const asUtc = Date.UTC(v('year'), v('month') - 1, v('day'), v('hour') % 24, v('minute'), v('second'));
    return Math.round((asUtc - Math.floor(atMs / 1000) * 1000) / 60_000);
  } catch {
    console.error(`booking-time: unknown time zone ${JSON.stringify(tzName)}; using UTC+${fallback / 60}`);
    return fallback;
  }
}

// ── the venue's day by ZONE NAME, the offset read AT THE CANDIDATE ──────────
//
// THE FUNCTIONS ABOVE TAKE ONE OFFSET FOR EVERY DAY, and the screens passed the
// offset in force NOW. On 24 October (+120) a guest booking Monday 26 October
// at 19:00 (+60 by then) was stored as 17:00Z, which Tirane reads as 18:00, and
// the console's day of 25 October (25 hours long) lost its last hour. Every
// function below takes the zone NAME and asks the offset of the instant it is
// building, so a day index past the change is measured in that day's offset.
// Screens use these; the offset-taking forms stay for the arithmetic tests.

/// The venue's civil date, minute of day and weekday (0 = Monday) at `atMs`.
export function venueClock(tz, atMs) {
  const local = new Date(atMs + offsetMinutes(tz, atMs) * 60_000);
  return {
    y: local.getUTCFullYear(), mo: local.getUTCMonth(), d: local.getUTCDate(),
    minute: local.getUTCHours() * 60 + local.getUTCMinutes(),
    weekday: (local.getUTCDay() + 6) % 7,
  };
}

/// Epoch ms of the wall time `minute` on civil day (y, mo, d) in `tz`.
///
/// EVERY OFFSET THE ZONE HAS NEAR THAT DAY IS A CANDIDATE, and only a candidate
/// whose own offset agrees with the one used to build it is a real instant --
/// the lesson of `dowiz_hub::tz::start_of_local_day_ms`, where both the one-pass
/// and the two-pass form returned a wrong midnight on a transition day. A wall
/// time inside a spring-forward gap does not exist; it is carried forward by
/// the gap (02:30 becomes 03:30), which is what a clock on the wall shows.
export function wallMs(tz, y, mo, d, minute) {
  const naive = Date.UTC(y, mo, d) + minute * 60_000;
  const offs = new Set([-26, 0, 26].map(h => offsetMinutes(tz, naive + h * 3_600_000)));
  const real = [...offs].map(o => naive - o * 60_000)
    .filter(t => naive - offsetMinutes(tz, t) * 60_000 === t);
  if (real.length) return Math.min(...real);
  return naive - offsetMinutes(tz, naive - offsetMinutes(tz, naive) * 60_000) * 60_000;
}

/// The venue's civil day `n` days from `nowMs`, with its own weekday.
export function venueDate(nowMs, tz, n = 0) {
  const c = venueClock(tz, nowMs);
  const day = new Date(Date.UTC(c.y, c.mo, c.d + n));
  return { y: day.getUTCFullYear(), mo: day.getUTCMonth(), d: day.getUTCDate(), weekday: (day.getUTCDay() + 6) % 7 };
}

/// Epoch ms of the venue's midnight `n` days from `nowMs`.
export function venueMidnightMs(nowMs, tz, n = 0) {
  const c = venueClock(tz, nowMs);
  return wallMs(tz, c.y, c.mo, c.d + n, 0);
}

/// The kernel's slot (minutes since the epoch) for venue-day `n` at local `minute`.
export function venueSlot(nowMs, tz, n, minute) {
  const c = venueClock(tz, nowMs);
  return Math.round(wallMs(tz, c.y, c.mo, c.d + n, minute) / 60_000);
}

/// Venue-day `n` as slot minutes `[from, to)`, FROM TWO MIDNIGHTS: a day is 23,
/// 24 or 25 hours long, and `from + 1440` drops or doubles an hour twice a year.
export function venueDayRange(nowMs, tz, n = 0) {
  return [Math.round(venueMidnightMs(nowMs, tz, n) / 60_000), Math.round(venueMidnightMs(nowMs, tz, n + 1) / 60_000)];
}

/// The local minute of day at which a stored slot falls, in that slot's own offset.
export const slotMinuteOfDay = (tz, slotMin) => venueClock(tz, slotMin * 60_000).minute;

/// A `datetime-local` value ("2026-10-26T19:00") read as the VENUE's wall time.
/// `new Date(value)` reads it in the phone's zone: a phone in Kyiv picking 19:00
/// sent the kitchen 18:00. Null for anything that is not that shape.
export function venueWallMs(tz, value) {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(String(value || ''));
  if (!m) return null;
  const [y, mo, d, h, mi] = m.slice(1).map(Number);
  if (mo < 1 || mo > 12 || d < 1 || d > 31 || h > 23 || mi > 59) return null;
  return wallMs(tz, y, mo - 1, d, h * 60 + mi);
}

/// The inverse: an instant as a `datetime-local` value on the venue's wall.
export function venueWallValue(tz, atMs) {
  const c = venueClock(tz, atMs);
  const p = n => String(n).padStart(2, '0');
  return `${c.y}-${p(c.mo + 1)}-${p(c.d)}T${p(Math.floor(c.minute / 60))}:${p(c.minute % 60)}`;
}

/// The checkout's "later" prefill: `lead` from now, rounded up to the next
/// `round` minutes of the VENUE's clock, as a `datetime-local` value on the
/// venue's wall. Pure, so the zone of the phone running it cannot reach it.
export function laterPrefill(tz, nowMs, leadMs, round = 30) {
  const at = Math.floor((nowMs + leadMs) / 60_000) * 60_000;
  const mm = venueClock(tz, at).minute % round;
  return venueWallValue(tz, at + (mm ? round - mm : 0) * 60_000);
}
