<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Changelog: the bebop store library

The crates `bebop-image`, `bebop-kv`, `bebop-log`, `bebop-io`, `bebop-store` and `bebop-ffi`.
Format changes are listed first in any entry that has them, because a format change must update
every reader in the same change.

## Unreleased (0.1.0)

### Added
- **Format** (W-MG-M1, FORMAT §9.4): root-version **5 = DOC** -- `DOC {FLAGS, ref LOG, ref TREE,
  ref HEADS, ref KEYS, 5, ref SPEC, 5, ref ANCH, 0}` over an EvLog v2 log, with `SEAL` (one
  ML-DSA-65 signature per device commit over the tip), `HEADS`, `KEY` / `KEYS` and the ADMIT /
  REVOKE record bodies. Old images are byte-identical; every reader written before refuses a DOC
  root by name (`UNKNOWN_VERSION` 12, fixture `doc` in `conformance.sh`).
- **Format** (W-MG-M1B, FORMAT §9.4): `ANCH {1, kind, over 4, time_ms, ref PREV_ANCH, proof_len,
  proof packed}`, the level-1 time anchors of a DOC (DOC cell 8 names the newest); kinds 1
  platform TSA mark, 2 OpenTimestamps, 3 Roughtime; proofs are opaque blobs, the adapters live
  in dowiz-hub `doc_seal/`. `Doc::anchor_bytes`, `doc::check_anchors` through the `AnchorCheck`
  seam (`Verdict::Ok / Bad / Unchecked` -- a kind without an adapter is never ok), `Doc::copy_bytes`
  (a DOC carried whole into another store, nothing re-signed), `Doc::init_over_bytes` (a DOC over
  an existing chain, verbatim). `doc::Ids` now sees the record's flags (`legacy(flags, prev,
  payload, id)`, `redacted(flags, ..)`, `body(flags, ..)`) and `verify` applies M0's rule inside a
  DOC: a legacy id newer than a chained record of its device is `broken`. `oracle.py --proj`
  refuses a log version above 2 by name (12) instead of walking it (4).
- **bebop-log** `doc`: `Doc::init_bytes` / `Doc::commit_bytes` (the writer), `doc::verify` with
  the counters `records / chained / legacy / redacted / broken / seals_ok / seals_bad / forks`,
  the seams `Signer` / `Verifier` / `Ids` (no signature or hash code in the MIT crates), and
  `EvLog::unknown_version` / `MAX_VERSION` for log readers. `EvLog::{version,len,tip,walk_until,
  chain_len,chain_crc}_at(root)` read a log that is not the image's root.
- **bebop-io** feature `doc`: `docverify::verify_doc(bytes, ..)` -- the last-commit rule, then
  the DOC counters. `Db` still refuses a DOC root (`Refusal::UnknownVersion(5)`) until M5.
- **Format** (W-MG-M3b, FORMAT §9.4): the SEG frontier row carries `scheme` (M0's forward-once
  rule across a cut; row 9 -> 10 cells, SEG digest 3288852188 -> 2192499951, no production image
  holds a SEG). A cut carries the anchors (`Carries("ANCH")` is gone); an anchor over a record that
  left under a segment is `under_cut` in `check_anchors` -- never ok, bad only on a failed proof,
  placed by the archive -- and `AnchorCounts.cut` counts them. `Doc::redact_into` is redaction in
  place for a DOC. dowiz-hub `SealedHub::forget` / `SealedHub::cut` commit the signed FORGOTTEN /
  TRUNCATE through the device's ML-DSA-65 seal (`KIND_DECL` records).
- **Format** (W-MG-M3/M4, FORMAT §9.4): DOC cell 9 is `ref SEG` (was reserved; DOC digest
  3669646107 -> 2619226223, no DOC image predates it). `SEG {VERSION, first, last, root, ref
  PREV_SEG, ref BLOB, redacted, declared, frontier rows}` -- an RFC 9162 root over the ids a cut
  took out of the log, and the record bodies `TRUNCATE` (names the segment, signed through the
  cutting device's seal) and `FORGOTTEN` (n redacted, subject). The log is kept history: records
  leave a DOC file only under a segment declared by a TRUNCATE; `verify` REFUSES a file whose
  segment nobody declared (`DocError::CutUndeclared`).
- **bebop-log** `doc`: `Doc::cut_into(old, fresh, keep, ids)` (the rebuild without the oldest
  records; `Segment::truncate_body` is what the caller then commits), `Ids::tree_root` (the seam
  for the root; `None` refuses the cut), counters `segments / cut / cut_redacted / declared`, a
  FORGOTTEN counts only under an admitted key's valid seal, `holds` requires `redacted +
  cut_redacted == declared`.
- **dowiz-hub** `rotate`: the checkpoint descriptor names `root=` -- the same RFC 9162 root over
  the archived ids (`rotate::segment_root`) -- so a witness cites one value per archive.
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
