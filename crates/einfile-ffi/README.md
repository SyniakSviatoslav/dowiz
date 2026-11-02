<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# einfile-ffi

A C ABI over `einfile-io`'s database, built as a cdylib (`libeinfile_ffi.so`). It is the one crate of
the store family that uses `unsafe`.

- Every function returns an `i32` status. `0` is OK. The reason for a refusal is read with
  `einfile_last_error`, and its name with `einfile_status_name`.
- Codes shared with dowiz-hub-wasm's reader ABI keep their numbers: 1 NO_SUPERBLOCK, 2 NOT_A_KV,
  5 NULL_ARG, 10 BAD_CRC, 12 UNKNOWN_VERSION. The rest are 40..60 and listed in `src/lib.rs`.
- Handles are `EinfileDb`, `EinfileRead`, `EinfileWrite` and `EinfileScan`. Closing a parent with an open
  child is refused with `BUSY` rather than leaving the child dangling.
- Panics are caught at the boundary (`PANIC`).

Python: `bindings/python/einstore.py` (ctypes, no build step beyond `cargo build --release`).
Test: `python3 bindings/python/test_einstore.py`. Fifth reader gate: `tools/gates/ffi-readers.sh`.

## Licence

MIT OR Apache-2.0.
