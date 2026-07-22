// web/src/lib/compose/compose.mjs — browser-side composer + SDF rasteriser.
//
// S5 (BLUEPRINT-INTERFACE-ENGINE-WEBGPU-SHADER-SPINE). Composes the customer
// "menu" screen from the REAL Dubin & Sushi vendor menu (the same 9 Chef's Picks
// `engine::compose_ui::menu_fragment` builds server-side) and rasterises the SDF
// scene to an ImageData the existing #cv canvas paints — so the web shell shows
// the composed vendor menu WITHOUT unlocking wgpu (the CPU raster path is the
// verification authority; the GPU is a paint layer over the same field buffer).
//
// Single source of truth: this JS port mirrors `engine/src/{vendor.rs,sdf.rs,
// scene.rs,compose_ui.rs::menu_grid}` — kept in lock-step by the
// `compose_matches_engine_convention` smoke (`web/src/render/compose.smoke.mjs`).
// It does NOT re-implement math; it is the same expressions transcribed. Money
// is integer Lek throughout; no float arithmetic on money.

// ── vendor menu (Chef's Picks — synced with engine::vendor::CHEF_INDICES) ──
// [0,1,4,7,15,20,22,23,49] into the 59-item menu (engine/src/vendor.rs::MENU).
export const CHEF_PICKS = [
  { id: 'item-01', name: 'Sake Futomaki',          price: 900,  drink: false },
  { id: 'item-02', name: 'Ebi Futomaki',           price: 850,  drink: false },
  { id: 'item-05', name: 'Philadelphia Premium',   price: 1400, drink: false },
  { id: 'item-08', name: 'Sesame Sake',             price: 850,  drink: false },
  { id: 'item-16', name: 'California Classic',     price: 950,  drink: false },
  { id: 'item-21', name: 'Sake Sunset',            price: 950,  drink: false },
  { id: 'item-23', name: 'Truffle Sake premium',   price: 1200, drink: false },
  { id: 'item-24', name: 'Hot Ebi',                price: 850,  drink: false },
  { id: 'item-50', name: 'Salmon Bowl',            price: 850,  drink: false },
];

// ── SDF primitives (mirror engine/src/sdf.rs; pure f64 math, no deps) ──
function sdfCircle(px, py, cx, cy, r) {
  const dx = px - cx, dy = py - cy;
  return Math.hypot(dx, dy) - r;
}
function sdfRoundedBox(px, py, bx, by, hx, hy, r) {
  const rr = Math.min(r, hx, hy);
  const qx = Math.abs(px - bx) - (hx - rr);
  const qy = Math.abs(py - by) - (hy - rr);
  const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0));
  const inside = Math.min(Math.max(qx, qy), 0);
  return outside + inside - rr;
}
function sdfLineSegment(px, py, ax, ay, bx, by) {
  const pax = px - ax, pay = py - ay, bax = bx - ax, bay = by - ay;
  const denom = bax * bax + bay * bay;
  let h = denom === 0 ? 0 : (pax * bax + pay * bay) / denom;
  h = Math.max(0, Math.min(1, h));
  const dx = pax - bax * h, dy = pay - bay * h;
  return Math.hypot(dx, dy);
}

// ── Scene ── (mirror engine/src/scene.rs::Scene::sample — union of shapes)
// A shape is one of: { t:'box', bx,by,hx,hy } { t:'rbox', bx,by,hx,hy,r }
// { t:'circ', cx,cy,r } { t:'line', ax,ay,bx,by }. `eval(px,py)` returns the
// signed distance. The composed menu_scene is a List of these.
function evalShape(shape, px, py) {
  switch (shape.t) {
    case 'rbox': return sdfRoundedBox(px, py, shape.bx, shape.by, shape.hx, shape.hy, shape.r);
    case 'box': {
      const qx = Math.abs(px - shape.bx) - shape.hx;
      const qy = Math.abs(py - shape.by) - shape.hy;
      return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0);
    }
    case 'circ': return sdfCircle(px, py, shape.cx, shape.cy, shape.r);
    case 'line': return sdfLineSegment(px, py, shape.ax, shape.ay, shape.bx, shape.by);
    default: return Infinity;
  }
}
function sceneSample(shapes, px, py) {
  if (shapes.length === 0) return Infinity;
  let d = evalShape(shapes[0], px, py);
  for (let i = 1; i < shapes.length; i++) d = Math.min(d, evalShape(shapes[i], px, py));
  return d;
}

