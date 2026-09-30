// IS THE LIVE WORKER THE COMMIT WE JUST DEPLOYED? (W-DEPLOY D4, 2026-09-30)
//
//   node tools/deploy/verify.mjs --host https://qa-durres.dowiz.org --commit <40-hex> \
//        --src <clean HEAD copy> [--files admin/app.js,admin/core.js,admin/i18n.js]
//
// Two independent questions, because each can be right while the other is wrong:
//   1. CODE: GET /api/version must answer {commit: <HEAD>}. The commit is compiled into the wasm
//      (workers/api/src/version.rs), so this is the Worker's own statement of what it was built from.
//   2. ASSETS: each named file under workers/api/public must come back byte-for-byte equal to HEAD's
//      copy. Static assets are uploaded beside the wasm and answered without it, so a right commit
//      proves nothing about them.
// ONE fetch per URL, no polling. Only a CONNECT that timed out is retried (the box's TCP connect
// to Cloudflare takes 4-11 s and undici gives up at 10 s -- e2e/kit-regression/_retry-fetch.mjs);
// such a request never reached the server. Every other failure is a FAIL with its name.
// Exit 0: every check passed. Exit 1: at least one FAIL line was printed. Exit 2: bad arguments.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const DEFAULT_FILES = ['admin/app.js', 'admin/core.js', 'admin/i18n.js'];

async function fetchOnce(url, fetchImpl) {
  for (let i = 0; ; i++) {
    try { return await fetchImpl(url, { headers: { 'cache-control': 'no-cache' } }); }
    catch (e) {
      if (i < 5 && e?.cause?.code === 'UND_ERR_CONNECT_TIMEOUT') continue;
      throw e;
    }
  }
}

// Returns { ok, lines } -- every line is `ok   <name> ...` or `FAIL <name> ...`.
export async function verify({ host, commit, src, files = DEFAULT_FILES, fetchImpl = globalThis.fetch }) {
  const lines = [];
  const say = (ok, name, detail) => lines.push(`${ok ? 'ok  ' : 'FAIL'} ${name} :: ${detail}`);
  const bust = `deploy=${commit.slice(0, 12)}`;

  try {
    const r = await fetchOnce(`${host}/api/version?${bust}`, fetchImpl);
    const text = await r.text();
    let live = null;
    try { live = JSON.parse(text); } catch { /* reported below */ }
    if (r.status !== 200 || !live) say(false, 'version-route', `HTTP ${r.status}: ${text.slice(0, 120)}`);
    else say(live.commit === commit, 'version-commit', `live=${live.commit} head=${commit} built_at=${live.built_at}`);
  } catch (e) {
    say(false, 'version-route', `fetch failed: ${e?.cause?.code || e.message}`);
  }

  for (const f of files) {
    const local = path.join(src, 'workers/api/public', f);
    let want;
    try { want = fs.readFileSync(local); } catch { say(false, `asset ${f}`, `not in HEAD copy: ${local}`); continue; }
    try {
      const r = await fetchOnce(`${host}/${f}?${bust}`, fetchImpl);
      const got = Buffer.from(await r.arrayBuffer());
      const same = r.status === 200 && Buffer.compare(got, want) === 0;
      say(same, `asset ${f}`, `HTTP ${r.status} live=${got.length}B head=${want.length}B${same ? ' identical' : ' DIFFER'}`);
    } catch (e) {
      say(false, `asset ${f}`, `fetch failed: ${e?.cause?.code || e.message}`);
    }
  }
  return { ok: lines.every(l => l.startsWith('ok')), lines };
}

function args(argv) {
  const o = {};
  for (let i = 0; i < argv.length; i += 2) o[argv[i].replace(/^--/, '')] = argv[i + 1];
  return o;
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  const a = args(process.argv.slice(2));
  if (!a.host || !/^[0-9a-f]{40}$/.test(a.commit || '') || !a.src) {
    console.log('usage: verify.mjs --host URL --commit <40-hex> --src DIR [--files a,b,c]');
    process.exit(2);
  }
  const res = await verify({ host: a.host, commit: a.commit, src: a.src, files: a.files ? a.files.split(',') : DEFAULT_FILES });
  for (const l of res.lines) console.log(l);
  console.log(`verify: ${res.ok ? 'PASS' : 'FAIL'} ${res.lines.filter(l => l.startsWith('FAIL')).length} failed of ${res.lines.length}`);
  process.exit(res.ok ? 0 : 1);
}
