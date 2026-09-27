#!/usr/bin/env node
// THE POST-GLUE STRIP (docs/research/2026-09-27-binary-size.md §4 row 2).
//
//   node scripts/strip-wasm.mjs build/index_bg.wasm --keep build/unstripped
//   node scripts/strip-wasm.mjs --name build/unstripped/index_bg.<digest>.wasm 4656
//
// WIRED 2026-09-27 (lane W-STRIP) into wrangler.toml's `[build] command`, so
// every `wrangler deploy` / `wrangler dev` uploads the stripped bundle.
//
// `--keep DIR` first writes the UNSTRIPPED bytes to
// `DIR/index_bg.<sha256 of the STRIPPED file, 16 hex>.wasm`: the name is the
// digest of what was uploaded, so a stack trace from a deployed build is
// symbolised against the copy whose stripped twin has that digest
// (`sha256sum build/index_bg.wasm | cut -c1-16`). worker-build replaces only
// the entries it produces, so DIR survives the next build. The copy is written
// BEFORE the strip, and a file with nothing to drop (a second run over the
// same build) keeps nothing, so DIR never holds a stripped file by mistake.
//
// Drops the `name` and `producers` custom sections from the bundle AFTER
// wasm-bindgen and wasm-opt have run, and writes it back in place. Nothing
// else changes: every other section is copied BYTE FOR BYTE, and the file is
// re-read and compared before it is written, so what the engine executes is
// the same bytes it was (the proof `strip-wasm.test.mjs` holds).
//
// WHY NOT `wasm-opt --strip-debug`: measured 2026-09-27 on the 4,859,159 B
// bundle, binaryen re-encodes the code section (+9,171 B) -- the same program,
// probably, but not a provably identical one. Dropping sections is.
//
// WHY IT IS SAFE FOR THE GLUE: `build/index.js` reads exports by name and never
// calls `WebAssembly.Module.customSections`; the panic recovery (Cargo.toml's
// `strip = false` comment) re-instantiates the SAME module from its exports.
// What is lost: Rust function names in a wasm stack trace in Workers Logs.
//
// Loud: a file that is not a module, or a section that runs past the end,
// exits 1 and writes nothing.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

export const DROP = ['name', 'producers'];

function leb(b, i) {
  let r = 0, s = 0, x;
  do {
    if (i >= b.length) throw new Error('truncated LEB128');
    x = b[i++];
    r += (x & 0x7f) * 2 ** s;
    s += 7;
  } while (x & 0x80);
  return [r, i];
}

/** The module's sections: `{ id, name, start, end }` (end exclusive, header included). */
export function sections(b) {
  if (b.length < 8 || b.readUInt32LE(0) !== 0x6d736100) throw new Error('not a wasm module');
  const out = [];
  let i = 8;
  while (i < b.length) {
    const start = i;
    const id = b[i];
    const [n, j] = leb(b, i + 1);
    if (j + n > b.length) throw new Error(`section ${id} at ${start} runs past the end`);
    let name = '';
    if (id === 0) {
      const [ln, k] = leb(b, j);
      name = b.subarray(k, k + ln).toString('utf8');
    }
    out.push({ id, name, start, end: j + n });
    i = j + n;
  }
  return out;
}

/** The bytes without the dropped sections, and what was dropped. */
export function strip(b) {
  const keep = [b.subarray(0, 8)];
  const dropped = [];
  for (const s of sections(b)) {
    if (s.id === 0 && DROP.includes(s.name)) dropped.push({ name: s.name, bytes: s.end - s.start });
    else keep.push(b.subarray(s.start, s.end));
  }
  const out = Buffer.concat(keep);
  // THE PROOF, before anything is written: every kept section is the same bytes.
  const body = x => sections(x).filter(s => !(s.id === 0 && DROP.includes(s.name))).map(s => x.subarray(s.start, s.end).toString('hex'));
  if (body(out).join() !== body(b).join()) throw new Error('a kept section changed');
  return { out, dropped };
}

/** The kept copy's file name: keyed by the digest of the bytes that ship. */
export const keepName = stripped => `index_bg.${crypto.createHash('sha256').update(stripped).digest('hex').slice(0, 16)}.wasm`;

/**
 * Function index -> name, from a module's `name` section (subsection 1). A stripped stack trace
 * reads `wasm-function[4656]:0x394b12`; the index and offset are the same in the kept copy, whose
 * name section says which Rust function that is.
 */
export function functionNames(b) {
  const sec = sections(b).find(s => s.id === 0 && s.name === 'name');
  const out = new Map();
  if (!sec) return out;
  let [, i] = leb(b, sec.start + 1);
  const [ln, k] = leb(b, i);
  i = k + ln;
  while (i < sec.end) {
    const kind = b[i];
    const [size, j] = leb(b, i + 1);
    if (kind === 1) {
      let [count, p] = leb(b, j);
      while (count--) {
        const [idx, q] = leb(b, p);
        const [nl, r] = leb(b, q);
        out.set(idx, b.subarray(r, r + nl).toString('utf8'));
        p = r + nl;
      }
    }
    i = j + size;
  }
  return out;
}

/** The command line. Returns the exit code; writes nothing on any error. */
export function main(argv, log, err) {
  if (argv[0] === '--name') {
    const [, kept, ...idx] = argv;
    if (!kept || !idx.length || idx.some(x => !/^\d+$/.test(x))) {
      err('usage: strip-wasm.mjs --name <kept.wasm> <function index>...');
      return 2;
    }
    try {
      const names = functionNames(fs.readFileSync(kept));
      for (const x of idx) log(`wasm-function[${x}] ${names.get(Number(x)) ?? '(no name)'}`);
      return 0;
    } catch (e) {
      err(`strip-wasm: ${kept}: ${e.message}`);
      return 1;
    }
  }
  const [file, flag, dir, ...rest] = argv;
  if (!file || (flag !== undefined && (flag !== '--keep' || !dir)) || rest.length) {
    err('usage: strip-wasm.mjs <file.wasm> [--keep <dir>]');
    return 2;
  }
  try {
    const before = fs.readFileSync(file);
    const { out, dropped } = strip(before);
    let kept = '';
    if (dir && dropped.length) {
      fs.mkdirSync(dir, { recursive: true });
      kept = path.join(dir, keepName(out));
      fs.writeFileSync(kept, before);
    }
    fs.writeFileSync(file, out);
    log(`strip-wasm: ${file} ${before.length} -> ${out.length} B (dropped ${dropped.map(d => `${d.name} ${d.bytes}`).join(', ') || 'nothing'})${kept ? `; unstripped kept at ${kept}` : ''}`);
    return 0;
  } catch (e) {
    err(`strip-wasm: ${file}: ${e.message}`);
    return 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exitCode = main(process.argv.slice(2), console.log, console.error);
}
