// THE TABLE SHEET'S HOST (CV1b): rows from room/logic.js rules (pure, table.test.mjs), taps back
// to the room's routes via room/net.js write. Why: crates/dowiz-canvas src/lib.rs `THE HOST FILES`.
// ASCII QUOTES ONLY (rule 11).
import { money, actionsFor, owed, REASONS, reasonWord, canTransfer, transferTargets, transferBody, canMoveSitting, refusalKey,
  guestWaiting, METHODS, TILL_CURRENCIES, parseMinor, minorToInput, ratePpm, convertPpm, maxAmountFor, quoteOf, settles,
  tipMinor, walletOk, walletTipOk, walletField, parseCaps, slugOfHost } from '../logic.js';
import { W, clean } from './feed.js';

export const A = { back: 1, qty: 2, ask: 3, reason: 4, unask: 5, other: 6, guestOk: 7, guestNo: 8, add: 9, pay: 10, move: 11,
  table: 12, moveSit: 13, cur: 20, method: 21, fill: 22, take: 23, line: 30, to: 31, send: 32, pick: 40, unpick: 41, addSend: 42,
  tableGo: 50, moveSitGo: 51 };
export const F = { text: 4, rate: 4, amount: 5, tip: 6, wallet: 7 };
export const word = (p, v) => p + String(v).replace(/(^|_)(.)/g, (m, a, b) => b.toUpperCase());
export const methodWord = v => word(v === 'other' ? 'R' : 'M', v);

const R = (...f) => f.map(clean).join('\t');
const btn = (act, arg, word, text, look, tour, block) => R(block ? 'B' : 'b', A[act], arg, word ? W(word) : '', text, look, tour);

export function rows(T, c) {
  const s = (c.sittings || []).find(x => x.sitting_id === T.id);
  if (!s) return '';
  const cur = c.currency, m = a => money(a, cur, c.locale);
  const round = (s.rounds || []).find(r => r.id === T.round) || null;
  const view = round || T.view === 'moveSit' ? T.view : 'sit';
  const out = [R('H', A.back, W('KTable'), s.table || '-', view)];
  if (view === 'pay') pay(out, T, c, round, m);
  else if (view === 'move') move(out, T, c, round, m);
  else if (view === 'add') add(out, T, c, m);
  else if (view === 'table' || view === 'moveSit') {
    if (view === 'moveSit') out.push(R('p', 0, W('MoveSittingHint'), ''));
    out.push(R('f', F.text, 't', W('MoveTable'), view === 'table' ? 'round.tableField' : 'move.table'),
      btn(view === 'table' ? 'tableGo' : 'moveSitGo', '', 'Move', '', 'p', view === 'table' ? 'round.tableMove' : 'move.submit', 1));
  } else {
    (s.rounds || []).forEach((r, i) => sheet(out, T, c, r, i, m));
    if (canMoveSitting(c.caps, s)) out.push(btn('moveSit', '', 'MoveSitting', '', 'n', 'sitting.move', 1));
  }
  return out.join('\n') + '\n';
}

