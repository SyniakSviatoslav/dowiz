// THE FLOOR (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.7): the owner's plan with
// every table in its state, and the one write -- a dirty table was cleared.
//
// THE SERVER DECIDES EVERYTHING. `GET /api/staff/floor` loads the plan, the
// booking holds and the live sittings itself and answers each table's state
// (`command::floor::state_of`); this file only draws that answer. The phone
// never computes a state and never sends one: `POST
// /api/staff/floor/:sitting/cleared` carries no state, and the server refuses
// it unless the table is dirty THERE, now.
//
// PURE RENDER (`renderFloor`), so the drawing is provable in node; the wire
// (`loadFloor`, `bindFloor`) goes through the app's `c.api` / `c.write`.
// No `style=` anywhere: the room's CSP drops it (room.css header), so every
// colour is a class in room.css.
import { esc, canClear, FLOOR_STATES } from './logic.js';

export const floorPath = loc => `/staff/floor?location_id=${encodeURIComponent(loc)}`;
export const clearedPath = sid => `/staff/floor/${encodeURIComponent(sid)}/cleared`;

const num = v => (Number.isFinite(Number(v)) ? Number(v) : 0);
const stateOf = s => (FLOOR_STATES.includes(s) ? s : 'free');

/// One table: its shape at the plan's own coordinates (x, y are the CENTRE,
/// `tables.rs` refuses a box that leaves the room), its number, its state.
/// Only a dirty table the signer may clear is a button.
function table(tb, t, caps, pick) {
  const st = stateOf(tb.state);
  const [x, y, w, h] = [num(tb.x), num(tb.y), num(tb.w), num(tb.h)];
  const tap = canClear(caps, st) && !!tb.sitting_id;
  const label = `${t('floor_table')} ${num(tb.n)}: ${t('floor_state_' + st)}`;
  const shape = tb.shape === 'circle'
    ? `<ellipse cx="${x}" cy="${y}" rx="${w / 2}" ry="${h / 2}"></ellipse>`
    : `<rect x="${x - w / 2}" y="${y - h / 2}" width="${w}" height="${h}" rx="6"></rect>`;
  const act = tap ? ` role="button" tabindex="0" data-act="pickTable" data-id="${esc(tb.sitting_id)}"` : ' role="img"';
  const picked = tap && pick === tb.sitting_id ? ' picked' : '';
  return `<g class="ft fs-${st}${tap ? ' tap' : ''}${picked}"${act} aria-label="${esc(label)}"><title>${esc(label)}</title>${shape}` +
    `<text x="${x}" y="${y}" text-anchor="middle" dominant-baseline="central">${num(tb.n)}</text></g>`;
}

/// The whole view. `fl` is the GET answer (null while loading), `pick` the
/// sitting a waiter tapped, which gets the one "cleared" button.
export function renderFloor(fl, t, caps, pick = null) {
  const bar = `<div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button><span class="sp"></span>
      <button class="btn" data-act="floorRefresh">${esc(t('refresh'))}</button></div>
    <h2>${esc(t('floor'))}</h2>`;
  if (!fl) return `${bar}<p class="muted">${esc(t('loading'))}</p>`;
  const W = num(fl.planW) || 390, H = num(fl.planH) || 446;
  const zones = (fl.zones || []).filter(z => (z.tables || []).length);
  const drawn = zones.map(z => `<section class="floor-zone"><h3>${esc(z.name || z.id)}</h3>
      <svg class="floor-plan" viewBox="0 0 ${W} ${H}" role="group" aria-label="${esc(z.name || z.id)}">
      ${(z.tables || []).map(tb => table(tb, t, caps, pick)).join('')}</svg></section>`).join('');
  const off = (fl.unplaced || []).map(u => {
    const st = stateOf(u.state);
    const tap = canClear(caps, st) && !!u.sitting_id;
    return `<li><button class="card fs-${st}${tap ? ' tap' : ''}" ${tap ? `data-act="pickTable" data-id="${esc(u.sitting_id)}"` : 'disabled'}>
      <span class="tbl">${esc(t('floor_table'))} ${esc(u.table || '—')}</span><span class="swatch" aria-hidden="true"></span>
      <span class="meta">${esc(t('floor_state_' + st))}</span></button></li>`;
  }).join('');
  const legend = FLOOR_STATES.map(s => `<li class="fs-${s}"><span class="swatch" aria-hidden="true"></span>${esc(t('floor_state_' + s))}</li>`).join('');
  const all = zones.flatMap(z => z.tables || []).concat(fl.unplaced || []);
  const chosen = pick && all.some(x => x.sitting_id === pick && canClear(caps, stateOf(x.state)));
  return `${bar}
    ${fl.plan_unreadable ? `<p class="muted">${esc(t('floor_noPlan'))}</p>` : ''}
    ${drawn || `<p class="muted">${esc(t('floor_noPlan'))}</p>`}
    ${off ? `<h3>${esc(t('floor_unplaced'))}</h3><ul class="tables">${off}</ul>` : ''}
    ${chosen ? `<div class="card-form floor-clear"><p>${esc(t('floor_clearHint'))}</p>
      <button class="cta" data-act="tableCleared" data-id="${esc(pick)}">${esc(t('floor_clear'))}</button></div>` : ''}
    <h3>${esc(t('floor_legend'))}</h3><ul class="floor-legend">${legend}</ul>`;
}

/// Read the floor into `S.floor`. A failure keeps the last answer and says so.
export async function loadFloor(c) {
  try {
    c.S.floor = await c.api(floorPath(c.S.loc));
  } catch (e) {
    c.toast(e.offline ? c.t('offline') : e.message || c.t('error'));
  }
  if (c.S.view === 'floor') c.render();
}

/// The taps. `back` returns to the room.
export function bindFloor(c, root, back) {
  const { S, t } = c;
  const clear = async sid => {
    try {
      const r = await c.write(clearedPath(sid), { location_id: S.loc }, 'floor:' + sid);
      if (!r.landed) { c.toast(t(r.queued ? 'queuedSaved' : r.reason === 'full' ? 'queueFull' : 'queueNoStore')); return; }
      c.toast(t('floor_cleared'));
      S.floorPick = null;
      await loadFloor(c);
    } catch (e) {
      // A refusal is the server's decision (a table no longer dirty is 409):
      // say it, and redraw from the server rather than from this phone.
      c.toast(e.message || t('error'));
      S.floorPick = null;
      await loadFloor(c);
    }
  };
  const act = el => {
    const a = el.dataset.act, id = el.dataset.id;
    if (a === 'back') return back();
    if (a === 'floorRefresh') return loadFloor(c);
    if (a === 'pickTable') { S.floorPick = S.floorPick === id ? null : id; return c.render(); }
    if (a === 'tableCleared') { el.disabled = true; return clear(id); }
  };
  root.onclick = ev => { const el = ev.target.closest('[data-act]'); if (el) act(el); };
  root.onkeydown = ev => {
    if (ev.key !== 'Enter' && ev.key !== ' ') return;
    const el = ev.target.closest('g[data-act]');
    if (el) { ev.preventDefault(); act(el); }
  };
}
