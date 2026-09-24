// Live-walk helpers: one browser, capped processes, every console error,
// failed request and CSP violation collected per screen.
import { chromium } from 'playwright';
import fs from 'node:fs';

export const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
export const OUT = process.env.OUT || '/tmp/claude-0/-root/2bd4b866-4cdd-4908-9891-de3ce4677585/scratchpad/walk';
fs.mkdirSync(OUT, { recursive: true });
export const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => { const s = l.slice(7); const i = s.indexOf('='); return [s.slice(0, i), s.slice(i + 1).replace(/^['"]|['"]$/g, '')]; }));
export const STATE = `${OUT}/state.json`;
export const st = () => { try { return JSON.parse(fs.readFileSync(STATE, 'utf8')); } catch { return {}; } };
export const save = patch => fs.writeFileSync(STATE, JSON.stringify({ ...st(), ...patch }, null, 1));

export const log = [];
export const say = (ok, name, detail = '') => {
  const l = `${ok === null ? 'info' : ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`;
  log.push(l); console.log(l);
};
export const issues = [];
export function watch(p, tag) {
  p.on('pageerror', e => { issues.push(`${tag} pageerror ${e.message.slice(0, 200)}`); console.log(`  !! ${tag} pageerror ${e.message.slice(0, 200)}`); });
  p.on('console', m => {
    if (m.type() !== 'error' && !/Content Security Policy|Refused to/.test(m.text())) return;
    if (/WebGL|GPU/i.test(m.text())) return;
    issues.push(`${tag} console ${m.text().slice(0, 240)}`); console.log(`  !! ${tag} console ${m.text().slice(0, 240)}`);
  });
  p.on('response', r => {
    if (r.status() >= 400) { const s = `${tag} http ${r.status()} ${r.request().method()} ${r.url().replace(HOST, '').slice(0, 120)}`; issues.push(s); console.log('  !! ' + s); }
  });
  p.on('requestfailed', r => { const s = `${tag} reqfail ${r.failure()?.errorText} ${r.url().replace(HOST, '').slice(0, 120)}`; issues.push(s); console.log('  !! ' + s); });
  p.addInitScript(() => {
    window.__csp = [];
    document.addEventListener('securitypolicyviolation', e => window.__csp.push(`${e.violatedDirective} ${e.blockedURI} ${e.sourceFile}:${e.lineNumber}`));
  });
}
export async function csp(p, tag) {
  const v = await p.evaluate(() => window.__csp || []).catch(() => []);
  for (const x of v) { issues.push(`${tag} CSP ${x}`); console.log(`  !! ${tag} CSP ${x}`); }
  return v;
}

export function procs() { return fs.readdirSync('/proc').filter(n => /^\d+$/.test(n)).length; }
export async function browser() {
  const n = procs();
  if (n > 26) { console.log(`process count ${n} > 26: refusing to launch`); process.exit(3); }
  return chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage', '--no-zygote', '--single-process', '--renderer-process-limit=1', '--disable-gpu', '--disable-extensions'] });
}
export async function api(path, { method = 'GET', body, token, headers = {} } = {}) {
  const r = await fetch(`${HOST}${path}`, { method, headers: { 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}), ...headers }, body: body == null ? undefined : JSON.stringify(body) });
  const t = await r.text();
  let b; try { b = JSON.parse(t); } catch { b = t; }
  return { status: r.status, body: b };
}
export async function ownerToken() {
  const r = await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD } });
  return r.body.access_token;
}
export async function staffToken() {
  const r = await api('/api/staff/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD } });
  return r.body.jwt;
}
export function finish() {
  fs.appendFileSync(`${OUT}/log.txt`, log.join('\n') + '\n' + issues.map(i => 'ISSUE ' + i).join('\n') + '\n');
  console.log(`\n${issues.length} issues`);
}