function sheet(out, T, c, r, i, m) {
  const can = actionsFor(c.caps, r), items = Array.isArray(r.items) ? r.items : [];
  out.push(R('-'), R('h', '', `#${i + 1}`, r.status));
  if (guestWaiting(r) && c.caps.has('take_orders')) {
    out.push(R('p', 1, W('GuestRound'), ''), btn('guestOk', r.id, 'GuestConfirm', '', 'g', 'guest.confirm'), btn('guestNo', r.id, 'StopReject', '', 'd', 'guest.reject'));
  }
  if (!items.length) out.push(R('p', 0, W('NoLines'), ''));
  items.forEach((it, k) => {
    const q = Number(it.quantity || 0), comped = it.comped === true;
    out.push(R('i', `${q}× ${it.name || it.product_id || ''}`, m(Number(it.unit_price || 0) * q), comped ? W('Comped') : ''));
    if (comped) return;
    if (can.qty) {
      if (q > 1) out.push(btn('qty', `${r.id}:${k}:${q - 1}`, '', '−', 'n', 'round.less'));
      out.push(btn('qty', `${r.id}:${k}:${q + 1}`, '', '+', 'n', 'round.more'));
    }
    if (can.remove) out.push(btn('ask', `${r.id}:remove:${k}`, 'Remove', '', 'n', 'round.remove'));
    if (can.comp) out.push(btn('ask', `${r.id}:comp:${k}`, 'Comp', '', 'n', 'round.comp'));
    const a = T.ask;
    if (a && a.round === r.id && a.line === k) {
      out.push(R('p', 0, W(a.op === 'comp' ? 'WhyComp' : 'WhyRemove'), ''));
      for (const x of REASONS) out.push(btn('reason', x, word('R', x), '', a.kind === x ? 's' : 'c', 'round.reason'));
      if (a.kind === 'other') out.push(R('f', F.text, 't', W('OtherText'), 'round.reasonText'), btn('other', '', 'Send', '', 'p', ''));
      out.push(btn('unask', '', 'Close', '', 'n', ''));
    }
  });
  out.push(R('k', W('Subtotal'), m(r.subtotal || 0), 0));
  if (r.discount) out.push(R('k', W('Discount'), '-' + m(r.discount), 0));
  out.push(R('k', W('Total'), m(r.total || 0), 0));
  if (r.payment_status === 'paid') out.push(R('p', 2, W('PaidInFull'), ''));
  else out.push(R('k', W('Owed'), m(owed(r)), 1));
  if (can.add) out.push(btn('add', r.id, 'AddItem', '', can.pay ? 'n' : 'p', 'round.add', 1));
  if (can.pay) out.push(btn('pay', r.id, 'Take', '', 'p', 'round.pay', 1));
  if (items.length > 1 && canTransfer(c.caps, r) && transferTargets(c.caps, c.sittings, r.id).length) out.push(btn('move', r.id, 'MoveLines', '', 'n', 'round.transfer', 1));
  if (can.table) out.push(btn('table', r.id, 'MoveTable', '', 'n', 'round.tableField', 1));
}

function pay(out, T, c, r, m) {
  const p = T.pay, order = c.currency;
  if (!order) return out.push(R('p', 1, W('MenuFailed'), ''));
  if (p.note) out.push(R('p', 2, W('Taken'), p.note));
  if (r.payment_status === 'paid') return out.push(R('p', 2, W('PaidInFull'), ''));
  out.push(R('k', W('Owed'), m(owed(r)), 1), R('h', W('Currency'), ''));
  for (const x of [...new Set([order, ...TILL_CURRENCIES])]) out.push(btn('cur', x, '', x, p.currency === x ? 's' : 'c', 'pay.currency'));
  out.push(R('h', W('Method'), ''));
  for (const x of METHODS) out.push(btn('method', x, methodWord(x), '', p.method === x ? 's' : 'c', 'pay.method'));
  const foreign = p.currency !== order, ppm = foreign ? ratePpm(c.field(F.rate), order, p.currency) : null;
  if (foreign) out.push(R('f', F.rate, 'd', W('Rate'), 'pay.rate'), R('p', 0, '', `1 ${quoteOf(order, p.currency).base} = … ${quoteOf(order, p.currency).quote}`));
  out.push(R('f', F.amount, 'd', W('Amount'), 'pay.amount'), R('f', F.tip, 'd', W('Tip'), 'pay.tip'));
  if (p.method === 'wallet') out.push(R('f', F.wallet, 't', W('WalletCode'), 'pay.wallet'));
  const amount = parseMinor(c.field(F.amount), p.currency);
  if (foreign) {
    out.push(ppm && amount ? R('k', W('OffTheBill'), '≈ ' + m(convertPpm(amount, ppm)), 0) : R('p', 0, W('RateNeeded'), ''));
    if (ppm) out.push(btn('fill', '', 'FillOwed', '', 'n', 'pay.fill'));
  }
  out.push(btn('take', '', 'TakeN', amount ? money(amount, p.currency, c.locale) : '-', 'p', 'pay.submit', 1));
}

