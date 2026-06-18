import { getMockResponse } from './mockData.js';

const PROXIED_PREFIXES = ['/api/', '/public/', '/auth/', '/v1/'];
const DELAY_MS = 150;

const paramDev = new URLSearchParams(window.location.search).get('dev');
const isDev = paramDev === 'true' || paramDev === '1' || (paramDev !== null && paramDev.length > 2);

function extractPath(url: string): string {
  try {
    // Most fetches are relative ("/api/...", "/v1/rates"); new URL needs a base
    // for those or it throws "Invalid URL" on every call (dev console spam).
    const base = typeof window !== 'undefined' ? window.location.origin : 'http://localhost';
    const u = new URL(url, base);
    return u.pathname.replace(/^\/api/, '') || '/';
  } catch (err) {
    console.debug('[devBootstrap] extractPath URL parse failed:', err);
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
      try { if (init?.body) body = JSON.parse(init.body as string); } catch (err) {
        console.debug('[devBootstrap] request body is not JSON:', err);
      }
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
    } catch (err) {
      console.debug('[devBootstrap] fetch failed:', err);
      return new Response('{}', {
        status: 503,
        headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
      });
    }
  } as typeof window.fetch;
}
