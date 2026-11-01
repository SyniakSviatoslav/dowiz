#!/usr/bin/env node
// SPDX-License-Identifier: MIT OR Apache-2.0
// bebop-verify (Node half): ONE FILE, NOTHING ELSE INSTALLED. The MIT reader wasm
// (crates/bebop-wasm-reader) is embedded below as base64 by tools/verify/build.sh; this file
// reads only the image named on its command line.
//
//   node bebop-verify.mjs <image> [--dump] [--verify]
//
// Prints ONE line, the line every reader of docs/STATUS-CODES.md prints:
//   kv status=<s> n=<entries> root=<fold>   log status=<s> n=<records> root=<fold>
//   doc status=<s> n=<records> root=<log fold>                       (W-MG-M5, FORMAT §9.4)
// and exits with the status (0 = read). The status in words goes to stderr. `--dump` prints
// every entry (`key = value`, bytes shown UTF-8 or as hex), or every record (oldest first) and,
// for a DOC, its heads first. `--verify` prints a DOC's counters on stdout:
//   verify records= chained= legacy= redacted= broken= forks= seals= checked= heads= keys= genesis= segments= cut= cut_redacted= declared= keys_disagree= unsealed= merges= merges_bad=
//   (or `verify status=<s>` when the counters are refused, e.g. 75 CUT_UNDECLARED), then one `anchor kind= over= time_ms=
//   proof_len= (present, NOT verified by this reader)` line per time anchor the file carries
//
// WHAT A GREEN LINE MEANS, EXACTLY (R-MAINGOAL-B §4): every object the root reaches hashes to
// the crc its header carries, the last commit hashes to its anchor, and the layout rules hold.
// It is NOT a proof that the content is true. For a DOC, `checked=0` means what it says: this
// reader carries no signature algorithm, so seals are COUNTED, never checked (§5.3).
//
// Independence: oracle.py in crates/bebop-wasm is a second reader from the bytes alone; this
// file runs the library's rules compiled to wasm. Where the two differ, conformance.sh is red.
import { readFileSync } from "node:fs";

const WASM_B64 = "__WASM_BASE64__";
const WASM_SHA256 = "__WASM_SHA256__";

// W-MG-HOST: whatever goes wrong below is a status line and a code, never a stack trace; every word
// that names something from outside (a path) passes `clean`. 78 = HOST_FAILED (read.mjs's number).
const clean = (t) => String(t).replace(/[\x00-\x1f\x7f-\x9f\u2028\u2029]/g, (c) => "\\x" + c.charCodeAt(0).toString(16).padStart(2, "0"));
const failed = (e) => {
  console.log("kv status=78 n=0 root=0");
  console.error(`HOST_FAILED: the file could not be read, or the reader trapped; no verdict (${clean(e?.message ?? e)})`);
  process.exit(78);
};
process.on("uncaughtException", failed);
process.on("unhandledRejection", failed);

