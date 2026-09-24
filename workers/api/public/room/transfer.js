// MOVE LINES TO ANOTHER ROUND, and MOVE A SITTING TO ANOTHER TABLE
// (`POST /api/staff/orders/:id/transfer`, `POST /api/staff/sittings/:id/move`,
// services/orders/room/transfer.rs; BLUEPRINT-POS-THE-ROOM §2.9).
//
// THE SAME WIRE AS `pay.js`: one Idempotency-Key minted at the tap, the outbox
// when the network did not carry it, and a refusal the server ANSWERED is said
// in the waiter's words (`logic.refusalKey`) or, when it is not one of the
// known phrases, in the server's own. A stale version reloads the room -- the
// lines are indices into what THIS screen showed, and moving "line 2" of a
// round someone else just changed moves the wrong dish.
import { money, transferTargets, transferBody, refusalKey } from './logic.js';
import { ui, k, act, backBar, amt } from './parts.js';

/// The picked lines and target, per source round, so a redraw keeps them.
function form(c, round) {
  const S = c.S;
  if (!S.moveForm || S.moveForm.id !== round.id) S.moveForm = { id: round.id, lines: [], to: null };
  return S.moveForm;
}

export function renderTransfer(c, round) {
  const { S, t } = c, cur = S.currency, loc = c.locale(), f = form(c, round);
  const items = Array.isArray(round.items) ? round.items : [];
  const targets = transferTargets(S.caps, S.sittings, round.id);
  // Lines are picked MANY at a time, destinations ONE: both are pressed rows.
  const lines = items.map((it, i) => ui.row({ select: true, pressed: f.lines.includes(i), cls: 'line pick',
    title: `${Number(it.quantity || 0)}× ${it.name || it.product_id || ''}`,
    trailing: amt(money(Number(it.unit_price || 0) * Number(it.quantity || 0), cur, loc)),
    attrs: act('line', { line: i }, 'transfer.line') })).join('');
  const dests = targets.map(({ sitting, round: r }) => ui.row({ select: true, pressed: f.to === r.id,
    title: `${t('table')} ${r.fulfilment?.table || sitting.table || '—'}`, trailing: amt(money(r.total || 0, cur, loc)),
    attrs: act('to', { id: r.id }, 'transfer.target') })).join('');
  return `
    ${backBar()}
    <h2>${ui.esc(t('moveLines'))} · ${ui.esc(t('table'))} ${ui.esc(c.tableOf(round) || '—')}</h2>
    ${ui.para(k('pickLines'), { hint: true })}
    ${lines ? ui.list([lines], { label: t('pickLines'), cls: 'lines' }) : ui.emptyState({ icon: 'receipt', title: k('noLines') })}
    <h3>${ui.esc(t('moveLinesTo'))}</h3>
    ${dests ? ui.list([dests], { label: t('moveLinesTo') }) : ui.emptyState({ icon: 'map-pin', title: k('noTargets') })}
    <div class="acts">${ui.button({ variant: 'primary', size: 'lg', block: true, icon: 'arrows-sort', label: k('moveLines'), disabled: !dests, attrs: act('send', {}, 'transfer.send') })}</div>`;
}

export function bindTransfer(c, root, round) {
  const { S, t } = c;
  root.onclick = async ev => {
    const b = ev.target.closest('[data-act]');
    if (!b || b.disabled) return;
    const f = form(c, round), act = b.dataset.act;
    if (act === 'back') { S.view = 'round'; S.moveForm = null; return c.render(); }
    if (act === 'line') {
      const i = Number(b.dataset.line);
      f.lines = f.lines.includes(i) ? f.lines.filter(x => x !== i) : [...f.lines, i];
      return c.render();
    }
    if (act === 'to') { f.to = b.dataset.id; return c.render(); }
    if (act !== 'send') return;
    const to = transferTargets(S.caps, S.sittings, round.id).map(x => x.round).find(r => r.id === f.to) || null;
    const body = transferBody(S.loc, round, to, f.lines);
    if (body.error) return c.toast(t(body.error));
    b.disabled = true;
    const ok = await send(c, `/staff/orders/${encodeURIComponent(round.id)}/transfer`, body, 'transfer:' + round.id);
    if (ok) { S.moveForm = null; S.view = 'round'; }
    c.render();
  };
}

export function renderMoveSitting(c, sitting) {
  const { t } = c;
  return `
    ${backBar()}
    <h2>${ui.esc(t('moveSitting'))} · ${ui.esc(t('table'))} ${ui.esc(sitting.table || '—')}</h2>
    ${ui.para(k('moveSittingHint'), { hint: true })}
    <form class="row-form" data-form="moveSit">${ui.inputRow({ label: k('moveTable'), placeholder: k('moveTable'),
      attrs: { name: 'table', required: true, maxlength: 24, inputmode: 'text', data: { tour: 'move.table' } },
      action: ui.button({ type: 'submit', variant: 'primary', label: k('move'), attrs: { data: { tour: 'move.submit' } } }) })}</form>`;
}

export function bindMoveSitting(c, root, sitting) {
  const { S, t } = c;
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (b && b.dataset.act === 'back') { S.view = S.roundId ? 'round' : 'sitting'; c.render(); }
  };
  root.onsubmit = async ev => {
    ev.preventDefault();
    const table = ev.target.elements.table.value.trim();
    if (!table) return;
    const btn = ev.target.querySelector('button[type="submit"]');
    ui.setBusy(btn, t('loading'));
    const ok = await send(c, `/staff/sittings/${encodeURIComponent(sitting.sitting_id)}/move`, { location_id: S.loc, table }, 'move:' + sitting.sitting_id);
    if (ok) S.view = 'sitting';
    c.render();
  };
}

/// One write, told to the waiter. True when the server took it or the outbox
/// holds it.
async function send(c, path, body, tag) {
  const { t } = c;
  try {
    const r = await c.write(path, body, tag);
    if (!r.landed) {
      c.toast(t(r.queued ? 'queuedSaved' : r.reason === 'full' ? 'queueFull' : 'queueNoStore'));
      return r.queued;
    }
    c.toast(t('moved'));
    await c.reload();
    return true;
  } catch (e) {
    const key = refusalKey(e.status, e.message);
    c.toast(key ? t(key) : e.message || t('error'));
    if (key === 'changedReload' || key === 'notHere') await c.reload();
    return false;
  }
}
