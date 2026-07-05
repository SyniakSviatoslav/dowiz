#!/usr/bin/env node
// P6 acquisition bulk-provisioning loop — operator tooling.
//
// Takes a list/CSV of restaurants and runs each through the SHIPPED /internal acquisition
// pipeline (apps/api/src/modules/acquisition/route.ts) to produce a claimable SHADOW tenant
// + a claim invite, at scale. Loop-shaped, not a one-shot:
//   * idempotent + resumable (reads each source's CURRENT state first, jumps to the right stage)
//   * a verification GATE after every mutating stage (assert the REAL response field, not HTTP 200)
//   * exit-state handling (MENU_NOT_FOUND/LOW_QUALITY/MANUAL_REVIEW → record + skip, never spine)
//   * per-item continue-on-failure (one bad item never aborts the run)
//   * a per-run report (per-item outcome + claim URLs + summary) written lossless to loops/runs/
//
// Safety: writes ONLY shadow tenants (owner_id NULL, status='closed', published_at NULL). Never
// publishes, never touches real tenants. The claim URL doubles as the day-one decline/erase undo.
//
// Secrets: PROVISION_OPS_SECRET + PROVISION_BASE_URL are read from env. The secret is sent ONLY as
// the x-provision-ops-secret header and is NEVER printed.
//
// Card: loops/acquisition-bulk-provision.yaml · Report: loops/reports/acquisition-bulk-provision-0.1.md
//
// Usage:
//   PROVISION_BASE_URL=https://dowiz-staging.fly.dev \
//   PROVISION_OPS_SECRET=*** \
//   node scripts/acquisition-bulk-provision.mjs ./restaurants.json   # or .csv
//
// Input row fields: place_id (req), website_url (req), name (req), slug (req), phone?, invited_contact?
//   - CSV: first line is the header; columns matched by name.
//   - JSON: an array of objects with those keys.

import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const isUuid = (v) => typeof v === 'string' && UUID_RE.test(v);
const isNonEmptyStr = (v) => typeof v === 'string' && v.length > 0;

// States that mean "menu not yet enriched" — extract must run to reach ENRICHED before any spine.
const NEED_EXTRACT = new Set(['SOURCED', 'PLACE_INGESTED', 'MENU_EXTRACTED']);
// Exit/terminal states from the shipped state-machine: never provision these; record + skip.
const EXIT_STATES = new Set(['MENU_NOT_FOUND', 'LOW_QUALITY', 'MANUAL_REVIEW', 'DISQUALIFIED', 'ABANDONED']);

function loadConfig() {
  const baseUrl = process.env.PROVISION_BASE_URL;
  const secret = process.env.PROVISION_OPS_SECRET;
  const file = process.argv[2];
  const missing = [];
  if (!baseUrl) missing.push('PROVISION_BASE_URL');
  if (!secret) missing.push('PROVISION_OPS_SECRET');
  if (!file) missing.push('<input-file> (argv[1])');
  if (missing.length) {
    // Fail-closed: refuse to run with a partial config. NEVER echo the secret value.
    console.error(`FATAL: missing required config: ${missing.join(', ')}`);
    process.exit(2);
  }
  return { baseUrl: baseUrl.replace(/\/+$/, ''), secret, file: resolve(file) };
}

function parseInput(file) {
  const raw = readFileSync(file, 'utf8');
  let rows;
  if (file.endsWith('.json')) {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) throw new Error('JSON input must be an array of row objects');
    rows = parsed;
  } else {
    // minimal CSV (no quoted-comma support — use JSON for values containing commas)
    const lines = raw.split(/\r?\n/).filter((l) => l.trim().length > 0);
    if (lines.length < 2) throw new Error('CSV needs a header line + ≥1 data row');
    const header = lines[0].split(',').map((h) => h.trim());
    rows = lines.slice(1).map((line) => {
      const cells = line.split(',');
      const o = {};
      header.forEach((h, i) => { o[h] = (cells[i] ?? '').trim(); });
      return o;
    });
  }
  // validate the loop's preconditions per row (a malformed row is a needs-review, not a crash)
  return rows.map((r, i) => ({
    place_id: r.place_id,
    website_url: r.website_url,
    name: r.name,
    slug: r.slug,
    phone: r.phone || undefined,
    invited_contact: r.invited_contact || undefined,
    _row: i + 1,
  }));
}

