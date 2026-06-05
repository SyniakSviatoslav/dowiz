import { getMockResponse } from './mockData.js';

const PROXIED_PREFIXES = ['/api/', '/public/', '/auth/'];
const DELAY_MS = 150;

const paramDev = new URLSearchParams(window.location.search).get('dev');
const isDev = paramDev === 'true' || paramDev === '1' || (paramDev !== null && paramDev.length > 2);

function extractPath(url: string): string {
  try {
    const u = new URL(url);
    return u.pathname.replace(/^\/api/, '') || '/';
  } catch {
    return url.replace(/^https?:\/\/[^/]+/, '').replace(/^\/api/, '') || '/';
  }
}

if (isDev) {
  sessionStorage.setItem('dos_dev', '1');
  const _orig = window.fetch;

  window.fetch = async function (input: RequestInfo | URL, init?: RequestInit) {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
    const isApi = PROXIED_PREFIXES.some(p => url.includes(p));

    if (isApi) {
      const method = (init?.method || 'GET').toUpperCase();
      let body: unknown;
      try { if (init?.body) body = JSON.parse(init.body as string); } catch { /* ignore */ }
      const path = extractPath(url);
      const mock = getMockResponse(method, path, body);
      if (mock) {
        await new Promise(r => setTimeout(r, DELAY_MS));
        return new Response(JSON.stringify(mock.data), {
          status: 200,
          headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
        });
      }
      return new Response(JSON.stringify({ error: 'No mock for ' + path }), {
        status: 404,
        headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
      });
    }

    try {
      return await _orig.call(window, input, init);
    } catch {
      return new Response('{}', {
        status: 503,
        headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
      });
    }
  } as typeof window.fetch;
}
