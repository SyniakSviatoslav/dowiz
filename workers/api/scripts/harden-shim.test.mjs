// node --test workers/api/scripts/harden-shim.test.mjs
//
// THE GUARD, DRIVEN. The three guard functions (hook `se`, recorder `c`, gate `s`) are cut out of
// a shim text -- the exact shape wasm-bindgen 0.2.128 emitted for the 2026-11-04 bundle, minified
// names and all -- and run against a stub instance whose abort hook THROWS, which is the state the
// incident left behind. Unpatched: every later call throws "Module terminated" forever (the RED
// twin that proves the test sees the defect). Patched: the instance is reinitialised and the next
// call passes.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { harden, main, MARK, REINIT, TERMINATED } from './harden-shim.mjs';

// Verbatim from build/index.js of version 1dacb988 (worker-build 0.8.5, wasm-bindgen 0.2.128).
const SE = 'function se(){H=!0;try{let _=J()[i.__abort_handler.value/4];_&&i.__wbindgen_export.get(_)()}catch{}}';
const C = 'function c(_){throw _ instanceof WebAssembly.Exception&&_.is(q)?_.getArg(q,0):(J()[B]=1,se(),_)}';
const S = 'function s(){if(B??=i.__instance_terminated.value/4,J()[B]){if(H||se(),U){M();return}throw new Error("Module terminated")}else U&&M()}';
const SHIM = `var q=new WebAssembly.Tag({parameters:["externref"]}),B,H=!1;${SE}${C}${S}function _e(_,e,t){let n;s();try{n=i.fetch(_,e,t)}catch(r){c(r)}return n}var U=!1;`;

/// The guard functions of `text`, alive over a stub instance. `hook` is what the registered
/// abort hook does when called through the function table.
function guard(text, hook) {
  const cut = re => (text.match(re) || [])[0] || assert.fail(`not in the text: ${re}`);
  const src = [cut(/function se\(\)\{.*?\}catch\{\}\}/), cut(/function c\(_\)\{.*?\}(?=function s)/), cut(/function s\(\)\{.*?\}else U&&M\(\)\}/)].join('');
  const mem = new Int32Array(64);
  mem[1] = 7; // __abort_handler at byte 4 holds table index 7
  const log = [];
  const i = {
    __instance_terminated: { value: 0 },
    __abort_handler: { value: 4 },
    __wbindgen_export: { get: idx => (idx === 7 ? hook : () => assert.fail(`table index ${idx}`)) },
  };
  const make = new Function('i', 'mem', 'q', 'console', `
    var B,H=!1,U=!1,reinits=0;const J=()=>mem;
    function M(){reinits++;mem.fill(0);mem[1]=7;H=!1;U=!1}
    ${src}
    return {s,c,se,get U(){return U},get H(){return H},get reinits(){return reinits},schedule(){U=!0}};`);
  const q = new WebAssembly.Tag({ parameters: ['externref'] });
  const g = make(i, mem, q, { error: (...a) => log.push(a.map(String).join(' ')) });
  return { g, mem, log, q };
}

test('the recorded shape is matched and rewritten; the result is idempotent', () => {
  const r = harden(SHIM);
  assert.equal(r.changed, true, r.refused);
  assert.deepEqual(r.names, { c: 'c', s: 's', J: 'J', B: 'B', se: 'se', H: 'H', U: 'U', M: 'M', inst: 'i', tag: 'q' });
  assert.ok(r.text.includes(MARK));
  assert.ok(!r.text.includes('"Module terminated"'), 'nothing throws the latched message any more');
  assert.equal(harden(r.text).changed, false);
  assert.equal(harden(r.text).text, r.text);
});

