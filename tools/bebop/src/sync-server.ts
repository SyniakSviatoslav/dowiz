// Bebop sync server — self-hosted Better Auth node (the optional multi-device sync backend).
//
// Better Auth is the DEFAULT auth for Bebop. The server is OPT-IN: it only starts when the user runs
// `bebop sync` (or sets BEBOP_SYNC=1). Until then Bebop is a single-user local CLI with no auth server
// at all — fully offline, fully native (RESEARCH §1.7). When started, it serves Better Auth's HTTP API
// from the user's own machine/infra. No Supabase, no Fly, no third party.

import http from 'node:http';
import { createBebopAuth, type BebopAuth } from './auth.ts';

export interface SyncServerOptions {
  port?: number;
  host?: string;
  baseURL?: string;
  secret?: string;
  dbFile?: string;
}

export interface SyncServer {
  auth: BebopAuth;
  url: string;
  close: () => Promise<void>;
}

export async function startSyncServer(opts: SyncServerOptions = {}): Promise<SyncServer> {
  const port = opts.port ?? Number(process.env.BEBOP_SYNC_PORT ?? 8787);
  const host = opts.host ?? process.env.BEBOP_SYNC_HOST ?? '127.0.0.1';
  const baseURL = opts.baseURL ?? `http://${host}:${port}`;
  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url ?? '/', baseURL);
      // Better Auth's `handler` is a standard Fetch API handler — it MUST receive a real `Request`,
      // not a fake shape. node 18+ has global Request. This is the bug that caused signup 500s.
      const request = new Request(url.toString(), {
        method: req.method ?? 'GET',
        headers: toHeaders(req.headers),
        body: req.method === 'GET' || req.method === 'HEAD' ? undefined : new Uint8Array(await readBody(req)),
      });
      const webRes = await auth.handler(request);
      res.statusCode = webRes.status;
      webRes.headers.forEach((v, k) => res.setHeader(k, v));
      const buf = Buffer.from(await webRes.arrayBuffer());
      res.end(buf);
    } catch (e: any) {
      res.statusCode = 500;
      res.setHeader('content-type', 'application/json');
      res.end(JSON.stringify({ error: e?.message ?? 'sync server error' }));
    }
  });

  await new Promise<void>((resolve) => server.listen(port, host, resolve));
  const addr = server.address();
  const actualPort = typeof addr === 'object' && addr ? addr.port : port;
  const url = `http://${host}:${actualPort}`;
  // Build the auth AFTER binding so its baseURL/origin matches the real listening address (fixes
  // Better Auth's MISSING_OR_NULL_ORIGIN / INVALID_ORIGIN CSRF check when port is OS-assigned).
  const auth = createBebopAuth({ baseURL: opts.baseURL ?? url, secret: opts.secret, dbFile: opts.dbFile });

  return {
    auth,
    url,
    close: () =>
      new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve()))),
  };
}

function toHeaders(h: http.IncomingHttpHeaders): Headers {
  const out = new Headers();
  for (const [k, v] of Object.entries(h)) {
    if (v == null) continue;
    out.set(k, Array.isArray(v) ? v.join(', ') : String(v));
  }
  return out;
}

function readBody(req: http.IncomingMessage): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    req.on('data', (c) => chunks.push(Buffer.from(c)));
    req.on('end', () => resolve(Buffer.concat(chunks)));
    req.on('error', reject);
  });
}