function move(out, T, c, r, m) {
  const f = T.move, items = Array.isArray(r.items) ? r.items : [];
  out.push(R('p', 0, W('PickLines'), ''));
  items.forEach((it, k) => out.push(btn('line', k, '', `${Number(it.quantity || 0)}× ${it.name || it.product_id || ''} · ${m(Number(it.unit_price || 0) * Number(it.quantity || 0))}`,
    f.lines.includes(k) ? 's' : 'c', 'transfer.line', 1)));
  out.push(R('h', W('MoveLinesTo'), ''));
  const to = transferTargets(c.caps, c.sittings, r.id);
  if (!to.length) out.push(R('p', 0, W('NoTargets'), ''));
  for (const { sitting, round } of to) out.push(btn('to', round.id, 'KTable', `${round.fulfilment?.table || sitting.table || '-'} · ${m(round.total || 0)}`, f.to === round.id ? 's' : 'c', 'transfer.target', 1));
  if (to.length) out.push(btn('send', '', 'MoveLines', '', 'p', 'transfer.send', 1));
}

function add(out, T, c, m) {
  out.push(R('f', F.text, 't', W('Search'), 'menu.search'));
  if (!c.menu) return out.push(R('p', c.menuErr ? 1 : 0, W(c.menuErr || 'Loading'), ''));
  const q = String(c.field(F.text) || '').trim().toLowerCase();
  let n = 0;
  for (const cat of c.menu) {
    const ps = (cat.products || []).filter(p => !q || String(p.name || '').toLowerCase().includes(q));
    if (!ps.length) continue;
    out.push(R('h', '', cat.name || ''));
    for (const p of ps) {
      const k = T.basket[p.id] || 0;
      if (p.available === false) { out.push(R('i', p.name || '', m(p.price || 0), W('SoldOut'))); continue; }
      out.push(btn('pick', p.id, '', `${k ? k + '× ' : ''}${p.name || ''} · ${m(p.price || 0)}`, k ? 's' : 'c', 'menu.dish', 1));
      if (k) out.push(btn('unpick', p.id, '', '−', 'n', 'menu.less'));
      n++;
    }
  }
  if (!n) out.push(R('p', 0, W('NoMatch'), ''));
  const total = Object.values(T.basket).reduce((a, b) => a + b, 0);
  if (total) out.push(btn('addSend', '', 'AddN', String(total), 'p', 'menu.send', 1));
}


let H = null;   // the host: board.js's { C, write, api, reload, toast, sittings(), loc(), caps(), statusWord?, locale() }
const T = { id: null, view: 'sit', round: null, ask: null, pay: null, move: null, basket: {} };
let menu = null, menuErr = null;

const ctx = () => ({ sittings: H.sittings(), caps: parseCaps(H.caps()), currency: H.currency(), locale: H.locale(), menu, menuErr,
  field: f => H.field(f) });
export function draw() {
  if (!H || !T.id) return;
  const text = rows(T, ctx());
  if (!text) { close(); return; }
  H.C.ex.sheet(H.C.put(text)); H.C.ask();
}
export function open(host, sittingId) {
  H = host; Object.assign(T, { id: sittingId, view: 'sit', round: null, ask: null, pay: null, move: null, basket: {} });
  draw();
  if (!H.currency()) loadMenu();
}
export function close() { T.id = null; if (H) { H.C.ex.sheet(0); H.C.drain(); } }
const go = (view, round = T.round) => { Object.assign(T, { view, round, ask: null }); H.setField(F.text, ''); draw(); };
const round = () => H.sittings().flatMap(s => s.rounds || []).find(r => r.id === T.round) || null;

