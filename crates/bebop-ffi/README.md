<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-ffi

A C ABI over `bebop-io`'s database, built as a cdylib (`libbebop_ffi.so`). It is the one crate of
the store family that uses `unsafe`.

- Every function returns an `i32` status. `0` is OK. The reason for a refusal is read with
  `bebop_last_error`, and its name with `bebop_status_name`.
- Codes shared with bebop-wasm's reader ABI keep their numbers: 1 NO_SUPERBLOCK, 2 NOT_A_KV,
  5 NULL_ARG, 10 BAD_CRC, 12 UNKNOWN_VERSION. The rest are 40..60 and listed in `src/lib.rs`.
- Handles are `BebopDb`, `BebopRead`, `BebopWrite` and `BebopScan`. Closing a parent with an open
  child is refused with `BUSY` rather than leaving the child dangling.
- Panics are caught at the boundary (`PANIC`).

Python: `bindings/python/bebop_store.py` (ctypes, no build step beyond `cargo build --release`).
Test: `python3 bindings/python/test_bebop_store.py`. Fifth reader gate: `tools/gates/ffi-readers.sh`.

## Licence

MIT OR Apache-2.0.
