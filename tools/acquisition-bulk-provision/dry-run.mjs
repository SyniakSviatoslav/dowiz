// Anti-cheat dry-run / certification harness for the P6 acquisition bulk-provisioning loop (M9).
//
// It runs the REAL orchestrator (scripts/acquisition-bulk-provision.mjs) as a child process against a
// state-machine-faithful mock, on crafted fixtures, and asserts the loop:
//   1. INVITES only items that pass every real GATE (claim token returned).
//   2. records NEEDS-REVIEW for every broken fixture and NEVER aborts the run (continue-on-failure).
//   3. NEVER provisions an exit-state source (no spine on MENU_NOT_FOUND/LOW_QUALITY).
//   4. is IDEMPOTENT/RESUMABLE: a re-run skips already-invited items + re-resumes a stuck one (no re-spine).
//   5. is NO-FAKE-GREEN: a LIAR backend (200 verified:false / 201 no-token) → NEEDS-REVIEW, not invited.
//   6. FAILS CLOSED: a wrong ops secret (404) → all needs-review, zero faked successes.
//
// A status-only ("HTTP 200 == pass") loop would FAIL scenarios 5 and 6 — that is the cheat this catches.
//
// Run: node tools/acquisition-bulk-provision/dry-run.mjs   (exit 0 = CERTIFIED gates green)

import { spawn } from 'node:child_process';
import { writeFileSync, readFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startMock } from './mock-internal.mjs';

const ORCH = new URL('../../scripts/acquisition-bulk-provision.mjs', import.meta.url).pathname;
const SECRET = 'dry-run-secret-xyz';
const tmp = mkdtempSync(join(tmpdir(), 'acq-dryrun-'));

let failures = 0;
const assert = (cond, msg) => { console.log(`  ${cond ? 'PASS' : 'FAIL'} — ${msg}`); if (!cond) failures++; };

function runOrchestrator(baseUrl, secret, fixturePath) {
  return new Promise((resolveP) => {
    const child = spawn(process.execPath, [ORCH, fixturePath], {
      env: { ...process.env, PROVISION_BASE_URL: baseUrl, PROVISION_OPS_SECRET: secret },
    });
    let stdout = '';
    child.stdout.on('data', (d) => (stdout += d));
    child.stderr.on('data', (d) => (stdout += d));
    child.on('close', (code) => {
      const m = /run artifact: (.+)/.exec(stdout);
      const report = m ? JSON.parse(readFileSync(m[1].trim(), 'utf8')) : null;
      // secret must never be echoed
      const leaked = stdout.includes(secret);
      resolveP({ code, stdout, report, leaked });
    });
  });
}

function fixture(name, rows) {
  const p = join(tmp, `${name}.json`);
  writeFileSync(p, JSON.stringify(rows, null, 2));
  return p;
}

