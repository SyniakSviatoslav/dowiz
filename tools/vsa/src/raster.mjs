// Pure-stdlib rasterizer + PNG encoder — ZERO deps (only node:zlib). The token-arbitrage
// visual layer (VSA-VIZ) needs to turn a system state into a raster a vision model can read;
// Claude vision takes PNG/JPEG/WebP, NOT SVG, so we rasterize. No sharp/resvg/canvas (all
// dep-gated here) — we draw crisp high-contrast geometry onto an RGBA buffer and DEFLATE it
// into a PNG ourselves. ~1-5ms for a 1024² frame. Simple shapes on purpose: vision models
// read clean geometry reliably and hallucinate on pixel noise (the whole design constraint).

import zlib from 'node:zlib';

// ── CRC32 (own table — not every node zlib exposes crc32) ──
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

export class Canvas {
  constructor(w, h, bg = [255, 255, 255, 255]) {
    this.w = w;
    this.h = h;
    this.px = Buffer.alloc(w * h * 4);
    for (let i = 0; i < w * h; i++) this.px.set(bg, i * 4);
  }

  set(x, y, [r, g, b, a = 255]) {
    x = x | 0;
    y = y | 0;
    if (x < 0 || y < 0 || x >= this.w || y >= this.h) return;
    const i = (y * this.w + x) * 4;
    if (a === 255) {
      this.px[i] = r;
      this.px[i + 1] = g;
      this.px[i + 2] = b;
      this.px[i + 3] = 255;
    } else {
      // alpha-over the existing pixel
      const ia = a / 255;
      this.px[i] = r * ia + this.px[i] * (1 - ia);
      this.px[i + 1] = g * ia + this.px[i + 1] * (1 - ia);
      this.px[i + 2] = b * ia + this.px[i + 2] * (1 - ia);
      this.px[i + 3] = 255;
    }
  }

  fillRect(x, y, w, h, color) {
    for (let dy = 0; dy < h; dy++) for (let dx = 0; dx < w; dx++) this.set(x + dx, y + dy, color);
  }

  // Crisp square centered at (cx,cy), side s, with an optional border.
  square(cx, cy, s, color, border = null) {
    const x = Math.round(cx - s / 2);
    const y = Math.round(cy - s / 2);
    this.fillRect(x, y, s, s, color);
    if (border) this.rectOutline(x, y, s, s, border, Math.max(2, Math.round(s / 12)));
  }

  rectOutline(x, y, w, h, color, t = 2) {
    this.fillRect(x, y, w, t, color);
    this.fillRect(x, y + h - t, w, t, color);
    this.fillRect(x, y, t, h, color);
    this.fillRect(x + w - t, y, t, h, color);
  }

