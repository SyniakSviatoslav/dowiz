// Row 33: the landing words SERVED by dowiz.org are the deployed commit's file
// and pass tools/gates/pq-words.sh -- run on the served bytes, not the tree's.
import fs from 'node:fs';
import os from 'node:os';
import { execFileSync } from 'node:child_process';
const F = 'workers/api/public/platform/landing-words.js';
export default async function ({ lib, check, must, note }) {
  const v = await (await fetch('https://dowiz.org/api/version', { headers: { 'user-agent': lib.UA } })).json();
  const r = await fetch('https://dowiz.org/platform/landing-words.js', { headers: { 'user-agent': lib.UA } });
  must(r.ok, `landing-words.js ${r.status}`);
  const served = check('response_schema', await r.text());
  let built;
  try { built = execFileSync('git', ['-C', new URL('../../..', import.meta.url).pathname, 'show', `${v.commit}:${F}`], { encoding: 'utf8', maxBuffer: 8 << 20 }); }
  catch { throw new Error(`the live commit ${v.commit} is not in this clone`); }
  must(served === built, `served file (${served.length} B) differs from ${F} at ${v.commit.slice(0, 8)} (${built.length} B)`);
  // The gate, on a scratch root holding the SERVED file as its whole public tree.
  const root = fs.mkdtempSync(`${os.tmpdir()}/pq-live-`);
  fs.mkdirSync(`${root}/workers/api/public/platform`, { recursive: true });
  fs.writeFileSync(`${root}/${F}`, served);
  fs.writeFileSync(`${root}/README.md`, ''); fs.writeFileSync(`${root}/SECURITY.md`, '');
  let out;
  try { out = execFileSync('sh', [new URL('../../gates/pq-words.sh', import.meta.url).pathname, root], { encoding: 'utf8' }); }
  catch (e) { throw new Error(`pq-words on the served file: rc ${e.status}: ${String(e.stdout).trim().split('\n').slice(-3).join(' / ')}`); }
  finally { fs.rmSync(root, { recursive: true, force: true }); }
  note(`served == ${F}@${v.commit.slice(0, 8)} (${served.length} B); ${out.trim().split('\n').pop()}`);
}
