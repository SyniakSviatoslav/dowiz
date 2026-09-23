// A ROUND'S SHEET: its lines, their money, and the changes this person may
// make (`POST /api/staff/orders/:id/amend`, handlers.rs).
//
// EVERY EDIT QUOTES `base_seq`, the version on this screen. A line index means
// "the line I was looking at"; the server refuses a stale one with 409 "this
// order changed while you were editing it", and this screen then RELOADS the
// round and says so -- it never resends the edit against lines it has not
// shown the waiter.
//
// What is hidden is what `actionsFor` says the caps do not allow; the server
// still refuses it (`may_void`, the stage rule), and its words are shown.
import { esc, money, actionsFor, owed, REASONS, reasonWord, canTransfer, transferTargets, canMoveSitting } from './logic.js';

const CHANGED = 'changed while you were editing';

/// The sheet's HTML. `c` is the app context (see app.js `ctx`).
export function renderRound(c, round) {
  const { t, S } = c;
  const cur = S.currency, loc = c.locale();
  const can = actionsFor(S.caps, round);
  const items = Array.isArray(round.items) ? round.items : [];
  const lines = items.map((it, i) => {
    const qty = Number(it.quantity || 0);
    const amount = Number(it.unit_price || 0) * qty;
    const comped = it.comped === true;
    const acts = comped ? `<span class="chip">${esc(t('comped'))}</span>` : [
      can.qty ? `<button class="btn sq" data-act="qty" data-line="${i}" data-q="${qty - 1}" aria-label="${esc(t('less'))}" ${qty <= 1 ? 'disabled' : ''}>−</button>
                 <button class="btn sq" data-act="qty" data-line="${i}" data-q="${qty + 1}" aria-label="${esc(t('more'))}">+</button>` : '',
      can.remove ? `<button class="btn" data-act="ask" data-op="remove" data-line="${i}">${esc(t('remove'))}</button>` : '',
      can.comp ? `<button class="btn" data-act="ask" data-op="comp" data-line="${i}">${esc(t('comp'))}</button>` : '',
    ].join('');
    const asking = S.ask && S.ask.line === i ? reasonPanel(c) : '';
    return `<li class="line${comped ? ' is-comped' : ''}">
      <div class="line-main"><span class="qty">${qty}×</span><span class="name">${esc(it.name || it.product_id || '')}</span><span class="amt">${money(amount, cur, loc)}</span></div>
      ${acts ? `<div class="line-acts">${acts}</div>` : ''}${asking}</li>`;
  }).join('');
  const due = owed(round);
  return `
    <div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button>
      <span class="status st-${esc(round.status)}">${esc(c.statusWord(round.status))}</span></div>
    <h2>${esc(t('table'))} ${esc(c.tableOf(round) || '—')}</h2>
    <ul class="lines">${lines || `<li class="muted">${esc(t('noLines'))}</li>`}</ul>
    <dl class="sums">
      <dt>${esc(t('subtotal'))}</dt><dd>${money(round.subtotal || 0, cur, loc)}</dd>
      ${round.discount ? `<dt>${esc(t('discount'))}</dt><dd>−${money(round.discount, cur, loc)}</dd>` : ''}
      <dt>${esc(t('total'))}</dt><dd>${money(round.total || 0, cur, loc)}</dd>
      <dt>${esc(t('owed'))}</dt><dd class="owed">${round.payment_status === 'paid' ? esc(t('paidInFull')) : money(due, cur, loc)}</dd>
    </dl>
    <div class="acts">
      ${can.add ? `<button class="cta" data-act="add">${esc(t('addItem'))}</button>` : ''}
      ${can.pay ? `<button class="cta" data-act="pay">${esc(t('take'))}</button>` : ''}
      ${items.length > 1 && canTransfer(S.caps, round) && transferTargets(S.caps, S.sittings, round.id).length ? `<button class="btn" data-act="transfer">${esc(t('moveLines'))}</button>` : ''}
      ${c.sitting() && canMoveSitting(S.caps, c.sitting()) ? `<button class="btn" data-act="moveSit">${esc(t('moveSitting'))}</button>` : ''}
    </div>
    ${can.table ? `<form class="row-form" data-form="table"><label>${esc(t('moveTable'))}
      <input name="table" required maxlength="24" autocomplete="off" inputmode="text"></label>
      <button class="btn" type="submit">${esc(t('move'))}</button></form>` : ''}`;
}

