// LIVE PROBE (W-AI, 2026-10-03): the owner's AI on the QA hub ONLY.
//
//   set -a; . /root/.dowiz_owner; set +a; node tools/live-proof/probes/feature-ai.mjs
//
// Main runs this after a deploy that carries the W-AI routes AND the `[ai]`
// binding in wrangler.toml. The host is fixed to qa-durres.dowiz.org.
//
// WHAT IT PROVES, through the real Worker, Durable Object and providers:
//   1. GET  /api/owner/ai validates against ai.owner.v1 and carries no key.
//   2. POST /api/owner/ai/ask with a starter question (no model): every number
//      equals the cells it names in the public route it cites
//      (/api/owner/analytics?days=7 ...), fetched separately.
//   3. GET  /api/owner/ai/explain?screen=analytics: the same, per card.
//   4. WORKERS AI: with ai.enabled=1 and ai.provider=workers (set for the run,
//      restored after), POST /ai/test answers state "ok" from
//      @cf/qwen/qwen3-30b-a3b-fp8, a long-tail question is PICKED by the model
//      (pickedBy "model", provider "workers-ai"), its numbers equal the
//      route's cells, and the venue's budget meter moved.
//   5. OPENROUTER: with no owner key on the QA hub, POST /ai/test {provider:
//      "own"} against https://openrouter.ai/api/v1 says "needs-key" ->
//      reported NEEDS-KEY, never a pass. With a key the QA owner saved, it
//      must say "ok" (the real /api/v1/chat/completions answered).
//   6. No response body of the run contains an OpenRouter key shape (sk-or-).
// Exit 0 = every part PROVEN; 1 = a part FAILED; 2 = NEEDS-KEY / NEEDS-BINDING
// for a part and nothing failed (a skip is never a pass).
import fs from 'node:fs';

const HOST = 'https://qa-durres.dowiz.org';
const UA = 'Mozilla/5.0 (dowiz live-proof feature-ai)';
const OPENROUTER = 'https://openrouter.ai/api/v1';
const results = [];
const say = (k, m) => { results.push(k); console.log(`${k} ${m}`); };
const STOP = new Error('stop');
const die = m => { say('FAILED', m); throw STOP; };

const email = process.env.QA_OWNER_EMAIL || process.env.OWNER_EMAIL, password = process.env.QA_OWNER_PASSWORD || process.env.OWNER_PASSWORD;
if (!email || !password) { console.log('NEEDS-KEY QA_OWNER_EMAIL / QA_OWNER_PASSWORD (or OWNER_*) are not set'); process.exit(2); }

const C = JSON.parse(fs.readFileSync(new URL('../contracts/feature-ai.json', import.meta.url), 'utf8'));

/// The JSON Schema subset the contracts use; the first violation, or null.
function check(schema, v, at = '$'){
  if (!schema) return null;
  if (schema.const !== undefined && v !== schema.const) return `${at}: expected ${JSON.stringify(schema.const)}, got ${JSON.stringify(v)}`;
  if (schema.enum && !schema.enum.includes(v)) return `${at}: ${JSON.stringify(v)} not in ${JSON.stringify(schema.enum)}`;
  const types = [].concat(schema.type || []);
  const kind = v === null ? 'null' : Array.isArray(v) ? 'array' : Number.isInteger(v) ? 'integer' : typeof v;
  if (types.length && !types.some(t => t === kind || (t === 'number' && kind === 'integer'))) return `${at}: type ${kind}, wanted ${types.join('|')}`;
  if (kind === 'object') {
    for (const r of schema.required || []) if (!(r in v)) return `${at}: missing ${r}`;
    for (const [k, s] of Object.entries(schema.properties || {})) if (k in v) { const e = check(s, v[k], `${at}.${k}`); if (e) return e; }
    if (schema.additionalProperties === false) for (const k of Object.keys(v)) if (!(k in (schema.properties || {}))) return `${at}: unexpected ${k}`;
  }
  if (kind === 'array') {
    if (schema.minItems != null && v.length < schema.minItems) return `${at}: ${v.length} items < ${schema.minItems}`;
    for (let i = 0; i < v.length; i++) { const e = check(schema.items, v[i], `${at}[${i}]`); if (e) return e; }
  }
  return null;
}

let token = null;
const bodies = [];
async function call(method, path, body){
  const r = await fetch(HOST + path, { method, headers: { 'user-agent': UA, 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}) },
    body: body == null ? undefined : JSON.stringify(body) });
  const text = await r.text();
  bodies.push(text);
  let json = null; try { json = JSON.parse(text); } catch {}
  return { status: r.status, json, text };
}
const valid = (route, r, what) => { const e = check(C.routes[route].response_schema, r.json); if (e) die(`${what}: ${route} schema: ${e}`); };
const ptr = (v, p) => String(p).split('/').slice(1).reduce((o, s) => (o == null ? o : o[s.replace(/~1/g, '/').replace(/~0/g, '~')]), v);

/// Every number equals the sum of the cells it names in its own source route.
async function traced(numbers, what){
  if (!numbers?.length) die(`${what}: no numbers`);
  const cache = {};
  for (const n of numbers) {
    cache[n.source] ||= await call('GET', n.source);
    const src = cache[n.source];
    if (src.status !== 200) die(`${what}: source ${n.source} ${src.status}`);
    const sum = (n.pointers || []).reduce((a, p) => a + Number(ptr(src.json, p)), 0);
    if ((n.pointers || []).length && sum !== n.value) die(`${what}: ${n.value} but ${n.source} ${n.pointers.join('+')} = ${sum}`);
  }
}

