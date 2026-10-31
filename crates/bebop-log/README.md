<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-log

An append-only, content-addressed event log inside a bebop image. Each record is its own object.
It carries an object-relative reference to its predecessor and the content id of the record before
it, so an append writes one object and relinks the root, O(1) per insert. `no_std` + `alloc`, and it
depends only on `bebop-image`.

## Versions

- **v1**: payload bytes one per cell.
- **v2**: payload bytes packed eight to a cell. The actor key is optional per record.
- Any other version is refused.

## What is in it

- `EvLog`: append, read back, and the fold.
- `Record`, `pack_payload` / `unpack_payload`.
- `verify::{ChainMark, ChainScan}`: walk the chain and check each record's crc and its link to the
  previous content id. A broken link is quarantined by policy, never silently skipped.

## The signed file

`doc` is a root object over a log: per-device chains, one seal per commit, the table of keys, time
anchors, and segments for history that left the file. This crate holds no signature or hash code.
`Signer`, `Verifier`, `Ids` and `AnchorCheck` are traits a caller implements. The layouts are in
FORMAT §9.4, and [`crates/bebop-store/README.md`](../bebop-store/README.md) says what they promise
and what they do not.

## Licence

MIT OR Apache-2.0.