export function input() { if (T.view === 'add' || T.view === 'pay') draw(); }
export function enter(f) {
  if (T.view === 'pay') return act(A.take, '');
  if (T.view === 'table') return act(A.tableGo, '');
  if (T.view === 'moveSit') return act(A.moveSitGo, '');
  if (T.ask?.kind === 'other' && f === F.text) return act(A.other, '');
}

async function send(path, body, tag, okWord = 'Saved') {
  try {
    const r = await H.write(path, body, tag);
    if (!r.landed) { H.toast(r.queued ? 'QueuedSaved' : r.reason === 'full' ? 'QueueFull' : 'QueueNoStore'); return r.queued ? 'queued' : false; }
    H.toast(okWord); await H.reload(); return r.data || true;
  } catch (e) {
    const key = refusalKey(e.status, e.message);
    H.toast(key ? word('', key) : '', e.message);
    if (key === 'changedReload' || key === 'notHere') await H.reload();
    return false;
  }
}
const amend = (r, ops, reason) => send(`/staff/orders/${encodeURIComponent(r.id)}/amend`,
  { location_id: H.loc(), base_seq: r.seq, ops, ...(reason ? { reason } : {}) }, 'amend:' + r.id);
const byId = id => H.sittings().flatMap(s => s.rounds || []).find(r => r.id === id) || null;

export async function act(code, arg) {
  if (!T.id) return;
  const [rid, x, y] = String(arg).split(':');
  if (code === A.back) return T.view === 'sit' ? close() : go('sit', null);
  if (code === A.qty) { const r = byId(rid); if (r) await amend(r, [{ op: 'set_qty', line: Number(x), qty: Number(y) }]); return draw(); }
  if (code === A.ask) { T.ask = { round: rid, op: x, line: Number(y), kind: null }; H.setField(F.text, ''); return draw(); }
  if (code === A.unask) { T.ask = null; return draw(); }
  if (code === A.reason || code === A.other) {
    const a = T.ask; if (!a) return;
    if (code === A.reason) { a.kind = arg; if (arg === 'other') return draw(); }
    const why = reasonWord(a.kind, H.field(F.text));
    if (!why) return H.toast('NeedReason');
    T.ask = null; const r = byId(a.round);
    if (r) await amend(r, [{ op: a.op, line: a.line }], why);
    return draw();
  }
  if (code === A.guestOk || code === A.guestNo) {
    const ok = code === A.guestOk;
    await send(`/staff/orders/${encodeURIComponent(arg)}/guest`, { location_id: H.loc(), action: ok ? 'confirm' : 'reject' }, 'guest:' + arg, ok ? 'GuestConfirmed' : 'GuestRejected');
    return draw();
  }
  if (code === A.add) { T.basket = {}; go('add', arg); return loadMenu(); }
  if (code === A.pay) {
    const r = byId(arg); if (!r) return;
    T.pay = { currency: H.currency(), method: 'cash', note: '' };
    for (const f of [F.rate, F.tip, F.wallet]) H.setField(f, '');
    H.setField(F.amount, minorToInput(owed(r), H.currency()));
    Object.assign(T, { view: 'pay', round: arg, ask: null }); return draw();
  }
  if (code === A.move) { T.move = { lines: [], to: null }; return go('move', arg); }
  if (code === A.table) return go('table', arg);
  if (code === A.moveSit) return go('moveSit', null);
  const r = round();
  if (code === A.cur && T.pay) { T.pay.currency = arg; H.setField(F.amount, arg === H.currency() && r ? minorToInput(owed(r), arg) : ''); return draw(); }
  if (code === A.method && T.pay) { T.pay.method = arg; return draw(); }
  if (code === A.fill && T.pay && r) {
    const ppm = ratePpm(H.field(F.rate), H.currency(), T.pay.currency);
    if (ppm) H.setField(F.amount, minorToInput(maxAmountFor(owed(r), ppm), T.pay.currency));
    return draw();
  }
  if (code === A.take && T.pay && r) return take(r);
  if (code === A.line && T.move) { const i = Number(arg); T.move.lines = T.move.lines.includes(i) ? T.move.lines.filter(v => v !== i) : [...T.move.lines, i]; return draw(); }
  if (code === A.to && T.move) { T.move.to = arg; return draw(); }
  if (code === A.send && T.move && r) {
    const to = transferTargets(parseCaps(H.caps()), H.sittings(), r.id).map(v => v.round).find(v => v.id === T.move.to) || null;
    const body = transferBody(H.loc(), r, to, T.move.lines);
    if (body.error) return H.toast(word('', body.error));
    if (await send(`/staff/orders/${encodeURIComponent(r.id)}/transfer`, body, 'transfer:' + r.id, 'Moved')) return go('sit', null);
    return draw();
  }
  if (code === A.pick || code === A.unpick) {
    const n = (T.basket[arg] || 0) + (code === A.pick ? 1 : -1);
    if (n > 0) T.basket[arg] = n; else delete T.basket[arg];
    return draw();
  }
  if (code === A.addSend && r) {
    const ops = Object.entries(T.basket).map(([product_id, quantity]) => ({ op: 'add', product_id, modifier_ids: [], quantity }));
    if (ops.length && await amend(r, ops)) { T.basket = {}; return go('sit', null); }
    return draw();
  }
  if (code === A.tableGo || code === A.moveSitGo) {
    const table = String(H.field(F.text) || '').trim();
    if (!table) return;
    const ok = code === A.tableGo ? r && await amend(r, [{ op: 'table', table }])
      : await send(`/staff/sittings/${encodeURIComponent(T.id)}/move`, { location_id: H.loc(), table }, 'move:' + T.id, 'Moved');
    if (ok) return go('sit', null);
    return draw();
  }
}

