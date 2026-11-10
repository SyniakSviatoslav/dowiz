// The money sheet's small rules, pure and tested: the month arithmetic of the
// picker, the form's check, what a source row says about itself. A typed
// amount is read by the console's one money parser (`parseMinor`). The hub
// does every sum; nothing here adds money up.
//
// ASCII QUOTES ONLY in this file.
import { parseMinor } from './wire-logic.js';

/// '2026-01' moved by whole months.
export function shiftMonth(m, by){
  const [y, mm] = String(m).split('-').map(Number);
  const n = y * 12 + (mm - 1) + by;
  return `${String(Math.floor(n / 12)).padStart(4, '0')}-${String((n % 12) + 1).padStart(2, '0')}`;
}

/// The month picker's words: 'листопад 2026'.
export function monthName(m, locale = 'en'){
  const [y, mm] = String(m).split('-').map(Number);
  return new Date(Date.UTC(y, mm - 1, 1)).toLocaleDateString(locale, { month: 'long', year: 'numeric', timeZone: 'UTC' });
}

/// '2026-11-03' as the row's short date: '03.11'.
export const shortDate = d => `${String(d).slice(8, 10)}.${String(d).slice(5, 7)}`;

/// The date a new entry starts on: today in the present month, the first day
/// of any other month on screen, so an entry made while reading October lands
/// in October unless the owner says otherwise.
export const startDate = (month, today) => (String(today || '').slice(0, 7) === month || !month ? String(today || '') : `${month}-01`);

/// The form's values as the body to send, or the word that says what is
/// missing. `today` (yyyy-mm-dd) refuses a date that has not happened; `id` is
/// the form's own id, so a repeated send is the same entry and not a second one.
export function checkForm({ date, name, amount }, kind, decimals = 0, { today = '', id = '' } = {}){
  if (!/^\d{4}-\d{2}-\d{2}$/.test(String(date || ''))) return { err: 'fn_noDate' };
  if (today && date > today) return { err: 'fn_futureDate' };
  const n = String(name || '').trim();
  if (!n) return { err: 'fn_noName' };
  const minor = parseMinor(amount, decimals);
  if (minor == null || minor <= 0) return { err: 'fn_noAmount' };
  return { body: { date, name: n, amount: minor, kind, ...(id ? { client_id: id } : {}) } };
}

/// A new form's id: letters and digits the hub accepts as an entry's client id.
export const formId = (now, salt) => `${Number(now).toString(36)}${String(salt).replace(/[^a-z0-9]/g, '').slice(0, 10)}`;

/// A source row's i18n key: `orders` -> 'fn_src_orders'.
export const srcKey = key => 'fn_src_' + key;

/// The sentence under the purchases row, or null when it has nothing to say.
export function purchasesHint(row){
  if (!row || row.key !== 'purchases') return null;
  if (!row.receipts) return 'fn_noReceipts';
  if (row.uncosted) return 'fn_uncosted';
  return null;
}

/// How many deliveries carry no date and so sit in no month's purchases; 0 for any other row.
export const undatedOf = row => (row && row.key === 'purchases' && Number.isSafeInteger(row.undated) && row.undated > 0 ? row.undated : 0);
