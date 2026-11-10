// The arithmetic of the payroll tab, with no DOM: what the owner typed, as
// integer minutes; a month stepped back and forth; the preview of one entry.
// The hub computes the totals (`services/operations/payroll/rules.rs`): this
// file only reads what a person typed and shows a preview of the same rule.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

/// Hours as a person types them ("7.5", "7,25", "7:30", "8") as integer
/// minutes in quarters of an hour; `null` when it is not one, or is under a
/// quarter or above a whole day.
export function parseHours(text){
  const s = String(text ?? '').trim().replace(',', '.');
  let m = /^(\d{1,2}):(\d{2})$/.exec(s);
  if (m) {
    if (!['00', '15', '30', '45'].includes(m[2])) return null;
    return bounded(Number(m[1]) * 60 + Number(m[2]));
  }
  m = /^(\d{1,2})(?:\.(\d{1,2}))?$/.exec(s);
  if (!m) return null;
  const quarters = { '': 0, '0': 0, '00': 0, '25': 15, '5': 30, '50': 30, '75': 45 };
  const f = quarters[m[2] ?? ''];
  return f === undefined ? null : bounded(Number(m[1]) * 60 + f);
}
const bounded = minutes => (minutes >= 15 && minutes <= 24 * 60 ? minutes : null);

/// Minutes as the hours a person reads: 450 -> "7.5", 465 -> "7.75", 480 -> "8".
export function hoursLabel(minutes){
  const m = Number(minutes) || 0;
  const whole = Math.floor(m / 60), rest = m % 60;
  const part = { 0: '', 15: '.25', 30: '.5', 45: '.75' }[rest];
  return part === undefined ? `${whole}:${String(rest).padStart(2, '0')}` : `${whole}${part}`;
}

/// One entry's pay: minutes x rate / 60, rounded half up, in minor units. A
/// preview only -- the month total is the hub's, rounded once per employee.
export const entryPay = (minutes, rateMinor) => Math.floor((minutes * rateMinor + 30) / 60);

/// "YYYY-MM" stepped by whole months (no Date: no zone can move it).
export function shiftMonth(month, delta){
  const [y, m] = month.split('-').map(Number);
  const i = y * 12 + (m - 1) + delta;
  return `${Math.floor(i / 12)}-${String((i % 12) + 1).padStart(2, '0')}`;
}

/// "YYYY-MM" as a title in the reader's language.
export function monthTitle(month, locale){
  const [y, m] = month.split('-').map(Number);
  return new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString(locale, { month: 'long', year: 'numeric', timeZone: 'UTC' });
}

/// The entered value of one person on one date in a month's answer, or null.
export function cellOf(data, staffId, date){
  const row = (data?.rows || []).find(r => r.staffId === staffId);
  return row?.days?.[date] || null;
}

/// The days the entry form may name while `month` is on screen: the month's
/// own, and none after `today`. The form knows only that month's entries, so a
/// date outside it would be drawn as empty when it is not. No Date: no zone.
export function monthBounds(month, today){
  const [y, m] = month.split('-').map(Number);
  const leap = (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;
  const last = `${month}-${String([31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1]).padStart(2, '0')}`;
  return { min: `${month}-01`, max: today && today < last ? today : last };
}

/// Whether `date` (yyyy-mm-dd) is a day those bounds allow.
export const dateIn = (date, { min, max }) => /^\d{4}-\d{2}-\d{2}$/.test(String(date || '')) && date >= min && date <= max;
