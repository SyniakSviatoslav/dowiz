#!/usr/bin/env node
// SPDX-License-Identifier: MIT OR Apache-2.0
// bebop-verify (Node half): ONE FILE, NOTHING ELSE INSTALLED. The MIT reader wasm
// (crates/bebop-wasm-reader) is embedded below as base64 by tools/verify/build.sh; this file
// reads only the image named on its command line.
//
//   node bebop-verify.mjs <image> [--dump]
//
// Prints ONE line, the line every reader of docs/STATUS-CODES.md prints:
//   kv status=<s> n=<entries> root=<fold>      or      log status=<s> n=<records> root=<fold>
// and exits with the status (0 = read). The status in words goes to stderr. `--dump` prints
// every entry (`key = value`, bytes shown UTF-8 or as hex) or every record (oldest first).
//
// WHAT A GREEN LINE MEANS, EXACTLY (R-MAINGOAL-B §4): every object the root reaches hashes to
// the crc its header carries, the last commit hashes to its anchor, and the layout rules hold.
// It is NOT a proof that the content is true, nor (yet) a signature: no seal is in the format.
//
// Independence: oracle.py in crates/bebop-wasm is a second reader from the bytes alone; this
// file runs the library's rules compiled to wasm. Where the two differ, conformance.sh is red.
import { readFileSync } from "node:fs";

const WASM_B64 = "__WASM_BASE64__";
const WASM_SHA256 = "__WASM_SHA256__";

const [, , imagePath, flag] = process.argv;
if (!imagePath || imagePath === "--help") {
  console.error("usage: node bebop-verify.mjs <image> [--dump]   (reader wasm sha256 " + WASM_SHA256 + ")");
  process.exit(2);
}
const { instance } = await WebAssembly.instantiate(Buffer.from(WASM_B64, "base64"), {});
const w = instance.exports;
if (w.bw_version() !== 1) {
  console.error(`abi version ${w.bw_version()} is not the 1 this file was written for`);
  process.exit(3);
}
const mem = () => new Uint8Array(w.memory.buffer);
const take = (call) => {
  // a call that writes bytes: ask with a small buffer, grow to what it needs, re-read memory
  let cap = 256;
  for (;;) {
    const p = w.bw_alloc(cap);
    const n = Number(call(p, cap));
    if (n < 0) { w.bw_free(p, cap); return n; }
    if (n <= cap) { const out = mem().slice(p, p + n); w.bw_free(p, cap); return out; }
    w.bw_free(p, cap); cap = n;
  }
};
const words = (code) => {
  const t = take((p, c) => w.bw_status_text(code, p, c));
  return typeof t === "number" ? `status ${code}` : Buffer.from(t).toString("utf8");
};

const image = readFileSync(imagePath);
const ptr = w.bw_alloc(image.length);
mem().set(image, ptr);
const status = w.bw_open(ptr, image.length);
w.bw_free(ptr, image.length);
if (status !== 0) {
  console.log(`kv status=${status} n=0 root=0`);
  console.error(words(status));
  process.exit(status);
}
const kind = w.bw_kind();
const family = kind === 16 ? "log" : "kv";
const n = w.bw_len();
const rp = w.bw_alloc(8);
w.bw_root(rp);
const root = new BigInt64Array(w.memory.buffer, rp, 1)[0];
w.bw_free(rp, 8);
console.log(`${family} status=0 n=${n} root=${root}`);
console.error(`${words(0)}; ${family === "log" ? "EvLog" : "KV v" + kind}; verify(whole arena) = ${words(w.bw_verify(2))}`);

const show = (b) => {
  const s = Buffer.from(b).toString("utf8");
  return /^[\x20-\x7e]*$/.test(s) ? JSON.stringify(s) : "0x" + Buffer.from(b).toString("hex");
};
const u64 = (b, at) => new DataView(b.buffer, b.byteOffset + at, 8).getBigUint64(0, true);
if (flag === "--dump") {
  if (family === "kv") {
    for (let i = 0; ; i++) {
      const f = take((p, c) => w.bw_scan_next(BigInt(i), p, c));
      if (typeof f === "number" || f.length === 0) break;
      const kl = Number(u64(f, 0));
      const vl = Number(u64(f, 8 + kl));
      console.log(`${show(f.subarray(8, 8 + kl))} = ${show(f.subarray(16 + kl, 16 + kl + vl))}`);
    }
  } else {
    for (let i = 0; i < Number(n); i++) {
      const f = take((p, c) => w.bw_log_at(BigInt(i), p, c));
      if (typeof f === "number") break;
      const plen = Number(u64(f, 8));
      console.log(`#${i} seq=${u64(f, 0)} id=${Buffer.from(f.subarray(16, 48)).toString("hex")} payload=${show(f.subarray(112, 112 + plen))}`);
    }
  }
}
process.exit(0);
