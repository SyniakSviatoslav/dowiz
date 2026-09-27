// One email and password, two doors: the owner's, then staff's (lane W-QA, 2026-09-26).
//
// THE KITCHEN COULD NOT SIGN IN. The console asked `/api/auth/login` first and
// read its answer with `r.json()` -- but the hub refuses with PLAIN TEXT
// (`Response::error`: "no active owner membership", "invalid credentials").
// The parse threw before `/api/staff/login` was ever asked, so every member of
// staff was shown `Unexpected token 'o', "no active o"... is not valid JSON`,
// and an owner who mistyped their password saw the same kind of line instead
// of "invalid credentials" (QA walk Q2, live on qa-durres).
//
// PURE but for the `fetch` it is handed, so node tests every answer
// (`signin.test.mjs`). ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
import { bodyOf } from '../lib/body.js';

/// Sign in. Answers `{ token, refresh, loc, staff }` or throws an Error whose
/// message is the hub's own words.
export async function signIn(fetchFn, email, password){
  const body = JSON.stringify({ email, password });
  const ask = url => fetchFn(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body });
  const r = await ask('/api/auth/login');
  const d = await bodyOf(r);
  if (r.ok && d.user && d.user.locationId) return { token: d.access_token, refresh: d.refresh_token || null, loc: d.user.locationId, staff: false };
  // THE SAME HUB FOR THE KITCHEN (operator Q3/Q8): a person who is staff here
  // signs in with the same email and password and gets their role's tabs.
  const sr = await ask('/api/staff/login');
  const sd = await bodyOf(sr);
  if (sr.ok && sd.jwt && sd.staff && sd.staff.locationId) return { token: sd.jwt, refresh: null, loc: sd.staff.locationId, staff: true };
  // Staff's words when the owner door only said "not an owner here"; else the owner's.
  const owners = d.error || d.message || '';
  const why = /no active owner membership/.test(owners) ? (sd.error || owners) : (owners || sd.error);
  throw new Error(why || `HTTP ${sr.status || r.status}`);
}

/// "I have a staff code" (lane W-KACCESS, 2026-09-27): the invite's code, the
/// person's email and a password turn into an account and a signed-in console
/// -- the same `POST /api/staff/claim` the room app uses, so a cook never needs
/// the room app to start. An address that already has an account keeps its
/// password; the hub says so in its own words. Nothing is sent when a field is
/// empty. Answers what `signIn` answers.
export async function claim(fetchFn, email, code, password){
  const e = String(email || '').trim(), c = String(code || '').trim();
  if (!e || !c || !password) throw new Error('missing');
  const r = await fetchFn('/api/staff/claim', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email: e, code: c, password }) });
  const d = await bodyOf(r);
  if (r.ok && d.jwt && d.staff && d.staff.locationId) return { token: d.jwt, refresh: null, loc: d.staff.locationId, staff: true };
  throw new Error(d.error || d.message || `HTTP ${r.status}`);
}

/// The fewest characters the hub accepts for a password (`staff_rules`
/// MIN_PASSWORD_CHARS); checked here only to answer before the round trip.
export const MIN_PASSWORD_CHARS = 8;

/// A member of staff changes their own password (`POST /api/staff/password`,
/// main 282d2c45). The hub checks the old one and ENDS every open staff
/// session of the person, this device's too -- so this signs in again with
/// the new password and answers what `signIn` answers: the console carries on.
export async function changePassword(fetchFn, email, oldPassword, newPassword){
  const e = String(email || '').trim();
  if (!e || !oldPassword || !newPassword) throw new Error('missing');
  if ([...String(newPassword)].length < MIN_PASSWORD_CHARS) throw new Error('short');
  const r = await fetchFn('/api/staff/password', { method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: e, old_password: oldPassword, new_password: newPassword }) });
  const d = await bodyOf(r);
  if (!(r.ok && d.changed)) throw new Error(d.error || d.message || `HTTP ${r.status}`);
  return signIn(fetchFn, e, newPassword);
}
