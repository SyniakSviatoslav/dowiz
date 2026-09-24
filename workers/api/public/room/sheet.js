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
import { money, actionsFor, owed, REASONS, reasonWord, canTransfer, transferTargets, canMoveSitting } from './logic.js';
import { renderGuestBar, answerGuest } from './guest.js';
import { ui, k, act, backBar, amt, pickChips } from './parts.js';

const CHANGED = 'changed while you were editing';

/// One line's controls: quantity, remove, comp -- what the caps allow.
function lineActs(can, i, qty) {
  return [
    can.qty ? ui.iconButton({ icon: 'minus', ariaLabel: k('less'), disabled: qty <= 1, attrs: act('qty', { line: i, q: qty - 1 }, 'round.less') })
      + ui.iconButton({ icon: 'plus', ariaLabel: k('more'), attrs: act('qty', { line: i, q: qty + 1 }, 'round.more') }) : '',
    can.remove ? ui.button({ icon: 'trash', label: k('remove'), attrs: act('ask', { op: 'remove', line: i }, 'round.remove') }) : '',
    can.comp ? ui.button({ icon: 'sparkles', label: k('comp'), attrs: act('ask', { op: 'comp', line: i }, 'round.comp') }) : '',
  ].join('');
}

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
    const acts = comped ? ui.badge({ tone: 'success', label: k('comped') }) : lineActs(can, i, qty);
    const asking = S.ask && S.ask.line === i ? reasonPanel(c) : '';
    return `<li class="line${comped ? ' is-comped' : ''}" data-tour="round.line">
      <div class="line-main"><span class="qty">${qty}×</span><span class="name">${ui.esc(it.name || it.product_id || '')}</span>${amt(money(amount, cur, loc), { cls: 'amt' })}</div>
      ${acts ? `<div class="line-acts">${acts}</div>` : ''}${asking}</li>`;
  }).join('');
  const due = owed(round);
  const sitting = c.sitting();
  return `
    ${backBar(ui.status({ status: round.status, label: c.statusWord(round.status) }))}
    <h2>${ui.esc(t('table'))} ${ui.esc(c.tableOf(round) || '—')}</h2>
    <ul class="lines">${lines || `<li>${ui.emptyState({ icon: 'receipt', title: k('noLines') })}</li>`}</ul>
    <dl class="sums" data-tour="round.sums">
      <dt>${ui.esc(t('subtotal'))}</dt><dd>${amt(money(round.subtotal || 0, cur, loc))}</dd>
      ${round.discount ? `<dt>${ui.esc(t('discount'))}</dt><dd>${amt(money(round.discount, cur, loc), { sign: '-' })}</dd>` : ''}
      <dt>${ui.esc(t('total'))}</dt><dd>${amt(money(round.total || 0, cur, loc))}</dd>
      <dt>${ui.esc(t('owed'))}</dt><dd class="owed">${round.payment_status === 'paid' ? ui.badge({ tone: 'success', icon: 'circle-check', label: k('paidInFull') }) : amt(money(due, cur, loc), { size: 'lg', strong: true })}</dd>
    </dl>
    ${renderGuestBar(c, round)}
    <div class="acts">
      ${can.add ? ui.button({ variant: can.pay ? 'secondary' : 'primary', size: 'lg', block: true, icon: 'plus', label: k('addItem'), attrs: act('add', {}, 'round.add') }) : ''}
      ${can.pay ? ui.button({ variant: 'primary', size: 'lg', block: true, icon: 'cash', label: k('take'), attrs: act('pay', {}, 'round.pay') }) : ''}
      ${items.length > 1 && canTransfer(S.caps, round) && transferTargets(S.caps, S.sittings, round.id).length ? ui.button({ block: true, icon: 'arrows-sort', label: k('moveLines'), attrs: act('transfer', {}, 'round.transfer') }) : ''}
      ${sitting && canMoveSitting(S.caps, sitting) ? ui.button({ block: true, icon: 'map-pin', label: k('moveSitting'), attrs: act('moveSit', {}, 'round.moveSitting') }) : ''}
    </div>
    ${can.table ? `<form class="row-form" data-form="table">${ui.inputRow({ label: k('moveTable'), placeholder: k('moveTable'),
      attrs: { name: 'table', required: true, maxlength: 24, inputmode: 'text', data: { tour: 'round.tableField' } },
      action: ui.button({ type: 'submit', label: k('move'), attrs: { data: { tour: 'round.tableMove' } } }) })}</form>` : ''}`;
}

/// The closed set of reasons, as chips; `other` opens a text field.
function reasonPanel(c) {
  const { t, S } = c;
  const chips = pickChips('reason', t('reason'), REASONS, S.ask.kind, r => t('reason_' + r), 'round.reason', 'r');
  const other = S.ask.kind === 'other'
    ? `<form class="row-form" data-form="other">${ui.inputRow({ label: k('otherText'), placeholder: k('otherText'),
        attrs: { name: 'text', maxlength: 140, required: true, data: { tour: 'round.reasonText' } },
        action: ui.button({ type: 'submit', variant: 'primary', icon: 'send', label: k('send') }) })}</form>` : '';
  return `<div class="ask" role="group" aria-label="${ui.esc(t('reason'))}"><p>${ui.esc(t(S.ask.op === 'comp' ? 'whyComp' : 'whyRemove'))}</p>${chips}${other}
    ${ui.button({ variant: 'ghost', icon: 'x', label: k('cancel'), attrs: act('unask') })}</div>`;
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
    if (act === 'guestConfirm' || act === 'guestReject') { b.disabled = true; return answerGuest(c, round, act === 'guestConfirm' ? 'confirm' : 'reject'); }
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
