// State-machine-faithful MOCK of the /internal acquisition surface — for the demo-builder loop's
// anti-cheat dry-run ONLY (no DB, no AI, no network egress). Mirrors the SHIPPED semantics the loop's
// gates depend on (apps/api/src/modules/acquisition/{route,state-machine,provisioning,claim}.ts):
//   * x-provision-ops-secret header gate → 404 when absent/wrong (fail-closed)
//   * POST /acquisition is idempotent ON CONFLICT, returns the CURRENT state (the resume seam)
//   * extract: SOURCED→ENRICHED (with a menu_draft), OR an exit verdict for designated broken place_ids
//   * mint/spine: REQUIRE state ENRICHED → 409 otherwise
//   * verify: 409 NOT_VERIFIABLE on an empty menu_draft (the real markVerified gate)
//   * claim/mint: 409 ACTIVE_INVITE_EXISTS on a second mint (the real partial-unique guard)
// A `liar` mode returns a SUCCESS code with a FALSE/missing field (no-fake-green probe).
//
// place_id keying (drives the extract verdict / menu quality, like the sibling loop's mock):
//   *goodmenu*     → ENRICHED with a rich, demo-quality draft (3 cats, 8 items, descriptions)
//   *lowquality*   → LOW_QUALITY exit verdict (H4 below threshold) — never provisionable
//   *menunotfound* → MENU_NOT_FOUND exit verdict
//   *emptymenu*    → ENRICHED but zero items (API verify will 409 NOT_VERIFIABLE)

import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';

const RICH_DRAFT = {
  categories: [
    { name: 'Antipasti', products: [
      { name: 'Bruschetta al Pomodoro', price: 45000, description: 'Grilled sourdough, San Marzano tomato, basil, olive oil.' },
      { name: 'Burrata', price: 62000, description: 'Creamy burrata, rocket, sun-dried tomato.' },
    ] },
    { name: 'Pizza', products: [
      { name: 'Margherita', price: 58000, description: 'Fior di latte, tomato, basil.' },
      { name: 'Diavola', price: 68000, description: 'Spicy salami, mozzarella, chilli.' },
      { name: 'Quattro Formaggi', price: 72000, description: 'Four-cheese, cracked pepper.' },
    ] },
    { name: 'Dolci', products: [
      { name: 'Tiramisu', price: 40000, description: 'Mascarpone, espresso-soaked savoiardi.' },
      { name: 'Panna Cotta', price: 38000, description: 'Vanilla cream, berry coulis.' },
      { name: 'Gelato', price: 30000, description: 'Three scoops, seasonal flavours.' },
    ] },
  ],
};

export function startMock({ secret, liar = null } = {}) {
  const sources = new Map();

  function classify(place_id) {
    if (place_id.includes('menunotfound')) return { state: 'MENU_NOT_FOUND', failure_reason: 'menu not located' };
    if (place_id.includes('lowquality')) return { state: 'LOW_QUALITY', failure_reason: 'H4 below threshold' };
    if (place_id.includes('emptymenu')) return { state: 'ENRICHED', menu_draft: { categories: [] } };
    return { state: 'ENRICHED', menu_draft: RICH_DRAFT };
  }

  const server = createServer((req, res) => {
    const send = (code, obj) => { res.writeHead(code, { 'content-type': 'application/json' }); res.end(JSON.stringify(obj)); };
    if (req.headers['x-provision-ops-secret'] !== secret) return send(404, { error: 'NOT_FOUND' }); // fail-closed

    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let b = {};
      try { b = JSON.parse(body || '{}'); } catch { return send(400, { error: 'VALIDATION_FAILED' }); }
      const url = req.url;
      const byId = (id) => [...sources.values()].find((s) => s.id === id);

      if (url === '/internal/acquisition') {
        let s = sources.get(b.place_id);
        if (!s) { s = { id: randomUUID(), state: 'SOURCED', menu_draft: null, org_id: null, location_id: null, invited: false, place_id: b.place_id }; sources.set(b.place_id, s); }
        return send(201, { id: s.id, state: s.state });
      }
      if (url === '/internal/acquisition/extract') {
        const s = byId(b.acquisition_source_id);
        if (!s) return send(400, { error: 'VALIDATION_FAILED' });
        const verdict = classify(s.place_id);
        s.state = verdict.state;
        if (verdict.menu_draft !== undefined) s.menu_draft = verdict.menu_draft;
        return send(200, { state: s.state, failure_reason: verdict.failure_reason });
      }
      if (url === '/internal/acquisition/provision/mint') {
        const s = byId(b.acquisition_source_id);
        if (!s) return send(400, { error: 'VALIDATION_FAILED' });
        if (s.state !== 'ENRICHED') return send(409, { error: 'NOT_ENRICHED' });
        return send(201, { token: 'prov-' + randomUUID(), expires_at: new Date(Date.now() + 3e5).toISOString() });
      }
      if (url === '/internal/acquisition/provision/spine') {
        const s = byId(b.acquisition_source_id);
        if (!s) return send(400, { error: 'VALIDATION_FAILED' });
        if (s.state !== 'ENRICHED') return send(409, { error: 'BAD_STATE' });
        if (!b.token) return send(409, { error: 'INVALID_TOKEN' });
        s.org_id = randomUUID(); s.location_id = randomUUID(); s.state = 'PROVISIONED'; s.slug = b.slug;
        return send(201, { org_id: s.org_id, location_id: s.location_id });
      }
      if (url === '/internal/acquisition/claim/verify') {
        const s = byId(b.acquisition_source_id);
        if (!s) return send(400, { error: 'VALIDATION_FAILED' });
        if (s.state !== 'PROVISIONED' && s.state !== 'VERIFIED') return send(409, { error: 'NOT_VERIFIABLE' });
        const hasItems = s.menu_draft?.categories?.some((c) => (c.products || []).length > 0);
        if (!hasItems) return send(409, { error: 'NOT_VERIFIABLE' });
        if (liar === 'verify') return send(200, { verified: false }); // LIAR: success status, false field
        s.state = 'VERIFIED';
        return send(200, { verified: true });
      }
      if (url === '/internal/acquisition/claim/mint') {
        const s = byId(b.acquisition_source_id);
        if (!s) return send(400, { error: 'VALIDATION_FAILED' });
        if (s.state !== 'VERIFIED' && s.state !== 'CLAIM_OFFERED') return send(409, { error: 'NOT_OFFERABLE' });
        if (s.invited) return send(409, { error: 'ACTIVE_INVITE_EXISTS' });
        if (liar === 'claim') return send(201, { expires_at: new Date().toISOString() }); // LIAR: 201 but NO token
        s.invited = true; s.state = 'CLAIM_OFFERED';
        return send(201, { token: 'claim-' + randomUUID(), expires_at: new Date(Date.now() + 2.6e8).toISOString(), notice: { subject: 's', body: 'b' } });
      }
      return send(404, { error: 'NOT_FOUND' });
    });
  });

  return new Promise((resolveP) => {
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      resolveP({ baseUrl: `http://127.0.0.1:${port}`, sources, close: () => new Promise((r) => server.close(r)) });
    });
  });
}