// ── compose the customer menu screen (mirror compose_ui::menu_grid) ──
// center (0,0); cols=4; pitch x=2.4 y=1.7; card 1.0×0.7 r=0.18. A price strip
// (line) under priced cards; the cart badge (circ) when cartCount>0 at (3.2,1.6).
export function composeMenuScene(cartCount = 0) {
  const cols = 4;
  const pitchX = 2.4, pitchY = 1.7, hw = 1.0, hh = 0.7, r = 0.18;
  const shapes = [];
  CHEF_PICKS.forEach((item, k) => {
    const col = k % cols, row = Math.floor(k / cols);
    const cx = -(cols) * pitchX * 0.5 + pitchX * 0.5 + col * pitchX;
    const cy = 1.0 - row * pitchY;
    shapes.push({ t: 'rbox', bx: cx, by: cy, hx: hw, hy: hh, r });
    if (!item.drink) {
      shapes.push({ t: 'line', ax: cx - hw * 0.6, ay: cy - hh - 0.05, bx: cx + hw * 0.6, by: cy - hh - 0.05 });
    }
  });
  if (cartCount > 0) shapes.push({ t: 'circ', cx: 3.2, cy: 1.6, r: 0.35 });
  return shapes;
}

// ── rasterise the SDF scene to a Float32Array field buffer (mirror Scene::render_frame) ──
// scale = world-units per pixel. Returns { width, height, data: Float32Array }.
export function renderFrame(shapes, width, height, scale = 0.25) {
  const x0 = -(width) * 0.5 * scale;
  const y0 = -(height) * 0.5 * scale;
  const data = new Float32Array(width * height);
  for (let row = 0; row < height; row++) {
    const wy = y0 + row * scale;
    for (let col = 0; col < width; col++) {
      const wx = x0 + col * scale;
      const d = sceneSample(shapes, wx, wy);
      data[row * width + col] = Number.isFinite(d) ? d : 3.4e38; // +inf sentinel
    }
  }
  return { width, height, data };
}

// ── paint the field buffer onto a canvas 2D context, tinted by role ──
// `role`: 0=customer (gold), 1=owner (cooler gold), 2=courier (amber). Inside
// the union (d<0) → brand tint (lerp gold→gold-light by depth); outside → bg.
export function paintField(ctx, { width, height, data }, role = 0) {
  const img = ctx.createImageData(width, height);
  const bg = [7, 20, 28];
  const gold = [212, 175, 55];
  const hi = role === 2 ? [255, 153, 46] : role === 1 ? [180, 165, 90] : [241, 213, 138];
  for (let i = 0; i < width * height; i++) {
    const d = data[i];
    let r, g, b;
    if (d < 0) {
      const depth = Math.max(0, Math.min(1, -d * 2));
      r = Math.round(gold[0] + (hi[0] - gold[0]) * depth);
      g = Math.round(gold[1] + (hi[1] - gold[1]) * depth);
      b = Math.round(gold[2] + (hi[2] - gold[2]) * depth);
    } else {
      const fade = 1 / (1 + d * 0.5);
      r = Math.round(bg[0] + (16 - bg[0]) * fade);
      g = Math.round(bg[1] + (28 - bg[1]) * fade);
      b = Math.round(bg[2] + (41 - bg[2]) * fade);
    }
    img.data[i * 4 + 0] = r;
    img.data[i * 4 + 1] = g;
    img.data[i * 4 + 2] = b;
    img.data[i * 4 + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
}

// ── integer cart total (mirror engine::vendor::cart_total — no float money) ──
// ask-drinks (price 0) are excluded. Pure Σ (price × qty) into a Number — safe up
// to 2^53 (the realistic ceiling is 5250 lek × u32 max ≈ 2.25e13, well under).
export function cartTotal(items) {
  let total = 0;
  for (const { item, qty } of items) {
    if (item.drink) continue;
    total += item.price * qty;
  }
  return total;
}