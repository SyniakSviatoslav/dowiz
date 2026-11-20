// HOW A WRITE ENDED, AS ONE NOTICE. PURE: no DOM, no network (the test is
// `outcome-logic.test.mjs`); the console's api() (`admin/core.js`) and the
// courier's taps (`courier/app.js`) call it for every write a person makes,
// so no screen can end a save in silence or say "saved" for a refusal. Saved
// is green; not saved is red, with the reason by status in the reader's
// language (`outcome-words.js`) and the hub's own words beside it.
//
// ASCII QUOTES ONLY in this file.

/// The status that says why, as a word key.
const WHY = [[s => s === 400 || s === 422, 'out_bad'], [s => s === 401 || s === 403, 'out_denied'], [s => s === 404, 'out_missing'],
  [s => s === 409, 'out_conflict'], [s => s === 413, 'out_tooBig'], [s => s === 429, 'out_busy'], [s => s >= 500, 'out_server']];
/// The hub's own words are cut to one glance.
const HUB_WORDS_MAX = 160;

/// `{ ok, status, message, offline }` -> `{ text, tone, icon }`.
export function notice(o, t){
  if (o.ok) return { text: t('saved'), tone: 'success', icon: 'circle-check' };
  if (o.offline) return { text: `${t('out_notSaved')}: ${t('out_offline')}`, tone: 'danger', icon: 'wifi-off' };
  const hit = WHY.find(([is]) => is(o.status));
  const why = hit ? t(hit[1]) : `${t('out_failed')} ${o.status}`;
  let said = String(o.message || '').trim();
  if (/^HTTP \d+$/.test(said)) said = '';
  if (said.length > HUB_WORDS_MAX) said = said.slice(0, HUB_WORDS_MAX - 1) + '…';
  return { text: `${t('out_notSaved')}: ${why}${said ? ` (${said})` : ''}`, tone: 'danger', icon: 'alert-circle' };
}

/// A landed write's notice, said after the turn, and only when nothing was
/// toasted in it: a screen's own word ("Delivered") is more specific and wins.
/// `count` reads how many toasts were shown; `defer` runs a function later.
export function unlessToasted(count, show, defer = f => setTimeout(f, 0)){
  const before = count();
  defer(() => { if (count() === before) show(); });
}

/// The calls a session makes for itself: never a person's action.
const SESSION = /^\/(auth|staff)\/(refresh|login|logout)\b|^\/telemetry\b|\/push\/subscribe\b/;
const WRITES = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);

/// Is this call a write whose ending a person is told?
export function announces(method, path, opts = {}){
  return WRITES.has(String(method || 'GET').toUpperCase()) && !opts.quiet && !SESSION.test(String(path));
}
