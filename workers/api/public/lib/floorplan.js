// The floor editor's rules, PURE: no DOM, no fetch, no clock.
// `node public/lib/floorplan.test.mjs` runs them.
//
// THE HUB DECIDES. `POST /api/owner/floorplan` parses the plan back through
// `dowiz_hub::tables::from_json` and refuses it whole, naming the zone and the
// table. `check` below is only the editor's early word, so the owner sees the
// problem where they are drawing rather than after pressing save. Its limits
// come from the hub (`GET /api/owner/floorplan` answers planW/planH/maxSeats),
// never from a second copy of the numbers.
//
// A table's x/y is its CENTRE, the way the storefront draws it.

/// The grid a table moves on, in plan units. Arrow nudges and drags snap to it.
export const GRID = 10;
/// A new table's footprint and seats: a four-top the owner then adjusts.
export const NEW_TABLE = { w: 44, h: 44, seats: 4, shape: 'rect' };

const lim = (limits = {}) => ({ W: limits.planW || 390, H: limits.planH || 446, S: limits.maxSeats || 20 });

/// A zone id from its name: a-z, 0-9, '-' only (it becomes part of a storage
/// key), unique among `taken`. Cyrillic or Albanian letters fall back to 'zone'.
export function zoneId(name, taken = []) {
  const base = String(name || '').toLowerCase()
    .normalize('NFKD').replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 24) || 'zone';
  let id = base, i = 2;
  while (taken.includes(id)) id = `${base}-${i++}`;
  return id;
}

/// The smallest table number not yet used in this zone.
export function nextNumber(zone) {
  const used = new Set((zone?.tables || []).map(t => t.n));
  let n = 1;
  while (used.has(n)) n++;
  return n;
}

export const snap = (v, step = GRID) => Math.round(v / step) * step;

/// Keep a table's box inside the drawing: the centre is clamped so that
/// centre +/- half the size stays within [0, W] x [0, H].
export function clampTable(t, limits) {
  const { W, H } = lim(limits);
  const hw = Math.floor(t.w / 2), hh = Math.floor(t.h / 2);
  return { ...t, x: Math.min(W - hw, Math.max(hw, t.x)), y: Math.min(H - hh, Math.max(hh, t.y)) };
}

/// Move a table by (dx, dy) grid steps, snapped and kept on the plan.
export function nudge(t, dx, dy, limits, step = GRID) {
  return clampTable({ ...t, x: snap(t.x + dx * step, step), y: snap(t.y + dy * step, step) }, limits);
}

/// Put a table's centre at a point (a drag), snapped and kept on the plan.
export function placeAt(t, x, y, limits, step = GRID) {
  return clampTable({ ...t, x: snap(x, step), y: snap(y, step) }, limits);
}

/// Do two tables' boxes share any area? Edges that only touch do not. A round
/// table is judged by its square box: conservative, never a false "clear".
export function overlaps(a, b) {
  const ax = Math.floor(a.w / 2), ay = Math.floor(a.h / 2), bx = Math.floor(b.w / 2), by = Math.floor(b.h / 2);
  return a.x - ax < b.x + bx && b.x - bx < a.x + ax && a.y - ay < b.y + by && b.y - by < a.y + ay;
}

/// A new table in `zone`: the next free number, at the first grid spot (row
/// by row from the top left) where its box overlaps no other table's box.
///
/// THE DEFECT (2026-09-24): this used to dodge only an exact CENTRE on a
/// 60-unit grid, so a table dragged to (50,50) was silently covered by the
/// next new one at (40,40). A plan with no free spot left gets the first spot
/// anyway, and `check` names the overlap so it cannot be saved unseen.
export function newTable(zone, limits) {
  const { W, H } = lim(limits);
  const others = zone?.tables || [];
  const at = (x, y) => clampTable({ n: nextNumber(zone), x, y, ...NEW_TABLE }, limits);
  for (let y = 40; y <= H - 40; y += GRID) {
    for (let x = 40; x <= W - 40; x += GRID) {
      const t = at(x, y);
      if (!others.some(o => overlaps(o, t))) return t;
    }
  }
  return at(40, 40);
}

/// A round table has w equal to h (the hub refuses one that does not).
export function withShape(t, shape) {
  if (shape !== 'circle') return { ...t, shape: 'rect' };
  const d = Math.max(t.w, t.h);
  return { ...t, shape: 'circle', w: d, h: d };
}

/// The editor's early word, as [{ zone, n, key }] -- `key` is an i18n key.
/// The same rules the hub enforces: a zone has a name and a unique id; in a
/// zone no two tables share a number; seats are 1..maxSeats; a table fits.
/// One rule is the editor's own: in a zone no two tables' boxes overlap.
export function check(zones, limits) {
  const { W, H, S } = lim(limits);
  const out = [], ids = new Set();
  for (const z of zones || []) {
    if (!String(z.name || '').trim()) out.push({ zone: z.id, n: null, key: 'fpNoName' });
    if (ids.has(z.id)) out.push({ zone: z.id, n: null, key: 'fpZoneTwice' });
    ids.add(z.id);
    const ns = new Set();
    for (const t of z.tables || []) {
      if (!Number.isInteger(t.n) || t.n < 1) out.push({ zone: z.id, n: t.n, key: 'fpBadNumber' });
      else if (ns.has(t.n)) out.push({ zone: z.id, n: t.n, key: 'fpNumberTwice' });
      ns.add(t.n);
      if (!Number.isInteger(t.seats) || t.seats < 1 || t.seats > S) out.push({ zone: z.id, n: t.n, key: 'fpSeats' });
      const hw = Math.floor(t.w / 2), hh = Math.floor(t.h / 2);
      if (t.x - hw < 0 || t.y - hh < 0 || t.x + hw > W || t.y + hh > H) out.push({ zone: z.id, n: t.n, key: 'fpOffPlan' });
    }
    // Two tables on one spot of one floor: named once, by the later table.
    const ts = z.tables || [];
    ts.forEach((t, i) => { if (ts.slice(0, i).some(o => overlaps(o, t))) out.push({ zone: z.id, n: t.n, key: 'fpOverlap' }); });
  }
  return out;
}

/// What is sent: only the fields the hub reads, as integers.
export const toWire = zones => (zones || []).map(z => ({
  id: z.id, name: String(z.name || '').trim(),
  tables: (z.tables || []).map(t => ({
    n: t.n | 0, x: t.x | 0, y: t.y | 0, w: t.w | 0, h: t.h | 0, seats: t.seats | 0, shape: t.shape === 'circle' ? 'circle' : 'rect',
  })),
}));
