#!/usr/bin/env node
// SPDX-License-Identifier: MIT OR Apache-2.0
// A bebop ARCHIVE read by the reader EMBEDDED IN IT. Node 22, no npm, nothing installed, no repo
// (W-MG-M5, R-MAINGOAL-A §5.2 bootstrap, §5.3 trust):   node examples/read.mjs <archive.store>
// Prints the embedded reader's size and sha256 (the file's claim beside the recomputed digest --
// TESTIMONY: compare it with docs/FORMAT-BEBOP-IMAGE-v2.md §9.5 and the release, never trust a
// file about itself), then heads, records, the verify counters and every status in words.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
const b = readFileSync(process.argv[2] ?? (console.error("usage: node read.mjs <archive.store>"), process.exit(2)));
const cell = (i) => new DataView(b.buffer, b.byteOffset + 8 * i, 8).getBigInt64(0, true);
// THE BOOTSTRAP, six lines (FORMAT §9.5): cell 1024 is SPEC (h0's high half = 849619745); its payload cell 1 is a ref to the WASM blob
if ((BigInt.asUintN(64, cell(1024)) >> 32n) !== 849619745n) { console.error("REFUSED: cell 1024 is not a SPEC -- not a self-describing archive (a working image has none)"); process.exit(2); }
const w = 1024 + Number(cell(1027)), len = Number(cell(w + 2)), wasm = b.subarray((w + 3) * 8, (w + 3) * 8 + len);
const claimed = Buffer.from(b.subarray(1029 * 8, 1033 * 8)).toString("hex"), got = createHash("sha256").update(wasm).digest("hex");
if (got !== claimed) { console.error(`REFUSED: the embedded reader hashes to ${got}, the file claims ${claimed} -- not run`); process.exit(4); }
console.log(`embedded reader: ${len} bytes, sha256 ${got} (the file's claim agrees; TESTIMONY, not authority -- compare with FORMAT §9.5)`);
const x = (await WebAssembly.instantiate(wasm, {})).instance.exports, mem = () => new Uint8Array(x.memory.buffer);
const take = (call) => { for (let cap = 256; ; ) { const p = x.bw_alloc(cap), n = Number(call(p, cap)); if (n < 0 || n <= cap) { const o = n < 0 ? n : mem().slice(p, p + n); x.bw_free(p, cap); return o; } x.bw_free(p, cap); cap = n; } };
const words = (c) => { const t = take((p, n) => x.bw_status_text(c, p, n)); return typeof t === "number" ? `status ${c}` : Buffer.from(t).toString(); };
const hex = (u) => Buffer.from(u).toString("hex"), u64 = (u, at) => new DataView(u.buffer, u.byteOffset + at, 8).getBigUint64(0, true);
const p = x.bw_alloc(b.length); mem().set(b, p); const st = x.bw_open(p, b.length); x.bw_free(p, b.length);
if (st !== 0) { console.log(`doc status=${st} n=0 root=0`); console.error(words(st)); process.exit(st); }
if (x.bw_kind() !== 5) { console.error(`REFUSED: the root is not a DOC (kind ${x.bw_kind()})`); process.exit(2); }
const rp = x.bw_alloc(8); x.bw_root(rp); const root = new BigInt64Array(x.memory.buffer, rp, 1)[0]; x.bw_free(rp, 8);
console.log(`doc status=0 n=${x.bw_len()} root=${root} -- ${words(0)}`);
const h = take((p, n) => x.bw_heads(p, n));
for (let i = 0; i + 72 <= h.length; i += 72) console.log(`head actor=${hex(h.subarray(i, i + 32))} seq=${u64(h, i + 32)} tip=${hex(h.subarray(i + 40, i + 72))}`);
const show = (u) => { const s = Buffer.from(u).toString("latin1"); return /^[\x20-\x7e]*$/.test(s) ? JSON.stringify(s) : "0x" + hex(u); };
for (let i = 0; i < x.bw_len(); i++) { const f = take((p, n) => x.bw_log_at(BigInt(i), p, n)), l = Number(u64(f, 8)); console.log(`#${i} seq=${u64(f, 0)} id=${hex(f.subarray(16, 48))} payload=${show(f.subarray(112, 112 + l))}`); }
const v = x.bw_verify(2), c = take((p, n) => x.bw_counts(p, n));
if (typeof c === "number") { console.log(`verify status=${-c} -- ${words(-c)}`); process.exit(-c); }
const g = (i) => new DataView(c.buffer, c.byteOffset + 8 * i, 8).getBigInt64(0, true);
console.log(`verify(whole arena) = ${words(v)}`);
console.log(`verify records=${g(0)} chained=${g(1)} legacy=${g(2)} redacted=${g(3)} broken=${g(4)} forks=${g(6)} seals=${g(5)} checked=${g(10)} heads=${g(7)} keys=${g(8)} admitted=${g(9)} segments=${g(12)} cut=${g(13)} cut_redacted=${g(14)} declared=${g(15)}`);
if (g(10) === 0n) console.log(Buffer.from(take((p, n) => x.bw_unchecked_text(p, n))).toString());
const an = take((p, n) => x.bw_anchors(p, n)), a64 = (at) => new DataView(an.buffer, an.byteOffset + at, 8).getBigInt64(0, true);
for (let i = 0; typeof an !== "number" && i + 56 <= an.length; i += 56) console.log(`anchor kind=${a64(i)} over=${hex(an.subarray(i + 8, i + 40))} time_ms=${a64(i + 40)} proof_len=${a64(i + 48)} (present, NOT verified by this reader)`);
