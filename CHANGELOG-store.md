<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Changelog: the bebop store library

The crates `bebop-image`, `bebop-kv`, `bebop-log`, `bebop-io`, `bebop-store` and `bebop-ffi`.
Format changes are listed first in any entry that has them, because a format change must update
every reader in the same change.

## Unreleased (0.1.0)

### Added
- **bebop-ffi** (W-ES-P6): a C ABI (cdylib) over `Db` / `ReadTxn` / `WriteTxn` / `Scan`, with
  `Refusal` as integer codes plus a message. The codes it shares with bebop-wasm keep their
  numbers.
- **bindings/python/bebop_store.py**: a ctypes module, and a test that writes, reopens, reads
  and checks refusals.
- **tools/gates/ffi-readers.sh**: the fifth reader (T-ES-01). The C ABI reads the bebop-wasm
  fixtures (`kv`, `kv2`, `kv3`, `ptaway`, `kv4stub`) against their `.expected`.
- `examples/kv.rs`, `examples/kv.py`, and a README for each crate.
- **bebop-io** (W-ES-P3): `Db`, `ReadTxn`, `WriteTxn`, `Scan`, `Refusal`, `Options`, `Verify`,
  `OnTorn`, and the `Detached` device. **bebop-kv**: `Batch`, `Staged`, `MergeCursor`, `Bytes`,
  `Limits`, and `KeyOp` byte-key deltas.
- **bebop-kv** (W-ES-P8A): `KvError::UnknownVersion`. A KV root version this code does not know
  is refused before any of it is read (fixture `kv4stub`).

### Changed
- W-ES-P1: `bebop-store` split into `bebop-image`, `bebop-kv`, `bebop-log` and `bebop-io`.
  `bebop-store` stays as a re-exporting facade, so existing paths keep working.

### Format
- W-ES-P0: superblock cell 9 `anc` anchors the last commit (`0` = no anchor, as in every image
  written before it).
- v3 KV root (W-DELTA, 2026-10-06): a v2 base plus a delta chain.
- v2 KV / EvLog roots (DG3, 2026-09-28): bytes packed eight to a cell.
