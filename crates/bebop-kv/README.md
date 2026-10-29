<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# bebop-kv

The key-value layout inside a bebop image. It is sorted byte keys to byte values, with an FNV-1a
fold over the entries (`snapshot_root`) that every reader of the format agrees on. `no_std` + `alloc`,
and it depends only on `bebop-image`.

## Versions (the root says which)

- **v1**: one byte per cell. These are the images written before 2026-09-28. Still read.
- **v2**: the bytes packed eight to a cell. Every fresh image and every compaction is v2.
- **v3**: a v2 base plus a chain of delta records. A commit appends one delta and does not rewrite
  the world. Compaction folds the chain back into a v2 base.
- Any other version is **refused** (`KvError::UnknownVersion`), never guessed at.

The fold is over bytes, never over cells, so a v1 and a v2 image of the same entries give the same
root.

## What is in it

- `Kv`: the owned map, plus `compacted_bytes` / `compacted_from` to write a v2 image.
- `zc::KvIn`: the zero-copy reader over any `Image`. `open` crc-checks every object the root
  reaches. `open_lazy` trusts what recovery already proved.
- `Batch`, `Staged`, `MergeCursor`, `Bytes`, `Limits`: the pure parts of a transaction. These are
  the uncommitted writes, the merged view of image plus batch, a key-ordered cursor, and a key or
  value that is borrowed when the image allows it.
- `delta::append_delta_ops`: byte-key delta commits.

Transactions and devices live in `bebop-io`.

## Licence

MIT OR Apache-2.0.
