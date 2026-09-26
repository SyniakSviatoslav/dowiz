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
