// serve.mjs — zero-dependency static server for the kernel-driven field UI.
//
// Serves the REPO ROOT (not web/) so app.mjs's relative import
// `../../kernel/pkg-web/dowiz_kernel.js` resolves to /root/dowiz/kernel/pkg-web/.
// Sets application/wasm so the browser can stream-compile the kernel module.
// No npm dependencies — plain node:http.

import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { join, extname, normalize, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, ".."); // web/.. === repo root
const PORT = Number(process.env.PORT) || 8099;

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".json": "application/json; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".map": "application/json; charset=utf-8",
};

createServer(async (req, res) => {
  try {
    let url = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (url === "/") url = "/web/index.html";
    const p = join(ROOT, normalize(url));
    if (!p.startsWith(ROOT)) { res.writeHead(403).end("forbidden"); return; }
    const s = await stat(p).catch(() => null);
    if (!s || s.isDirectory()) { res.writeHead(404).end("not found: " + url); return; }
    const body = await readFile(p);
    res.writeHead(200, { "content-type": MIME[extname(p)] || "application/octet-stream" });
    res.end(body);
  } catch (e) {
    res.writeHead(500).end(String(e));
  }
}).listen(PORT, () => {
  console.log(`field UI dev server: http://localhost:${PORT}/web/index.html`);
});
