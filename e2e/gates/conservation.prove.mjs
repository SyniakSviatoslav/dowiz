// THE GATE'S OWN PROOF. Runs `conservation.mjs` against a stubbed platform and
// checks that each law goes red when it should and green when it should not.
//
// WHY THIS EXISTS AND NOT A LIVE CORRUPTION. This repo keeps a catalogue of
// instruments that measured nothing, and the rule that came out of it is that
// a gate is triggered before it is trusted. Laws 6 and 7 are about a corrupted
// record and an edited ledger — the two things nobody may go and do to a live
// venue to see whether the alarm works. So the platform is stubbed instead:
// `fetch` answers with the shapes `/api/owner/health` and
// `/api/owner/backup/cloud` really return, and the gate is run unmodified.
//
// THE GREEN CASES ARE HALF THE PROOF. A check that refuses everything is not a
// check, and "absent" must read as NOT MEASURED rather than as zero, or the
// first deployment that has not run its nightly yet turns the gate red and the
// gate gets switched off.
//
//   node e2e/gates/conservation.prove.mjs
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const GATE = path.join(HERE, 'conservation.mjs');
const NIGHT = 26 * 3600e3;
const hex = (c) => c.repeat(64);

const ok = { images: {}, orders: 6, events: 6, quarantined: [] };
const witnessed = (over) => ({
  configured: true,
  witness: { atMs: Date.now() - NIGHT, records: 6, archived: 0, total: 6, tip: 'aa', found: [], ...over },
});

// Each case: what the two endpoints answer, and the sentence the gate must say.
const CASES = {
  healthy: { health: ok, backup: witnessed(), red: false },

  'quarantined record': {
    health: { ...ok, events: 5, quarantined: [{ image: 'log', id: hex('a'), at: 0, reason: 'kind' }] },
    backup: witnessed(),
    red: 'unreadable (kind)',
  },
  'quarantine in another image': {
    health: { ...ok, quarantined: [{ image: 'audit', id: hex('c'), at: 3, reason: 'subject-framing' }] },
    backup: witnessed(),
    red: 'audit record',
  },
  'the log does not add up': {
    health: { ...ok, events: 4, quarantined: [{ image: 'log', id: hex('a'), at: 0, reason: 'kind' }] },
    backup: witnessed(),
    red: 'claims 6 records, serves 4, withholds 1',
  },

  contradicted: {
    health: ok,
    backup: witnessed({ found: ['the history shrank: 20 records on 1789808707000, 16 now'] }),
    red: 'the history shrank',
  },
  'the nightly has stopped': {
    health: ok,
    backup: witnessed({ atMs: Date.now() - 5 * 24 * 3600e3 }),
    red: 'the nightly has stopped',
  },
  // ABSENT IS NOT ZERO, in both directions: a venue whose nightly has not run
  // since the witness shipped, and a deployment older than the `events` field.
  'no census yet': { health: ok, backup: { configured: true }, red: false },
  'a worker without the new fields': {
    health: { images: {}, orders: 6 },
    backup: { configured: true },
    red: false,
  },
};

const RUNNER = `
globalThis.fetch = async (url) => {
  const body = url.includes('/api/auth/login')
    ? { access_token: 't', user: { locationId: 'stub-venue' } }
    : url.includes('/api/owner/health') ? HEALTH
    : url.includes('/api/owner/backup/cloud') ? BACKUP
    : url.includes('/api/owner/orders') ? []
    : {};
  return { status: 200, text: async () => JSON.stringify(body) };
};
await import(GATE);
`;

let failed = 0;
for (const [name, c] of Object.entries(CASES)) {
  const script = RUNNER
    .replace('HEALTH', JSON.stringify(c.health))
    .replace('BACKUP', JSON.stringify(c.backup))
    .replace('GATE', JSON.stringify(GATE));
  const r = spawnSync(process.execPath, ['--input-type=module', '-e', script], {
    encoding: 'utf8',
    env: { ...process.env, HOSTS: 'stub' },
  });
  const out = (r.stdout || '') + (r.stderr || '');
  const red = r.status === 1;
  const want = c.red !== false;
  let verdict = red === want ? 'ok' : `WRONG COLOUR (exit ${r.status})`;
  if (verdict === 'ok' && want && !out.includes(c.red)) {
    verdict = `red, but did not say "${c.red}"`;
  }
  if (verdict !== 'ok') {
    failed += 1;
    console.log(`  ${name}: ${verdict}\n${out.split('\n').map((l) => `    | ${l}`).join('\n')}`);
  } else {
    console.log(`  ${name}: ${want ? 'red' : 'green'}, as it should be`);
  }
}

if (failed) {
  console.log(`\nTHE GATE DOES NOT FIRE AS DESCRIBED — ${failed} case(s).`);
  process.exit(1);
}
console.log('\nEVERY LAW FIRES, AND ONLY WHEN IT SHOULD.');
