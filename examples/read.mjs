#!/usr/bin/env node
// SPDX-License-Identifier: MIT OR Apache-2.0
// A bebop ARCHIVE read by the reader EMBEDDED IN IT. Node 22, no npm, nothing installed, no repo
// (W-MG-M5, R-MAINGOAL-A §5.2 bootstrap, §5.3 trust):   node read.mjs <archive.store>
// THE FILE IS NOT TRUSTED (W-MG-HOST). Its sha256 claim only says the file agrees with itself, so a
// reader runs as a verifier only if it hashes to PINNED below. Any other reader is UNPINNED and is
// not run, unless `--run-files-own-code`: then every line says the verdict is the file's own code
// (TESTIMONY, §5.3) and the exit code is never 0. Either way the module has no imports, no start
// section, one bounded memory, and runs in a worker under a clock; no byte of it is ever printed:
// the words below are this file's, and every line passes `clean`.
// PINNED is written by tools/gates/atk3_pin.py from tools/verify/bebop-verify.mjs; tools/gates/atk3.sh holds them equal.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { crc32 } from "node:zlib";
import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";
const PINNED = "5a8b5cae863fb9dcd717ff679da933103c53b7422485b6cfed70ab43aa6b5fa3";
const MAX_PAGES = 16384, UNPINNED = 76, MODULE_REFUSED = 77, HOST_FAILED = 78, OWN = "--run-files-own-code", MARK = "[file's own code: testimony, not a verify] ";
const WORDS = { // crates/bebop-wasm-reader/src/status.rs TABLE, word for word (tools/gates/atk3.sh), then the host's three
  0: "OK: read; n = entries or records, root = the FNV-1a 64 fold (a DOC's root is its log's fold)",
  1: "NO_SUPERBLOCK: no valid superblock, or the live one was refused structurally (arena_used or PartTab outside, PartTab misplaced or its crc bad, root outside)",
  2: "NOT_A_KV: a superblock but no KV root, or a KV claim (a slice, a chain, a v4 rule) that does not hold, or a DOC whose SPEC or v1-v3 TREE this reader does not read",
  3: "NOT_A_LOG: asked for the log of an image whose root is not a log or DOC root",
  4: "TRUNCATED: the log's root claims more records than its chain delivers, or the chain never ends",
  5: "NULL_ARG: a null pointer at the ABI",
  10: "BAD_CRC: an object the read reached fails its crc (root, an array, a delta record, a node, a value, a log record, HEADS, KEYS, SPEC, an object of a torn last commit)",
  12: "UNKNOWN_VERSION: the KV root names a version above 4 and is not a DOC (root-version 5 in both version cells)",
  41: "LAST_COMMIT: the last commit's range does not hash to its anchor and every object in it checks (a re-sealed object, a changed header); never fallen back past",
  70: "NOT_OPEN: no image is open (bw_open was not called, or refused)",
  71: "NOT_FOUND: no entry under that key",
  73: "OUT_OF_RANGE: the index is past the last record",
  74: "NOT_A_DOC: a DOC call (bw_counts) against a KV or a log",
  75: "CUT_UNDECLARED: a log segment left this file (a SEG under the DOC) and no TRUNCATE record on the kept chain names it by root and range -- records were dropped without the cut being declared",
  [UNPINNED]: "UNPINNED: the embedded reader is not the one this read.mjs pins; nothing it says is a verify",
  [MODULE_REFUSED]: "MODULE_REFUSED: the embedded module has an import, a start section, or no single bounded memory; not instantiated",
  [HOST_FAILED]: "HOST_FAILED: the file could not be read, or the reader trapped or ran out of its clock or memory; no verdict",
};
const UNCHECKED = "seals COUNTED, not CHECKED: this reader carries no signature algorithm (MIT, no ML-DSA); a count is not a proof -- bring a verifier to the same seam, or the format text's independent readers";
if (!isMainThread) { // THE WORKER: everything the module says comes back as bytes and numbers; only the main thread prints
  const { wasm, image } = workerData, x = (await WebAssembly.instantiate(wasm, {})).instance.exports, mem = () => new Uint8Array(x.memory.buffer);
  const take = (call) => { for (let cap = 256; ; ) { const p = x.bw_alloc(cap), n = Number(call(p, cap)); if (n > image.length * 8 + 4096) throw new Error("the reader asks for more than the file holds"); if (n < 0 || n <= cap) { const o = n < 0 ? n : mem().slice(p, p + n); x.bw_free(p, cap); return o; } x.bw_free(p, cap); cap = n; } };
  const p = x.bw_alloc(Math.max(image.length, 1)); mem().set(image, p); const st = x.bw_open(p, image.length); x.bw_free(p, Math.max(image.length, 1));
  const r = { st, kind: st === 0 ? x.bw_kind() : 0 };
  if (st === 0 && r.kind === 5) {
    const rp = x.bw_alloc(8); x.bw_root(rp); r.root = new DataView(x.memory.buffer).getBigInt64(rp, true); x.bw_free(rp, 8);
    r.n = x.bw_len(); r.heads = take((p, n) => x.bw_heads(p, n)); r.recs = [];
    if (r.n > BigInt(image.length)) throw new Error("the reader claims more records than the file has bytes");
    for (let i = 0n; i < r.n; i++) r.recs.push(take((p, n) => x.bw_log_at(i, p, n)));
    r.v = x.bw_verify(2); r.counts = take((p, n) => x.bw_counts(p, n)); r.anchors = take((p, n) => x.bw_anchors(p, n));
  }
  parentPort.postMessage(r);
} else {
  const args = process.argv.slice(2), path = args.find((a) => !a.startsWith("--")), own = args.includes(OWN), limit = Number((args.find((a) => a.startsWith("--limit-s=")) ?? "=60").split("=")[1]) * 1000;
  const clean = (s) => String(s).replace(/[\x00-\x1f\x7f-\x9f\u2028\u2029]/g, (c) => "\\x" + c.charCodeAt(0).toString(16).padStart(2, "0"));
  const out = (s) => console.log((own ? MARK : "") + clean(s)), note = (s) => console.error((own ? MARK : "") + clean(s));
  const hex = (u) => Buffer.from(u).toString("hex"), words = (c) => WORDS[c] ?? `status ${c}`;
  const stop = (code, why) => { out(`doc status=${code} n=0 root=0`); note(why ?? words(code)); process.exit(own && code ? UNPINNED : code); };
  const num = (v, max) => { if (v < 0n || v > BigInt(max)) stop(2, "REFUSED: a cell names a place outside the file"); return Number(v); }; // every cell that becomes a Number
  const sections = (w) => { // the module's outline, no parser but this: why it is refused, or null
    if (w.length < 8 || hex(w.subarray(0, 8)) !== "0061736d01000000") return "not a wasm 1 module";
    const leb = (at) => { let v = 0, s = 1; for (let i = 0; i < 5; i++, s *= 128) { const b = w[at.i++]; if (b === undefined) return NaN; v += (b & 127) * s; if (b < 128) return v; } return NaN; };
    let memories = 0;
    for (const at = { i: 8 }; at.i < w.length; ) {
      const id = w[at.i++], size = leb(at), body = at.i;
      if (!(size <= w.length - body) || id > 12) return `section ${id} does not read`;
      if (id === 2 && leb(at) !== 0) return "it imports";
      if (id === 8) return "it has a start section";
      if (id === 5) { const n = leb(at); memories += n; if (n !== 1 || w[at.i++] !== 1) return "its memory is not one unshared memory with a maximum"; leb(at); if (!(leb(at) <= MAX_PAGES)) return `its memory maximum is past ${MAX_PAGES} pages`; }
      at.i = body + size;
    }
    return memories === 1 ? null : "it has no memory";
  };
  try {
    if (!path) { console.error(`usage: node read.mjs <archive.store> [${OWN}]`); process.exit(2); }
    const b = readFileSync(path), N = Math.floor(b.length / 8), cell = (i) => (i >= 0 && i < N ? b.readBigInt64LE(8 * i) : 0n);
    // THE BOOTSTRAP, six lines (FORMAT §9.5): cell 1024 is SPEC (h0's high half = 849619745); its payload cell 1 is a ref to the WASM blob
    if ((BigInt.asUintN(64, cell(1024)) >> 32n) !== 849619745n) stop(2, "REFUSED: cell 1024 is not a SPEC -- not a self-describing archive (a working image has none)");
    const w = 1024 + num(cell(1027), N), len = num(cell(w + 2), b.length), wasm = b.subarray((w + 3) * 8, (w + 3) * 8 + len);
    const claimed = hex(b.subarray(1029 * 8, 1033 * 8)), got = createHash("sha256").update(wasm).digest("hex");
    if (got !== claimed) { out("doc status=4 n=0 root=0"); note(`REFUSED: the embedded reader hashes to ${got}, the file claims ${claimed} -- not run`); process.exit(4); }
    const bad = sections(wasm); if (bad) stop(MODULE_REFUSED, `${words(MODULE_REFUSED)} (${bad})`);
    // The live superblock (FORMAT §3.1) names the DOC, and the DOC must name cell 1024: a file does not get to run a reader it does not declare
    const live = (at) => cell(at) === 3554557610294396226n && crc32(b.subarray(8 * at, 8 * (at + 15))) === Number(BigInt.asUintN(32, cell(at + 15)));
    const at = live(512) && (!live(0) || cell(514) > cell(2)) ? 512 : 0, doc = num(cell(num(cell(at + 3), N) + 18), N);
    if (BigInt.asUintN(64, cell(doc)) >> 32n !== 2619226223n || BigInt(doc) + cell(doc + 8) !== 1024n) stop(2, "REFUSED: the DOC does not name the SPEC at cell 1024 -- the embedded reader is not the one the file declares");
    if (got !== PINNED && !own) { out(`doc status=${UNPINNED} n=0 root=0`); note(`UNPINNED: the embedded reader is sha256 ${got}, this read.mjs pins ${PINNED} -- not run. The file agreeing with itself is not a verify; ${OWN} runs it as testimony`); process.exit(UNPINNED); }
    out(`embedded reader: ${len} bytes, sha256 ${got} (${got === PINNED ? "the reader this read.mjs pins" : "UNPINNED: the file's own code"}; TESTIMONY, not authority -- compare with FORMAT §9.5)`);
    const r = await new Promise((yes, no) => {
      const t = new Worker(new URL(import.meta.url), { workerData: { wasm, image: b }, resourceLimits: { maxOldGenerationSizeMb: 128, maxYoungGenerationSizeMb: 32, stackSizeMb: 4 } });
      const clock = setTimeout(() => { no(new Error(`the reader ran past ${limit / 1000} s`)); t.terminate(); }, limit);
      t.once("message", (m) => { clearTimeout(clock); t.terminate(); yes(m); }); t.once("error", (e) => { clearTimeout(clock); no(e); });
    });
    if (r.st !== 0) stop(r.st);
    if (r.kind !== 5) stop(2, `REFUSED: the root is not a DOC (kind ${r.kind})`);
    out(`doc status=0 n=${r.n} root=${r.root} -- ${words(0)}`);
    const u64 = (u, at) => new DataView(u.buffer, u.byteOffset + at, 8).getBigUint64(0, true), show = (u) => { const s = Buffer.from(u).toString("latin1"); return /^[\x20-\x7e]*$/.test(s) ? JSON.stringify(s) : "0x" + hex(u); };
    for (let i = 0; i + 72 <= r.heads.length; i += 72) out(`head actor=${hex(r.heads.subarray(i, i + 32))} seq=${u64(r.heads, i + 32)} tip=${hex(r.heads.subarray(i + 40, i + 72))}`);
    r.recs.forEach((f, i) => out(`#${i} seq=${u64(f, 0)} id=${hex(f.subarray(16, 48))} payload=${show(f.subarray(112, 112 + num(u64(f, 8), f.length)))}`));
    const c = r.counts; if (typeof c === "number") { out(`verify status=${-c} -- ${words(-c)}`); process.exit(own ? UNPINNED : WORDS[-c] ? -c : HOST_FAILED); }
    const g = (i) => new DataView(c.buffer, c.byteOffset + 8 * i, 8).getBigInt64(0, true);
    out(`verify(whole arena) = ${words(r.v)}`);
    out(`verify records=${g(0)} chained=${g(1)} legacy=${g(2)} redacted=${g(3)} broken=${g(4)} forks=${g(6)} seals=${g(5)} checked=${g(10)} heads=${g(7)} keys=${g(8)} genesis=${g(9)} segments=${g(12)} cut=${g(13)} cut_redacted=${g(14)} declared=${g(15)} keys_disagree=${c.length > 128 ? g(16) : "absent"} unsealed=${c.length > 136 ? g(17) : "absent"}`);
    if (g(10) === 0n) out(UNCHECKED);
    const an = r.anchors, a64 = (at) => new DataView(an.buffer, an.byteOffset + at, 8).getBigInt64(0, true);
    for (let i = 0; typeof an !== "number" && i + 56 <= an.length; i += 56) out(`anchor kind=${a64(i)} over=${hex(an.subarray(i + 8, i + 40))} time_ms=${a64(i + 40)} proof_len=${a64(i + 48)} (present, NOT verified by this reader)`);
    process.exit(own ? UNPINNED : 0);
  } catch (e) { stop(HOST_FAILED, `${words(HOST_FAILED)} (${e?.message ?? "no message"})`); }
}