async function take(r) {
  const p = T.pay, order = H.currency();
  const amount = parseMinor(H.field(F.amount), p.currency);
  if (!amount || amount < 1) return H.toast('BadAmount');
  const tip = tipMinor(H.field(F.tip), order);
  if (tip == null) return H.toast('BadTip');
  if (!walletOk(p.method, H.field(F.wallet))) return H.toast('NeedWallet');
  if (!walletTipOk(p.method, tip)) return H.toast('WalletNoTip');
  const body = { location_id: H.loc(), amount, method: p.method };
  if (Number.isInteger(r.seq)) body.base_seq = r.seq;
  if (tip > 0) body.tip = tip;
  if (p.method === 'wallet') Object.assign(body, walletField(H.field(F.wallet)));
  if (p.currency !== order) {
    const ppm = ratePpm(H.field(F.rate), order, p.currency);
    if (!ppm) return H.toast('BadRate');
    Object.assign(body, { currency: p.currency, rate_ppm: ppm });
  }
  const d = await send(`/staff/orders/${encodeURIComponent(r.id)}/pay`, body, 'pay:' + r.id, 'Taken');
  if (d === 'queued') return go('sit', null);
  if (d && d !== true) {
    const ps = d.order?.payments, last = Array.isArray(ps) && ps.length ? ps[ps.length - 1] : null;
    p.note = money(amount, p.currency, H.locale()) + (last ? ` → ${money(settles(last), order, H.locale())}` : '');
    H.setField(F.amount, minorToInput(owed(round() || r), order));
  }
  draw();
}

async function loadMenu() {
  if (menu) return;
  const slug = slugOfHost(location.hostname, location.search);
  if (!slug) { menuErr = 'NoSlug'; return draw(); }
  try {
    const d = await H.api(`/public/locations/${encodeURIComponent(slug)}/menu?locale=${encodeURIComponent(H.locale())}`);
    menu = Array.isArray(d?.categories) ? d.categories : []; menuErr = null;
    if (d?.location?.currencyCode) H.remember(d.location.currencyCode);
  } catch (e) { menuErr = e.offline ? 'Offline' : 'MenuFailed'; }
  draw();
}