async function main() {
  console.log('=== P6 bulk-provisioning loop · anti-cheat dry-run ===\n');

  // ---------- Scenario A: mixed batch (happy + 3 broken) ----------
  console.log('Scenario A — mixed batch (continue-on-failure + real gates + never-spine-exit):');
  const mockA = await startMock({ secret: SECRET });
  const rowsA = [
    { place_id: 'place-happy-1', website_url: 'https://a.example', name: 'Alpha', slug: 'alpha', invited_contact: 'a@x.com' },
    { place_id: 'place-happy-2-nocontact', website_url: 'https://b.example', name: 'Beta', slug: 'beta' },
    { place_id: 'place-menunotfound-3', website_url: 'https://c.example', name: 'Gamma', slug: 'gamma', invited_contact: 'c@x.com' },
    { place_id: 'place-lowquality-4', website_url: 'https://d.example', name: 'Delta', slug: 'delta', invited_contact: 'd@x.com' },
    { place_id: 'place-emptymenu-5', website_url: 'https://e.example', name: 'Eps', slug: 'eps', invited_contact: 'e@x.com' },
  ];
  const a = await runOrchestrator(mockA.baseUrl, SECRET, fixture('A', rowsA));
  assert(a.report?.summary.attempted === 5, `all 5 items processed (no abort) — got ${a.report?.summary.attempted}`);
  assert(a.report?.summary.invited === 2, `exactly 2 invited (the two enriched-with-items) — got ${a.report?.summary.invited}`);
  assert(a.report?.summary.needs_review === 3, `3 needs-review (menunotfound/lowquality/emptymenu) — got ${a.report?.summary.needs_review}`);
  const happy1 = a.report?.results.find((r) => r.place_id === 'place-happy-1');
  assert(/\/claim#token=/.test(happy1?.claim_url || ''), 'happy-1 got a real fragment claim URL');
  assert(happy1?.state === 'CLAIM_OFFERED', 'happy-1 advanced to CLAIM_OFFERED');
  const happy2 = a.report?.results.find((r) => r.place_id.includes('happy-2'));
  assert((happy2?.warnings || []).some((w) => /CONTACT_REQUIRED/.test(w)), 'happy-2 (no contact) carries the decline-only warning (no silent half-success)');
  const mnf = mockA.sources.get('place-menunotfound-3');
  assert(mnf?.state === 'MENU_NOT_FOUND' && mnf?.org_id === null, 'NEVER provisioned the MENU_NOT_FOUND source (org_id still null)');
  const lq = mockA.sources.get('place-lowquality-4');
  assert(lq?.org_id === null, 'NEVER provisioned the LOW_QUALITY source');
  assert(a.code === 3, `exit code 3 signals partial failure to a cron caller — got ${a.code}`);
  assert(a.leaked === false, 'ops secret never printed to stdout');

  // ---------- Scenario B: idempotent re-run against the SAME mock ----------
  console.log('\nScenario B — idempotent / resumable re-run (no double-provision):');
  const b = await runOrchestrator(mockA.baseUrl, SECRET, fixture('A', rowsA)); // same state, persisted in mockA
  assert(b.report?.summary.invited === 0, `re-run invites 0 (happy ones already CLAIM_OFFERED) — got ${b.report?.summary.invited}`);
  assert(b.report?.summary.skipped_already_done === 2, `2 skipped-already-invited — got ${b.report?.summary.skipped_already_done}`);
  const empty2 = mockA.sources.get('place-emptymenu-5');
  assert(empty2?.state === 'PROVISIONED', 'empty-menu source resumed at verify, stayed PROVISIONED (re-spine did NOT happen)');
  await mockA.close();

  // ---------- Scenario C: LIAR verify (200 verified:false) ----------
  console.log('\nScenario C — no-fake-green: backend lies at VERIFY (200 verified:false):');
  const mockC = await startMock({ secret: SECRET, liar: 'verify' });
  const c = await runOrchestrator(mockC.baseUrl, SECRET, fixture('C', [rowsA[0]]));
  assert(c.report?.summary.invited === 0, 'a 200-with-verified:false is NOT counted as invited');
  assert(/NOT_VERIFIABLE/.test(c.report?.results[0]?.reason || ''), 'classified NEEDS-REVIEW:NOT_VERIFIABLE (gate read the field, not the 200)');
  await mockC.close();

  // ---------- Scenario D: LIAR claim/mint (201 but no token) ----------
  console.log('\nScenario D — no-fake-green: backend lies at CLAIM/MINT (201, no token):');
  const mockD = await startMock({ secret: SECRET, liar: 'claim' });
  const d = await runOrchestrator(mockD.baseUrl, SECRET, fixture('D', [rowsA[0]]));
  assert(d.report?.summary.invited === 0, 'a 201-with-no-token is NOT counted as invited');
  assert(/CLAIM_MINT/.test(d.report?.results[0]?.reason || ''), 'classified NEEDS-REVIEW:CLAIM_MINT (gate requires a real token string)');
  await mockD.close();

  // ---------- Scenario E: fail-closed on a wrong ops secret ----------
  console.log('\nScenario E — fail-closed: wrong ops secret (surface 404s):');
  const mockE = await startMock({ secret: SECRET });
  const e = await runOrchestrator(mockE.baseUrl, 'WRONG-SECRET', fixture('E', [rowsA[0]]));
  assert(e.report?.summary.invited === 0, 'zero faked successes when the secret is rejected');
  assert(/OPS_AUTH_404/.test(e.report?.results[0]?.reason || ''), 'classified NEEDS-REVIEW:OPS_AUTH_404 (fail-closed, not crash)');
  assert(e.leaked === false, 'wrong secret not echoed either');
  await mockE.close();

  console.log(`\n=== ${failures === 0 ? 'ALL GATES GREEN — anti-cheat dry-run PASS' : `${failures} ASSERTION(S) FAILED — RED`} ===`);
  process.exit(failures === 0 ? 0 : 1);
}

main().catch((e) => { console.error('HARNESS FATAL:', e); process.exit(1); });