/// The closed set of reasons, as buttons; `other` opens a text field.
function reasonPanel(c) {
  const { t, S } = c;
  const btns = REASONS.map(r => `<button class="btn${S.ask.kind === r ? ' on' : ''}" data-act="reason" data-r="${r}">${esc(t('reason_' + r))}</button>`).join('');
  const other = S.ask.kind === 'other'
    ? `<form class="row-form" data-form="other"><input name="text" maxlength="140" required placeholder="${esc(t('otherText'))}"><button class="btn" type="submit">${esc(t('send'))}</button></form>` : '';
  return `<div class="ask" role="group" aria-label="${esc(t('reason'))}"><p>${esc(t(S.ask.op === 'comp' ? 'whyComp' : 'whyRemove'))}</p><div class="chips">${btns}</div>${other}
    <button class="btn ghost" data-act="unask">${esc(t('cancel'))}</button></div>`;
}

/// Send one amendment. Answers true when the round moved, 'queued' when the
/// network did not carry it and the outbox holds it, false otherwise.
export async function amend(c, round, ops, reason) {
  const { S, t } = c;
  const body = { location_id: S.loc, base_seq: round.seq, ops };
  if (reason) body.reason = reason;
  try {
    const r = await c.write(`/staff/orders/${encodeURIComponent(round.id)}/amend`, body, 'amend:' + round.id);
    if (r.landed) {
      const o = r.data && r.data.order;
      if (o) Object.assign(round, pick(o), { seq: r.data.seq });
      c.toast(t('saved'));
      c.reload();
      return true;
    }
    c.toast(t(r.queued ? 'queuedSaved' : r.reason === 'full' ? 'queueFull' : 'queueNoStore'));
    return r.queued ? 'queued' : false;
  } catch (e) {
    if (e.status === 409 && String(e.message).includes(CHANGED)) {
      c.toast(t('changedReload'));
      await c.reload();
      return false;
    }
    c.toast(e.message || t('error'));
    return false;
  }
}

/// The fields of an order that a round card carries.
export const pick = o => ({
  status: o.status, items: o.items, subtotal: o.subtotal, discount: o.discount, tip: o.tip,
  total: o.total, payment_status: o.payment_status, payments: o.payments, adjustments: o.adjustments,
  fulfilment: o.fulfilment,
});

/// Wire the sheet's buttons. Called after every draw of the round view.
export function bindRound(c, root, round) {
  const { S } = c;
  root.onclick = async ev => {
    const b = ev.target.closest('[data-act]');
    if (!b || b.disabled) return;
    const act = b.dataset.act, line = Number(b.dataset.line);
    if (act === 'back') { S.view = 'room'; S.ask = null; return c.render(); }
    if (act === 'add') { S.view = 'add'; S.basket = {}; return c.render(); }
    if (act === 'pay') { S.view = 'pay'; S.payNote = null; return c.render(); }
    if (act === 'transfer') { S.view = 'transfer'; S.moveForm = null; return c.render(); }
    if (act === 'moveSit') { S.view = 'moveSit'; return c.render(); }
    if (act === 'qty') { b.disabled = true; await amend(c, round, [{ op: 'set_qty', line, qty: Number(b.dataset.q) }]); return c.render(); }
    if (act === 'ask') { S.ask = { op: b.dataset.op, line, kind: null }; return c.render(); }
    if (act === 'unask') { S.ask = null; return c.render(); }
    if (act === 'reason') {
      S.ask.kind = b.dataset.r;
      if (b.dataset.r === 'other') return c.render();
      return sendAsk(c, round, reasonWord(b.dataset.r));
    }
  };
  root.onsubmit = async ev => {
    ev.preventDefault();
    const f = ev.target, kind = f.dataset.form;
    if (kind === 'other') {
      const w = reasonWord('other', f.elements.text.value);
      if (!w) return c.toast(c.t('needReason'));
      return sendAsk(c, round, w);
    }
    if (kind === 'table') {
      const table = f.elements.table.value.trim();
      if (!table) return;
      await amend(c, round, [{ op: 'table', table }]);
      c.render();
    }
  };
}

async function sendAsk(c, round, reason) {
  const ask = c.S.ask;
  c.S.ask = null;
  await amend(c, round, [{ op: ask.op, line: ask.line }], reason);
  c.render();
}