  // Filled circle (crisp, tiny AA on the rim for legibility at small sizes).
  circle(cx, cy, r, color, border = null) {
    const r2 = r * r;
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        const d2 = dx * dx + dy * dy;
        if (d2 <= r2) this.set(cx + dx, cy + dy, color);
        else if (d2 <= (r + 1) * (r + 1)) this.set(cx + dx, cy + dy, [...color.slice(0, 3), 110]);
      }
    }
    if (border) {
      const bt = Math.max(2, Math.round(r / 8));
      for (let a = 0; a < 360; a += 2) {
        for (let k = 0; k < bt; k++) {
          const rr = r - k;
          this.set(Math.round(cx + rr * Math.cos((a * Math.PI) / 180)), Math.round(cy + rr * Math.sin((a * Math.PI) / 180)), border);
        }
      }
    }
  }

  // Thick line via perpendicular offset stamping (Bresenham-free; fine at these sizes).
  line(x0, y0, x1, y1, color, thickness = 2) {
    const dx = x1 - x0;
    const dy = y1 - y0;
    const len = Math.max(1, Math.hypot(dx, dy));
    const steps = Math.ceil(len);
    const nx = -dy / len; // unit normal
    const ny = dx / len;
    const half = thickness / 2;
    for (let s = 0; s <= steps; s++) {
      const t = s / steps;
      const x = x0 + dx * t;
      const y = y0 + dy * t;
      for (let o = -half; o <= half; o += 0.6) this.set(Math.round(x + nx * o), Math.round(y + ny * o), color);
    }
  }

  // Filled convex polygon (scanline). Covers triangle (car) + diamond/rhombus (van) — the
  // meso-level vehicle glyphs of the fractal encoding.
  polygon(pts, color, border = null) {
    const ys = pts.map((p) => p[1]);
    const yMin = Math.floor(Math.min(...ys));
    const yMax = Math.ceil(Math.max(...ys));
    for (let y = yMin; y <= yMax; y++) {
      const xs = [];
      for (let i = 0; i < pts.length; i++) {
        const [x1, y1] = pts[i];
        const [x2, y2] = pts[(i + 1) % pts.length];
        if (y1 <= y && y2 > y) xs.push(x1 + ((y - y1) / (y2 - y1)) * (x2 - x1));
        else if (y2 <= y && y1 > y) xs.push(x2 + ((y - y2) / (y1 - y2)) * (x1 - x2));
      }
      xs.sort((a, b) => a - b);
      for (let k = 0; k + 1 < xs.length; k += 2) {
        for (let x = Math.round(xs[k]); x <= Math.round(xs[k + 1]); x++) this.set(x, y, color);
      }
    }
    if (border) {
      const t = typeof border === 'object' && border.t ? border.t : 3;
      const col = border.color ?? border;
      for (let i = 0; i < pts.length; i++) this.line(pts[i][0], pts[i][1], pts[(i + 1) % pts.length][0], pts[(i + 1) % pts.length][1], col, t);
    }
  }

  // Up-triangle centred at (cx,cy), height s. (Car.)
  triangle(cx, cy, s, color, border = null) {
    const h = s / 2;
    this.polygon([[cx, cy - h], [cx - h, cy + h], [cx + h, cy + h]], color, border);
  }

  // Diamond/rhombus centred at (cx,cy), diagonal s. (Van.)
  diamond(cx, cy, s, color, border = null) {
    const h = s / 2;
    this.polygon([[cx, cy - h], [cx + h, cy], [cx, cy + h], [cx - h, cy]], color, border);
  }

  // Micro-matrix: an n×n grid of `cell`-px squares (gap between) at top-left (x,y). `colors[i]`
  // colors cell i in row-major order; `null` = an empty (outlined) slot. The level-3 patch grid.
  matrix(x, y, cell, gap, n, colors, empty = C.bg) {
    for (let i = 0; i < n * n; i++) {
      const cx = x + (i % n) * (cell + gap);
      const cy = y + Math.floor(i / n) * (cell + gap);
      const col = colors[i] ?? null;
      if (col) this.fillRect(cx, cy, cell, cell, col);
      else {
        this.fillRect(cx, cy, cell, cell, empty);
        this.rectOutline(cx, cy, cell, cell, [180, 186, 194, 255], 1);
      }
    }
  }

  // 5x7 bitmap text (scaled). Encodes labels legibly for a vision model.
  text(x, y, str, color, scale = 2) {
    let cx = x;
    for (const ch of String(str).toUpperCase()) {
      const glyph = FONT[ch] ?? FONT['?'];
      for (let row = 0; row < 7; row++) {
        for (let col = 0; col < 5; col++) {
          if ((glyph[row] >> (4 - col)) & 1) this.fillRect(cx + col * scale, y + row * scale, scale, scale, color);
        }
      }
      cx += 6 * scale;
    }
    return cx - x; // advance width
  }

  toPNG() {
    const { w, h } = this;
    // filtered scanlines: each row prefixed by filter byte 0 (none)
    const raw = Buffer.alloc((w * 4 + 1) * h);
    for (let y = 0; y < h; y++) {
      raw[y * (w * 4 + 1)] = 0;
      this.px.copy(raw, y * (w * 4 + 1) + 1, y * w * 4, (y + 1) * w * 4);
    }
    const idat = zlib.deflateSync(raw, { level: 9 });
    const ihdr = Buffer.alloc(13);
    ihdr.writeUInt32BE(w, 0);
    ihdr.writeUInt32BE(h, 4);
    ihdr[8] = 8; // bit depth
    ihdr[9] = 6; // color type RGBA
    const chunk = (type, data) => {
      const len = Buffer.alloc(4);
      len.writeUInt32BE(data.length, 0);
      const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
      const crc = Buffer.alloc(4);
      crc.writeUInt32BE(crc32(td), 0);
      return Buffer.concat([len, td, crc]);
    };
    return Buffer.concat([
      Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
      chunk('IHDR', ihdr),
      chunk('IDAT', idat),
      chunk('IEND', Buffer.alloc(0)),
    ]);
  }
}

// Palette (high-contrast, colorblind-tolerant).
export const C = {
  bg: [248, 249, 251, 255],
  ink: [24, 28, 34, 255],
  muted: [120, 128, 138, 255],
  green: [22, 163, 74, 255],
  amber: [217, 119, 6, 255],
  red: [220, 38, 38, 255],
  blue: [37, 99, 235, 255],
  gray: [156, 163, 175, 255],
  line: [90, 100, 112, 255],
  white: [255, 255, 255, 255],
};

// Compact 5x7 font (rows are 5-bit masks). Enough for labels: 0-9 A-Z : % . # / - space.
const FONT = {
  ' ': [0, 0, 0, 0, 0, 0, 0],
  '0': [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
  '1': [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
  '2': [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
  '3': [0x1f, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0e],
  '4': [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
  '5': [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e],
  '6': [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e],
  '7': [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
  '8': [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
  '9': [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c],
  A: [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
  B: [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
  C: [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e],
  D: [0x1c, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1c],
  E: [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
  F: [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
  G: [0x0e, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0f],
  H: [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
  I: [0x0e, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
  J: [0x07, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0c],
  K: [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
  L: [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
  M: [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
  N: [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
  O: [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
  P: [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
  Q: [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
  R: [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
  S: [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
  T: [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
  U: [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
  V: [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
  W: [0x11, 0x11, 0x11, 0x15, 0x15, 0x1b, 0x11],
  X: [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
  Y: [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
  Z: [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
  ':': [0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00],
  '%': [0x19, 0x19, 0x02, 0x04, 0x08, 0x13, 0x13],
  '.': [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
  '#': [0x0a, 0x1f, 0x0a, 0x0a, 0x1f, 0x0a, 0x00],
  '/': [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
  '-': [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
  '?': [0x0e, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
};