// ---- HTTP: one POST helper. Returns {status, json}; NEVER throws on non-2xx so gates can inspect.
async function makePost(baseUrl, secret) {
  return async function post(path, body) {
    let res;
    try {
      res = await fetch(`${baseUrl}${path}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-provision-ops-secret': secret },
        body: JSON.stringify(body),
      });
    } catch (e) {
      return { status: 0, json: { error: 'NETWORK', message: String(e?.message || e) } };
    }
    let json = {};
    try { json = await res.json(); } catch { json = {}; }
    return { status: res.status, json };
  };
}

// ---- the per-item loop body (resumable by state; a GATE after every mutating stage).
async function processItem(item, post, baseUrl) {
  const out = { row: item._row, place_id: item.place_id, name: item.name, slug: item.slug, warnings: [] };
  const fail = (reason, state) => ({ ...out, outcome: 'needs-review', reason, state });
  const skip = (reason, state) => ({ ...out, outcome: 'skipped-already-done', reason, state });

  // precondition: required fields present
  for (const k of ['place_id', 'website_url', 'name', 'slug']) {
    if (!isNonEmptyStr(item[k])) return fail(`MALFORMED_ROW:missing ${k}`);
  }

  // STAGE 0 — idempotent create / state read. POST /acquisition is ON CONFLICT DO UPDATE RETURNING,
  // so it returns the source's CURRENT state. This is the resume seam.
  const c = await post('/internal/acquisition', { place_id: item.place_id });
  if (c.status === 404) return fail('OPS_AUTH_404 (secret rejected / surface disabled)');
  if (!isUuid(c.json?.id) || !isNonEmptyStr(c.json?.state)) return fail(`CREATE_FAILED:${c.json?.error || c.status}`);
  const id = c.json.id;
  out.source_id = id;
  let state = c.json.state;

  // resume routing on the read state
  if (state === 'CLAIMED') return skip('already-claimed', state);
  if (state === 'CLAIM_OFFERED') return skip('already-invited (active invite exists)', state); // re-mint would 409
  if (EXIT_STATES.has(state)) return fail(`EXIT_STATE:${state}`, state); // never provision an exit-state source

  // STAGE 1 — extract (SOURCED→ENRICHED). The external/fallible stage (real site + AI). GATE on the
  // verdict STATE, not the HTTP code: MENU_NOT_FOUND/LOW_QUALITY/MANUAL_REVIEW → needs-review, never spine.
  if (NEED_EXTRACT.has(state)) {
    const ex = await post('/internal/acquisition/extract', { acquisition_source_id: id, website_url: item.website_url });
    if (ex.status === 503) return fail('EXTRACTION_UNAVAILABLE', state);
    const exState = ex.json?.state;
    if (exState !== 'ENRICHED') return fail(`EXTRACT:${exState || ex.json?.error || 'FAILED'}`, exState);
    state = 'ENRICHED';
  }

  // re-read state before any mutation (anti-double-provision: never spine on a stale assumption)
  const re = await post('/internal/acquisition', { place_id: item.place_id });
  state = re.json?.state || state;
  if (!['ENRICHED', 'PROVISIONED', 'VERIFIED'].includes(state)) return fail(`UNEXPECTED_STATE:${state}`, state);

  // STAGE 2 — mint provisioning token + write the shadow spine (ENRICHED→PROVISIONED). Two GATEs.
  if (state === 'ENRICHED') {
    const mint = await post('/internal/acquisition/provision/mint', { acquisition_source_id: id });
    if (mint.status !== 201 || !isNonEmptyStr(mint.json?.token)) return fail(`MINT:${mint.json?.error || mint.status}`, state);
    const spine = await post('/internal/acquisition/provision/spine', {
      acquisition_source_id: id, token: mint.json.token, name: item.name, slug: item.slug, phone: item.phone,
    });
    // GATE: real spine FKs, not a 201 alone
    if (spine.status !== 201 || !isUuid(spine.json?.org_id) || !isUuid(spine.json?.location_id)) {
      return fail(`SPINE:${spine.json?.error || spine.status}`, state);
    }
    out.org_id = spine.json.org_id;
    out.location_id = spine.json.location_id;
    state = 'PROVISIONED';
  }

  // STAGE 3 — verify (PROVISIONED→VERIFIED). The hard GATE: the rendered preview must serve + have items
  // + honest banner + noindex + never-orderable. GATE on verified===true (an empty shadow returns 409).
  if (state === 'PROVISIONED') {
    const v = await post('/internal/acquisition/claim/verify', { acquisition_source_id: id });
    if (v.json?.verified !== true) return fail(`NOT_VERIFIABLE:${v.json?.error || v.status}`, state);
    state = 'VERIFIED';
  }

  // STAGE 4 — mint the claim invite (VERIFIED→CLAIM_OFFERED). GATE on a real token.
  const base = item.base_url || baseUrl;
  const cm = await post('/internal/acquisition/claim/mint', {
    acquisition_source_id: id, invited_contact: item.invited_contact, base_url: base,
  });
  if (cm.status !== 201 || !isNonEmptyStr(cm.json?.token)) return fail(`CLAIM_MINT:${cm.json?.error || cm.status}`, state);

  // honesty: a token-only invite (no bound contact) is NOT web-claimable (claim.ts CONTACT_REQUIRED) —
  // only the decline/erase path works. Surface that as a warning, not a silent half-success.
  if (!item.invited_contact) out.warnings.push('no invited_contact → claim link is decline-only (CONTACT_REQUIRED on web accept)');

  const claimUrl = `${base}/claim#token=${cm.json.token}`;
  return { ...out, outcome: 'invited', state: 'CLAIM_OFFERED', claim_url: claimUrl, decline_url: claimUrl };
}

async function main() {
  const cfg = loadConfig();
  const items = parseInput(cfg.file);
  const post = await makePost(cfg.baseUrl, cfg.secret);

  const results = [];
  for (const item of items) {
    // continue-on-failure: a thrown error inside one item becomes that item's needs-review, never an abort.
    try {
      results.push(await processItem(item, post, cfg.baseUrl));
    } catch (e) {
      results.push({ row: item._row, place_id: item.place_id, name: item.name, outcome: 'needs-review', reason: `EXCEPTION:${String(e?.message || e)}` });
    }
  }

  const summary = {
    attempted: results.length,
    invited: results.filter((r) => r.outcome === 'invited').length,
    needs_review: results.filter((r) => r.outcome === 'needs-review').length,
    skipped_already_done: results.filter((r) => r.outcome === 'skipped-already-done').length,
  };

  // ---- per-run report (the §5 artifact for this operator loop): lossless JSON + human stdout.
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const runDir = resolve('loops/runs');
  mkdirSync(runDir, { recursive: true });
  const runFile = `${runDir}/acquisition-bulk-provision-${stamp}.json`;
  writeFileSync(runFile, JSON.stringify({ at: new Date().toISOString(), base_url: cfg.baseUrl, summary, results }, null, 2));

  console.log('\n=== P6 acquisition bulk-provisioning — run report ===');
  console.log(`base: ${cfg.baseUrl}   (secret: [redacted])`);
  for (const r of results) {
    const tag = r.outcome === 'invited' ? 'INVITED' : r.outcome === 'needs-review' ? 'NEEDS-REVIEW' : 'SKIPPED';
    let line = `  [${tag}] row#${r.row} ${r.name || r.place_id} (${r.place_id})`;
    if (r.state) line += ` state=${r.state}`;
    if (r.reason) line += ` — ${r.reason}`;
    console.log(line);
    if (r.claim_url) console.log(`           claim/decline: ${r.claim_url}`);
    for (const w of r.warnings || []) console.log(`           ⚠ ${w}`);
  }
  console.log('\nsummary:', JSON.stringify(summary));
  console.log(`run artifact: ${runFile}`);
  console.log('undo: each claim link is also the token-only decline/erase (POST /api/claim/decline {token}).');

  // exit code: 0 if every item resolved to a terminal outcome (it always does); 3 if ANY needs-review,
  // so a CI/cron caller can detect partial failure without parsing stdout.
  process.exit(summary.needs_review > 0 ? 3 : 0);
}

main().catch((e) => { console.error('FATAL:', e?.message || e); process.exit(1); });
