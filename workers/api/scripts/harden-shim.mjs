#!/usr/bin/env node
// THE SHIM'S TERMINATION GUARD, HARDENED AFTER THE GLUE (W-EBILLS-POISON, incident 2026-11-04).
//
//   node scripts/harden-shim.mjs build/index.js          # patch in place, loud on any surprise
//   node scripts/harden-shim.mjs --check build/index.js  # say what it finds, change nothing
//
// WIRED into wrangler.toml's `[build] command` after strip-wasm.mjs, so every upload carries it.
//
// THE INCIDENT. From 08:15 to 08:31 UTC a quarter to a third of EVERY request on EVERY venue
// answered 500 "Module terminated" with cpuTime 0 (wrangler tail, 226 events, 1 exception text).
// That text is thrown by wasm-bindgen 0.2.128's export guard (`s()` below): once anything that is
// not a wasm-bindgen throw -- a trap, a Rust panic (panic=abort -> `unreachable`), a JS exception
// crossing wasm frames from an import not marked `catch` -- leaves a wasm call, `c()` marks the
// instance terminated (`__instance_terminated`), runs the abort hook ONCE (`se()`, latching `H`),
// and every later call throws unless the hook managed to ask for a reinit (`U`). workers-rs 0.8.5
// registers exactly such a hook (`init.rs`: set_on_abort -> schedule_reinit), and measured on this
// bundle in Node (scratchpad `shim-probe.mjs`) the machinery works: c() then s() reinitialises.
// So the production state -- flag set, hook latched, no reinit -- means the hook CALL itself failed
// (the catch in `se()` swallows it), after which the shim NEVER tries again: one failed hook call
// is a permanently poisoned isolate, with the original exception thrown into a microtask where no
// log ever shows it. worker-build's own recovery (`K()`: criticalError on a WebAssembly.RuntimeError
// caught by the entrypoint Proxy) never sees it either: the throw happens inside a promise callback
// of the Durable Object's alarm, not in the Proxy's call.
//
// THE CHANGE (two functions, matched by shape so a minifier's renaming does not matter, and REFUSED
// loudly if the shape has moved -- a worker-build upgrade must be looked at, not shipped unpatched):
//  * `c()`: before marking the instance terminated, console.error the ORIGINAL exception. The
//    trigger is then named in the event's logs instead of vanishing.
//  * `s()`: a terminated instance is ALWAYS reinitialised, whether or not the hook asked for it; the
//    call that found it terminated then throws a plain, named Error ONCE (so a stale closure of the
//    dead instance never runs on the new one), and the next call runs on the fresh instance. The
//    latch on `H` no longer decides anything. workers-rs's start hook bumps the instance id, so the
//    entrypoint Proxy rebuilds Durable Object wrappers as it does after its own reinit.
// A shim already carrying the patch is left as it is (idempotent), and the file is only written when
// both functions were found and rewritten.
import fs from 'node:fs';

export const MARK = '/*dowiz:hardened-guard*/';

// c(_): throw _ instanceof WebAssembly.Exception&&_.is(q)?_.getArg(q,0):(J()[B]=1,se(),_)
const C_RE = /function (\w+)\((\w+)\)\{throw \2 instanceof WebAssembly\.Exception&&\2\.is\((\w+)\)\?\2\.getArg\(\3,0\):\((\w+)\(\)\[(\w+)\]=1,(\w+)\(\),\2\)\}/;
// s(): if(B??=i.__instance_terminated.value/4,J()[B]){if(H||se(),U){M();return}throw new Error("Module terminated")}else U&&M()
const S_RE = /function (\w+)\(\)\{if\((\w+)\?\?=(\w+)\.__instance_terminated\.value\/4,(\w+)\(\)\[\2\]\)\{if\((\w+)\|\|(\w+)\(\),(\w+)\)\{(\w+)\(\);return\}throw new Error\("Module terminated"\)\}else \7&&\8\(\)\}/;

export const TERMINATED = 'Critical: the wasm instance was terminated by';
export const REINIT = 'the wasm instance was terminated and has been reinitialised; this call is refused, the next one runs on the fresh instance';

/// The hardened text, or a refusal naming what was not found. Pure.
export function harden(text) {
  if (text.includes(MARK)) return { text, changed: false, why: 'already hardened' };
  const c = text.match(C_RE);
  const s = text.match(S_RE);
  if (!c || !s) return { refused: `the termination guard was not found: c() ${c ? 'found' : 'MISSING'}, s() ${s ? 'found' : 'MISSING'}` };
  const [, cName, arg, tag, J, B, se] = c;
  const [, sName, B2, inst, J2, H, se2, U, M] = s;
  if (J !== J2 || B !== B2 || se !== se2) return { refused: `c() and s() do not share their globals (${J}/${J2}, ${B}/${B2}, ${se}/${se2})` };
  if (text.split(c[0]).length !== 2 || text.split(s[0]).length !== 2) return { refused: 'the guard appears more than once' };
  const cNew =
    `function ${cName}(${arg}){if(${arg} instanceof WebAssembly.Exception&&${arg}.is(${tag}))throw ${arg}.getArg(${tag},0);` +
    `console.error(${JSON.stringify(TERMINATED)},${arg});${J}()[${B}]=1;${se}();throw ${arg}}`;
  const sNew =
    `function ${sName}(){${MARK}if(${B}??=${inst}.__instance_terminated.value/4,${J}()[${B}]){${H}||${se}();` +
    `console.error("Critical",${U}?"reinitialising the terminated wasm instance":"the terminated wasm instance's abort hook scheduled no reinit; reinitialising anyway");` +
    `${M}();throw new Error(${JSON.stringify(REINIT)})}else ${U}&&${M}()}`;
  return { text: text.replace(c[0], cNew).replace(s[0], sNew), changed: true, names: { c: cName, s: sName, J, B, se, H, U, M, inst, tag } };
}

export function main(argv, log, err) {
  const check = argv.includes('--check');
  const file = argv.filter(a => a !== '--check')[0];
  if (!file) { err('usage: harden-shim.mjs [--check] build/index.js'); return 2; }
  let text;
  try { text = fs.readFileSync(file, 'utf8'); } catch (e) { err(`harden-shim: cannot read ${file}: ${e.message}`); return 2; }
  const r = harden(text);
  if (r.refused) { err(`harden-shim REFUSED ${file}: ${r.refused}. The generated guard has changed shape (wasm-bindgen/worker-build upgrade?): read it before shipping.`); return 1; }
  if (!r.changed) { log(`harden-shim: ${file} ${r.why}`); return 0; }
  log(`harden-shim: guard found (c=${r.names.c} s=${r.names.s} flag=${r.names.B} hook=${r.names.se} reinit=${r.names.M})${check ? ', not written (--check)' : ''}`);
  if (check) return 0;
  fs.writeFileSync(file, r.text);
  if (!harden(fs.readFileSync(file, 'utf8')).why) { err('harden-shim: the written file does not read back as hardened'); return 1; }
  log(`harden-shim: ${file} hardened (${text.length} -> ${r.text.length} bytes)`);
  return 0;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exit(main(process.argv.slice(2), console.log, console.error));
}
