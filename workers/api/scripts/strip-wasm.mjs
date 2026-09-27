#!/usr/bin/env node
// THE POST-GLUE STRIP (docs/research/2026-09-27-binary-size.md §4 row 2).
//
//   node scripts/strip-wasm.mjs build/index_bg.wasm
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

if (import.meta.url === `file://${process.argv[1]}`) {
  const file = process.argv[2];
  try {
    const before = fs.readFileSync(file);
    const { out, dropped } = strip(before);
    fs.writeFileSync(file, out);
    console.log(`strip-wasm: ${file} ${before.length} -> ${out.length} B (dropped ${dropped.map(d => `${d.name} ${d.bytes}`).join(', ') || 'nothing'})`);
  } catch (e) {
    console.error(`strip-wasm: ${file}: ${e.message}`);
    process.exit(1);
  }
}