const args = process.argv.slice(2);
const imagePath = args.find((a) => !a.startsWith("--"));
if (!imagePath || args.includes("--help")) {
  console.error("usage: node bebop-verify.mjs <image> [--dump] [--verify]   (reader wasm sha256 " + WASM_SHA256 + ")");
  process.exit(2);
}
const { instance } = await WebAssembly.instantiate(Buffer.from(WASM_B64, "base64"), {});
const w = instance.exports;
if (w.bw_version() !== 2) {
  console.error(`abi version ${w.bw_version()} is not the 2 this file was written for`);
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
const text = (b) => Buffer.from(b).toString("utf8");
const words = (code) => {
  const t = take((p, c) => w.bw_status_text(code, p, c));
  return typeof t === "number" ? `status ${code}` : text(t);
};
const hex = (b) => Buffer.from(b).toString("hex");
const u64 = (b, at) => new DataView(b.buffer, b.byteOffset + at, 8).getBigUint64(0, true);
const i64 = (b, at) => new DataView(b.buffer, b.byteOffset + at, 8).getBigInt64(0, true);
// Every cell that becomes a JS Number: a length inside the bytes it came with, or a refusal.
const num = (v, max) => {
  if (v < 0n || v > BigInt(max)) throw new Error("a length past the bytes the reader returned");
  return Number(v);
};

const image = readFileSync(imagePath);
// One byte of room at least: `bw_alloc(0)` is null, and an empty file is status 1 as on every reader, not NULL_ARG.
const room = Math.max(image.length, 1);
const ptr = w.bw_alloc(room);
mem().set(image, ptr);
const status = w.bw_open(ptr, image.length);
w.bw_free(ptr, room);
if (status !== 0) {
  console.log(`kv status=${status} n=0 root=0`);
  console.error(words(status));
  process.exit(status);
}
const kind = w.bw_kind();
const family = kind === 16 ? "log" : kind === 5 ? "doc" : "kv";
const n = w.bw_len();
const rp = w.bw_alloc(8);
w.bw_root(rp);
const root = new BigInt64Array(w.memory.buffer, rp, 1)[0];
w.bw_free(rp, 8);
console.log(`${family} status=0 n=${n} root=${root}`);
const what = family === "log" ? "EvLog" : family === "doc" ? "DOC (root-version 5)" : "KV v" + kind;
const v2 = w.bw_verify(2);
console.error(`${words(0)}; ${what}; verify(whole arena) = ${words(v2)}`);

if (family === "doc") {
  const say = args.includes("--verify") ? console.log : console.error;
  const c = take((p, cap) => w.bw_counts(p, cap));
  if (typeof c === "number") { say(`verify status=${-c}`); console.error(words(-c)); process.exit(-c); }
  // 20 cells (ABI 2): the twelve M1 counters, then segments cut cut_redacted declared (W-MG-M5b), keys_disagree (W-MG-REVOKE), unsealed (W-MG-SEAL), merges merges_bad (W-MG-M2) appended
  const v = Array.from({ length: c.length / 8 }, (_, i) => i64(c, 8 * i));
  say(`verify records=${v[0]} chained=${v[1]} legacy=${v[2]} redacted=${v[3]} broken=${v[4]} forks=${v[6]} seals=${v[5]} checked=${v[10]} heads=${v[7]} keys=${v[8]} genesis=${v[9]} segments=${v[12]} cut=${v[13]} cut_redacted=${v[14]} declared=${v[15]} keys_disagree=${v[16]} unsealed=${v[17]} merges=${v[18]} merges_bad=${v[19]}`);
  if (v[10] === 0n) console.error(text(take((p, cap) => w.bw_unchecked_text(p, cap))));
  const an = take((p, cap) => w.bw_anchors(p, cap));
  for (let i = 0; typeof an !== "number" && i + 56 <= an.length; i += 56) {
    say(`anchor kind=${i64(an, i)} over=${hex(an.subarray(i + 8, i + 40))} time_ms=${i64(an, i + 40)} proof_len=${i64(an, i + 48)} (present, NOT verified by this reader)`);
  }
  const s = take((p, cap) => w.bw_spec(p, cap));
  if (typeof s !== "number" && s.length >= 40) {
    console.error(`SPEC: embedded reader abi ${i64(s, 0)} sha256 ${hex(s.subarray(8, 40))} (the file's CLAIM; this file's own reader is ${WASM_SHA256}), format text ${s.length - 40} bytes`);
  } else console.error("SPEC: none (a working image; archives carry one)");
}

const show = (b) => {
  const s = text(b);
  return /^[\x20-\x7e]*$/.test(s) ? JSON.stringify(s) : "0x" + hex(b);
};
if (args.includes("--dump")) {
  if (family === "kv") {
    for (let i = 0; ; i++) {
      const f = take((p, c) => w.bw_scan_next(BigInt(i), p, c));
      if (typeof f === "number" || f.length === 0) break;
      const kl = num(u64(f, 0), f.length);
      const vl = num(u64(f, 8 + kl), f.length);
      console.log(`${show(f.subarray(8, 8 + kl))} = ${show(f.subarray(16 + kl, 16 + kl + vl))}`);
    }
  } else {
    const h = take((p, c) => w.bw_heads(p, c));
    for (let i = 0; typeof h !== "number" && i + 72 <= h.length; i += 72) {
      console.log(`head actor=${hex(h.subarray(i, i + 32))} seq=${u64(h, i + 32)} tip=${hex(h.subarray(i + 40, i + 72))}`);
    }
    for (let i = 0; i < num(n, image.length); i++) {
      const f = take((p, c) => w.bw_log_at(BigInt(i), p, c));
      if (typeof f === "number") break;
      const plen = num(u64(f, 8), f.length);
      console.log(`#${i} seq=${u64(f, 0)} id=${hex(f.subarray(16, 48))} payload=${show(f.subarray(112, 112 + plen))}`);
    }
  }
}
process.exit(0);