test('RED twin: the unpatched guard latches after one failed hook call and throws forever', () => {
  const { g } = guard(SHIM, () => { throw new Error('the hook call failed'); });
  assert.doesNotThrow(() => g.s(), 'a healthy instance passes the gate (and the gate learns the flag address, as every export call does first)');
  assert.throws(() => g.c(new Error('a trap')), /a trap/);
  assert.equal(g.H, true);
  assert.equal(g.U, false);
  for (let n = 0; n < 3; n++) assert.throws(() => g.s(), /^Error: Module terminated$/);
  assert.equal(g.reinits, 0, 'never reinitialised');
});

test('patched: a terminated instance whose hook failed is reinitialised; one named refusal, then the next call passes', () => {
  const { g, mem, log } = guard(harden(SHIM).text, () => { throw new Error('the hook call failed'); });
  assert.doesNotThrow(() => g.s());
  assert.throws(() => g.c(new Error('a trap')), /a trap/);
  assert.match(log[0], new RegExp(`${TERMINATED}.*a trap`), 'the original exception is logged');
  assert.equal(mem[0], 1, 'the flag is set as before');
  assert.throws(() => g.s(), new RegExp(REINIT));
  assert.equal(g.reinits, 1);
  assert.match(log[1], /scheduled no reinit; reinitialising anyway/);
  assert.doesNotThrow(() => g.s(), 'the fresh instance answers');
  assert.equal(g.reinits, 1, 'and is not rebuilt again');
});

test('patched: a hook that does schedule a reinit (workers-rs) behaves as upstream intended, with the one refusal', () => {
  const { g, log } = guard(harden(SHIM).text, () => g.schedule());
  assert.doesNotThrow(() => g.s());
  assert.throws(() => g.c(new WebAssembly.RuntimeError('unreachable')), /unreachable/);
  assert.equal(g.U, true);
  assert.throws(() => g.s(), new RegExp(REINIT));
  assert.match(log[1], /reinitialising the terminated wasm instance/);
  assert.doesNotThrow(() => g.s());
  assert.equal(g.reinits, 1);
});

test('a wasm-bindgen throw (the tagged exception, a Rust Err) terminates nothing, before and after', () => {
  for (const text of [SHIM, harden(SHIM).text]) {
    const { g, mem, q } = guard(text, () => assert.fail('the hook is not for a Rust Err'));
    assert.doesNotThrow(() => g.s());
    const payload = { rust: 'Err' };
    assert.throws(() => g.c(new WebAssembly.Exception(q, [payload])), e => e === payload);
    assert.equal(mem[0], 0, 'not terminated');
    assert.doesNotThrow(() => g.s());
  }
});

test('main: patches a file in place, is idempotent, and REFUSES a shim whose guard has moved', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'harden-shim-'));
  const f = path.join(dir, 'index.js');
  fs.writeFileSync(f, SHIM);
  const [out, err] = [[], []];
  assert.equal(main([f], l => out.push(l), l => err.push(l)), 0, err.join('\n'));
  assert.ok(fs.readFileSync(f, 'utf8').includes(MARK));
  assert.equal(main([f], l => out.push(l), l => err.push(l)), 0);
  assert.match(out.at(-1), /already hardened/);
  const moved = path.join(dir, 'moved.js');
  fs.writeFileSync(moved, SHIM.replace('throw new Error("Module terminated")', 'throw new Error("Instance terminated")'));
  assert.equal(main([moved], l => out.push(l), l => err.push(l)), 1);
  assert.match(err.at(-1), /REFUSED .*s\(\) MISSING/);
  assert.equal(fs.readFileSync(moved, 'utf8').includes(MARK), false, 'nothing written on a refusal');
  assert.equal(main(['--check', path.join(dir, 'absent.js')], l => out.push(l), l => err.push(l)), 2);
  assert.equal(main([], l => out.push(l), l => err.push(l)), 2);
});

test('the real built shim on this box, when present, is matched (the shape this script was written for)', () => {
  const built = '/root/.cache/dowiz-deploy/src/workers/api/build/index.js';
  if (!fs.existsSync(built)) return;
  const r = harden(fs.readFileSync(built, 'utf8'));
  assert.ok(r.changed || r.why === 'already hardened', r.refused);
});

void fileURLToPath;