const settings = async () => (await call('GET', '/api/owner/settings')).json?.values || {};
const set = (key, value) => call('POST', '/api/owner/settings', { key, value });
let restore = null;
async function finish(){
  if (restore) for (const [k, v] of Object.entries(restore)) await set(k, v);
  const leak = bodies.find(b => /sk-or-[A-Za-z0-9-]{8,}/.test(b));
  if (leak) { results.push('FAILED'); console.log('FAILED a response carried an OpenRouter key shape'); }
  const code = results.includes('FAILED') ? 1 : results.includes('NEEDS-KEY') || results.includes('NEEDS-BINDING') ? 2 : 0;
  console.log(`feature-ai: ${results.filter(r => r === 'PROVEN').length} PROVEN, ${results.filter(r => r !== 'PROVEN').join(' ') || 'nothing else'}; exit ${code}`);
  process.exit(code);
}

async function main(){
  const login = await call('POST', '/api/auth/login', { email, password });
  if (login.status !== 200 || !login.json?.access_token) die(`login ${login.status}: ${login.text.slice(0, 120)}`);
  token = login.json.access_token;

  // 1. the card
  const st = await call('GET', '/api/owner/ai');
  if (st.status !== 200) die(`GET /api/owner/ai ${st.status}: ${st.text.slice(0, 160)}`);
  valid('status', st, '1');
  say('PROVEN', `1 status: mode=${st.json.mode} keySet=${st.json.keySet} workersAi=${st.json.workersAi.available} budget ${st.json.budget.used}/${st.json.budget.cap}`);

  // 2. a starter question, no model
  const a = await call('POST', '/api/owner/ai/ask', { question: 'What was the revenue this week?', lang: 'en', reword: false });
  if (a.status !== 200) die(`ask ${a.status}: ${a.text.slice(0, 160)}`);
  valid('ask', a, '2');
  if (!a.json.understood || a.json.pickedBy !== 'lexicon' || a.json.query.query !== 'revenue') die(`2: ${a.text.slice(0, 200)}`);
  await traced(a.json.numbers, '2');
  say('PROVEN', `2 ask (lexicon): "${a.json.answer}" = ${a.json.source}`);

  // 3. the explain cards
  const ex = await call('GET', '/api/owner/ai/explain?screen=analytics&days=7&lang=sq');
  if (ex.status !== 200) die(`explain ${ex.status}: ${ex.text.slice(0, 160)}`);
  valid('explain', ex, '3');
  for (const c of ex.json.cards) if (c.numbers.length) await traced(c.numbers.map(n => ({ ...n, source: c.source })), `3 ${c.kind}`);
  say('PROVEN', `3 explain: ${ex.json.cards.map(c => c.kind).join(',')}`);

  // 4. Workers AI
  const before = await settings();
  restore = { 'ai.enabled': before['ai.enabled'] || '0', 'ai.provider': before['ai.provider'] || 'auto' };
  await set('ai.enabled', '1');
  await set('ai.provider', 'workers');
  const wt = await call('POST', '/api/owner/ai/test', { provider: 'workers' });
  valid('test', wt, '4');
  if (wt.json.state === 'workers-ai-unavailable') say('NEEDS-BINDING', '4 Workers AI: the deployed Worker has no [ai] binding');
  else if (wt.json.state !== 'ok') die(`4 test: ${wt.text.slice(0, 200)}`);
  else {
    if (wt.json.model !== C.external.workers_ai.model) die(`4: model ${wt.json.model}, contract ${C.external.workers_ai.model}`);
    const lt = await call('POST', '/api/owner/ai/ask', { question: 'How did we do lately?', lang: 'en', reword: false });
    valid('ask', lt, '4');
    const pick = (lt.json.ai || [])[0] || {};
    if (lt.json.pickedBy !== 'model' || pick.provider !== 'workers-ai') die(`4 long tail: ${lt.text.slice(0, 240)}`);
    if (lt.json.understood) await traced(lt.json.numbers, '4');
    const after = await call('GET', '/api/owner/ai');
    if (!(after.json.budget.used > st.json.budget.used)) die(`4: the budget did not move (${st.json.budget.used} -> ${after.json.budget.used})`);
    say('PROVEN', `4 Workers AI: picked ${JSON.stringify(lt.json.query || lt.json.why)}, budget ${st.json.budget.used} -> ${after.json.budget.used}`);
  }

  // 5. OpenRouter with the owner's own key, or NEEDS-KEY
  await set('ai.provider', 'own');
  if (!before['ai.endpoint']) { restore['ai.endpoint'] = ''; await set('ai.endpoint', OPENROUTER); }
  const ot = await call('POST', '/api/owner/ai/test', { provider: 'own' });
  valid('test', ot, '5');
  if (ot.json.state === 'needs-key') say('NEEDS-KEY', '5 OpenRouter: no owner key on the QA hub (each owner pastes their own; the platform holds none)');
  else if (ot.json.state === 'ok') say('PROVEN', `5 own endpoint answered: ${ot.json.model}`);
  else die(`5 test: ${ot.text.slice(0, 240)}`);
}

try { await main(); } catch (e) { if (e !== STOP) say('FAILED', `probe error: ${e.message || e}`); }
await finish();
