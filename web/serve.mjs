// serve.mjs — zero-dep static server for the kernel-driven field UI.
// Correct `application/wasm` MIME so the browser loads the kernel wasm.
// Serves repo root; maps /pkg-web/* -> kernel/pkg-web/* (no wasm duplication).
import { createServer } from 'http';
import { readFile, stat } from 'fs/promises';
import { join, extname, normalize } from 'path';
import { fileURLToPath } from 'url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..'); // web/
const KERNEL_PKG = join(ROOT, '..', 'kernel', 'pkg-web');
const PORT = process.env.PORT || 4173;
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};

const server = createServer(async (req, res) => {
  try {
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
