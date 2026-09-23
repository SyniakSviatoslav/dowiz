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
import { esc, money, transferTargets, transferBody, refusalKey } from './logic.js';

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
  const lines = items.map((it, i) => {
    const on = f.lines.includes(i);
    return `<li><button type="button" class="line pick${on ? ' on' : ''}" role="checkbox" aria-checked="${on}" data-act="line" data-line="${i}">
      <span class="qty">${Number(it.quantity || 0)}×</span><span class="name">${esc(it.name || it.product_id || '')}</span>
      <span class="amt">${money(Number(it.unit_price || 0) * Number(it.quantity || 0), cur, loc)}</span></button></li>`;
  }).join('');
  const dests = targets.map(({ sitting, round: r }) => `<button type="button" class="seg-b${f.to === r.id ? ' on' : ''}" role="radio" aria-checked="${f.to === r.id}" data-act="to" data-id="${esc(r.id)}">
      ${esc(t('table'))} ${esc(r.fulfilment?.table || sitting.table || '—')} · ${money(r.total || 0, cur, loc)}</button>`).join('');
  return `
    <div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div>
    <h2>${esc(t('moveLines'))} · ${esc(t('table'))} ${esc(c.tableOf(round) || '—')}</h2>
    <p class="muted">${esc(t('pickLines'))}</p>
    <ul class="lines">${lines || `<li class="muted">${esc(t('noLines'))}</li>`}</ul>
    <label>${esc(t('moveLinesTo'))}</label>
    ${dests ? `<div class="seg" role="radiogroup" aria-label="${esc(t('moveLinesTo'))}">${dests}</div>` : `<p class="muted">${esc(t('noTargets'))}</p>`}
    <div class="acts"><button class="cta" data-act="send"${dests ? '' : ' disabled'}>${esc(t('moveLines'))}</button></div>`;
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
    <div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div>
    <h2>${esc(t('moveSitting'))} · ${esc(t('table'))} ${esc(sitting.table || '—')}</h2>
    <p class="muted">${esc(t('moveSittingHint'))}</p>
    <form class="row-form" data-form="moveSit"><label>${esc(t('moveTable'))}
      <input name="table" required maxlength="24" autocomplete="off" inputmode="text"></label>
      <button class="btn" type="submit">${esc(t('move'))}</button></form>`;
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
    if (btn) btn.disabled = true;
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
