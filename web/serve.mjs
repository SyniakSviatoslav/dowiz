// serve.mjs — zero-dep static server + kernel-driven API endpoints.
// Serves repo root; maps /pkg-web/* -> kernel/pkg-web/*.
// Exposes /api/order POST for the checkout flow (Stage A).

import { createServer } from 'http';
import { readFile, stat, writeFile } from 'fs/promises';
import { join, extname, normalize } from 'path';
import { fileURLToPath } from 'url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..');
const KERNEL_PKG = join(ROOT, '..', 'kernel', 'pkg-web');
const PORT = process.env.PORT || 8099;

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};

// In-memory order store (Stage A — no persistence)
const orders = [];
let nextId = 1100;

function parseBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', c => chunks.push(c));
    req.on('end', () => {
      try { resolve(JSON.parse(Buffer.concat(chunks).toString())); }
      catch { reject(new Error('invalid JSON')); }
    });
    req.on('error', reject);
  });
}

function apiRoutes(req, res) {
  if (req.method === 'POST' && req.url === '/api/order') {
    return parseBody(req).then(body => {
      const order = { id: nextId++, status: 'pending', items: body.items || [], total: body.total || 0, createdAt: Date.now() };
      orders.unshift(order);
      res.writeHead(201, { 'Content-Type': 'application/json; charset=utf-8' });
      res.end(JSON.stringify(order));
    }).catch(() => {
      res.writeHead(400, { 'Content-Type': 'application/json; charset=utf-8' });
      res.end(JSON.stringify({ ok: false, error: 'invalid JSON body' }));
    });
  }
  if (req.method === 'GET' && req.url === '/api/orders') {
    res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
    res.end(JSON.stringify(orders));
    return Promise.resolve();
  }
  return Promise.resolve(false);
}

const server = createServer(async (req, res) => {
  try {
    if (await apiRoutes(req, res, req.method, req.url) !== false) return;
    let urlPath = decodeURIComponent((req.url || '/').split('?')[0]);
    let filePath;
    if (urlPath.startsWith('/pkg-web/')) {
      filePath = join(KERNEL_PKG, normalize(urlPath.slice('/pkg-web/'.length)));
    } else if (urlPath === '/' || urlPath === '') {
      filePath = join(ROOT, 'index.html');
    } else {
      filePath = join(ROOT, normalize(urlPath));
    }
    const info = await stat(filePath).catch(() => null);
    if (!info || !info.isFile()) { res.writeHead(404); res.end('not found'); return; }
    const body = await readFile(filePath);
    res.writeHead(200, { 'Content-Type': MIME[extname(filePath)] || 'application/octet-stream' });
    res.end(body);
  } catch (e) {
    res.writeHead(500); res.end(String(e));
  }
});

server.listen(PORT, () => {
  console.log(`dowiz field-UI serving on http://localhost:${PORT} (kernel wasm at /pkg-web/)`);
});
