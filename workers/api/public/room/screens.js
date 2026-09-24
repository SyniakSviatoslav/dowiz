// The room app's top-level screens -- sign in, the room, one sitting -- as PURE
// functions of the app state, on /lib/ui. `app.js` keeps the state, the
// network and the taps, and assigns what these return to `#app.innerHTML`.
// Nothing here reads the DOM, the clock or the network (screens.test.mjs).
//
// THE CONTRACT app.js and the e2e walks drive (e2e/walk/_lib.mjs `roomIn`,
// waiter.mjs, a-room-open.mjs, c-room-clear.mjs):
//   form[data-form="login"], input[name="email"|"code"|"password"],
//   its button[type="submit"], [data-act="claimToggle"], the `.bar` with
//   [data-act="open"|"till"|"floor"|"refresh"], `.tables` of
//   [data-act="sit"][data-id] / [data-act="round"][data-id], [data-act="back"],
//   [data-act="moveSit"], [data-act="signout"].
//
// `c` is app.js's context: { S, t, statusWord, locale }.
import { money, canTill, sittingDue, canMoveSitting } from './logic.js';
import { sittingHasGuest } from './guest.js';
import { ui, k, act, backBar, actionRow, amt, loading } from './parts.js';

export function renderLogin(c) {
  const { S } = c;
  const claiming = !!S.claiming;
  return `${ui.section({ title: 'dowiz', sub: k('loginLine'), level: 2 })}
    <form data-form="login" class="card-form" novalidate>
      ${ui.field({ id: 'roomEmail', name: 'email', type: 'email', label: k('email'), autocomplete: 'username', required: true, enterkeyhint: 'next', attrs: { data: { tour: 'login.email' } } })}
      ${claiming ? ui.field({ id: 'roomCode', name: 'code', label: k('claimCode'), autocomplete: 'one-time-code', required: true, enterkeyhint: 'next', attrs: { data: { tour: 'login.code' } } }) : ''}
      ${ui.field({ id: 'roomPassword', name: 'password', type: 'password', label: k('password'), autocomplete: claiming ? 'new-password' : 'current-password',
        minlength: claiming ? 8 : 1, required: true, enterkeyhint: 'go', attrs: { data: { tour: 'login.password' } } })}
      ${ui.button({ type: 'submit', variant: 'primary', size: 'lg', block: true, icon: 'login', label: k(claiming ? 'claim' : 'signIn'), attrs: { data: { tour: 'login.submit' } } })}
      ${ui.button({ variant: 'ghost', block: true, icon: claiming ? 'user' : 'ticket', label: k(claiming ? 'haveAccount' : 'haveCode'), attrs: act('claimToggle', {}, 'login.claimToggle') })}
    </form>`;
}

/// One open table: its number, its rounds' states, a guest waiting, what is due.
function tableCard(c, s) {
  const { S, t } = c;
  const n = (s.rounds || []).length;
  const st = (s.rounds || []).map(r => ui.status({ status: r.status, label: c.statusWord(r.status) })).join('');
  const guest = sittingHasGuest(s) ? ui.badge({ tone: 'warning', icon: 'alert-circle', label: t('guestWaiting') }) : '';
  return `<li>${actionRow({ cls: 'card', title: `${t('table')} ${s.table || '—'}`,
    sub: `<span class="meta">${n} ${ui.esc(t('rounds'))} ${st}${guest}</span>`,
    trailing: `<span class="due">${ui.esc(t('due'))} ${amt(money(sittingDue(s), S.currency, c.locale()), { strong: true })}</span>`,
    attrs: act('sit', { id: s.sitting_id }, 'room.table') })}</li>`;
}

export function renderRoom(c) {
  const { S, t } = c;
  const orders = S.caps.has('take_orders');
  const cards = S.sittings.map(s => tableCard(c, s)).join('');
  const body = S.role === 'kitchen'
    ? ui.emptyState({ icon: 'tools-kitchen-2', title: k('kitchenNoRoom') })
    : cards ? `<ul class="tables">${cards}</ul>`
    : S.at ? ui.emptyState({ icon: 'receipt', title: k('noOrders') })
    : loading(t, 'card', 2);
  return `<div class="bar">${ui.badge({ icon: 'user', label: t(S.role || 'waiter'), attrs: { data: { tour: 'room.role' } } })}<span class="sp"></span>
      ${orders ? ui.button({ variant: 'primary', icon: 'plus', label: k('openTable'), attrs: act('open', {}, 'room.open') }) : ''}
      ${canTill(S.caps) ? ui.button({ icon: 'cash', label: k('till'), attrs: act('till', {}, 'room.till') }) : ''}
      ${orders ? ui.button({ icon: 'map-pin', label: k('floor'), attrs: act('floor', {}, 'room.floor') }) : ''}
      ${ui.iconButton({ icon: 'refresh', ariaLabel: k('refresh'), variant: 'plain', attrs: act('refresh', {}, 'room.refresh') })}</div>
    <h2>${ui.esc(t('room'))}</h2>
    ${body}
    ${ui.button({ variant: 'ghost', icon: 'logout', label: k('signOut'), attrs: act('signout', {}, 'room.signout') })}`;
}

/// A sitting with more than one round lists them; one round opens directly.
export function renderSitting(c, s) {
  const { S, t } = c;
  const loc = c.locale();
  const rounds = (s.rounds || []).map((r, i) => `<li>${actionRow({ cls: 'card', title: `#${i + 1}`,
    sub: ui.status({ status: r.status, label: c.statusWord(r.status) }),
    trailing: `<span class="due">${amt(money(r.total || 0, S.currency, loc), { strong: true })}</span>`,
    attrs: act('round', { id: r.id }, 'sitting.round') })}</li>`).join('');
  return `${backBar()}
    <h2>${ui.esc(t('table'))} ${ui.esc(s.table || '—')}</h2>
    <ul class="tables">${rounds}</ul>
    ${canMoveSitting(S.caps, s) ? `<div class="acts">${ui.button({ icon: 'arrows-sort', label: k('moveSitting'), attrs: act('moveSit', {}, 'sitting.move') })}</div>` : ''}`;
}
