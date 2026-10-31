<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-wasm-reader

The bebop file read in wasm32: KV v1 to v4, EvLog v1 and v2, and the signed DOC root. No imports, no
I/O. The host hands it bytes and gets back entries, records, counters and status codes. A refusal is
a number from [`docs/STATUS-CODES.md`](../../docs/STATUS-CODES.md), never a trap.

It carries no signature algorithm, so it counts seals and does not check them, and it says so in
words through `bw_unchecked_text`. The ABI is in `src/abi.rs` and FORMAT §9.5.1.

`tools/verify/bebop-verify.mjs` is this module embedded in one Node file; `tools/verify/build.sh`
builds it. See [`crates/bebop-store/README.md`](../bebop-store/README.md) for what the file promises.

## Licence

MIT OR Apache-2.0.
