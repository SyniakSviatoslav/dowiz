import { getMockResponse } from './mockData.js';

const PROXIED_PREFIXES = ['/api/', '/public/', '/auth/'];
const DELAY_MS = 200;

let _active = false;

export function enableDevApiMock() {
  if (_active) return;
  _active = true;
  const _orig = window.fetch;

  window.fetch = async function (input: RequestInfo | URL, init?: RequestInit) {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
    const isApi = PROXIED_PREFIXES.some(p => url.startsWith(p) || url.includes(p));
    if (!isApi) return _orig.call(window, input, init);

    const method = (init?.method || 'GET').toUpperCase();
    const body = init?.body ? JSON.parse(init.body as string) : undefined;
    const mock = getMockResponse(method, url, body);

    if (mock) {
      await new Promise(r => setTimeout(r, DELAY_MS));
      return new Response(JSON.stringify(mock.data), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }

    try {
      return await _orig.call(window, input, init);
    } catch {
      return new Response(JSON.stringify({ error: 'Backend unavailable' }), {
        status: 503,
        headers: { 'Content-Type': 'application/json' },
      });
    }
  } as typeof window.fetch;
}
