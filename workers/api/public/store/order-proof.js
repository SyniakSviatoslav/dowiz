// THE RECEIPT'S QR ROW (LV-PCD-C): one price certificate per order line, each a QR that opens
// /store/verify.html. The certificate is cut from the menu fragment the storefront already holds
// (shell.js `fromCdn`) -- the same bytes whose prices the venue's manifest roots -- so it is built
// on the phone, offline, and the code is drawn by the same wasm module that checks it (`bw_qr`).
//
// A line whose price is not the published price gets NO code and says so: the receipt is then not
// provable against this menu (a price edited after the order, or a modifier surcharge -- base
// prices only, SPEC-PCD §2). Wiring: track.js calls `receiptProof` under an order (hand-back).
// ASCII quotes only (rule 11).

import { certFor, verifyLink, qrWith, loadReader } from '/store/verify.js';

/// `[{ id, price, cert }]` for an order's `lines` (`{ productId, unitPrice }`); cert null = unprovable.
export async function proofLines(fragment, lines) {
  const out = [];
  for (const l of lines || []) out.push({ id: l.productId, price: l.unitPrice, cert: await certFor(fragment, l.productId, l.unitPrice) });
  return out;
}

/// Paint a QR grid on a canvas, `scale` pixels a module, with the 4-module quiet zone.
export function drawQr(canvas, { side, cells }, scale = 3) {
  const n = side + 8;
  canvas.width = canvas.height = n * scale;
  const g = canvas.getContext('2d');
  g.fillStyle = '#fff';
  g.fillRect(0, 0, n * scale, n * scale);
  g.fillStyle = '#000';
  for (let y = 0; y < side; y++) for (let x = 0; x < side; x++) if (cells[y * side + x]) g.fillRect((x + 4) * scale, (y + 4) * scale, scale, scale);
}

/// Append one row per line under `el`: the dish id and its QR, or a note that it is unprovable.
export async function receiptProof(el, { fragment, lines, slug, origin = location.origin, reader = null, words = {} }) {
  const x = reader || await loadReader().catch(() => null);
  if (!x || !fragment) return 0;
  let drawn = 0;
  for (const p of await proofLines(fragment, lines)) {
    const row = document.createElement('div');
    row.className = 'proof-row';
    const label = document.createElement('span');
    label.textContent = p.cert ? p.id : `${p.id}: ${words.unprovable || 'price not in the published menu'}`;
    row.appendChild(label);
    const grid = p.cert && qrWith(x, verifyLink(origin, slug, p.cert));
    if (grid) {
      const c = document.createElement('canvas');
      c.setAttribute('role', 'img');
      c.setAttribute('aria-label', `${words.check || 'price check'} ${p.id}`);
      drawQr(c, grid);
      row.appendChild(c);
      drawn++;
    }
    el.appendChild(row);
  }
  return drawn;
}
